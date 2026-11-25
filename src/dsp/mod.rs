//! Signal chain:
//!
//!   in -> pitch (pre) -> wow/flutter -> saturation -> reverb -> pitch (post) -> transport -> out
//!
//! Pitch Position splits the pitch shift between the two pitch stages:
//! 0 = all before the tape, 1 = all after (so the reverb tail is pitched too).
//!
//! Each stage is plain Rust with no nih-plug types, so you can unit test it
//! or rip it out into another project.

mod delay_line;
mod pitch;
mod reverb;
mod saturation;
mod transport;
mod util;
mod wow_flutter;

use pitch::PitchShifter;
use reverb::Reverb;
use saturation::Saturation;
use transport::Transport;
use wow_flutter::WowFlutter;

/// Parameter values for one sample frame (already smoothed).
pub struct FrameParams {
    pub wow: f32,
    pub flutter: f32,
    pub drive_gain: f32,
    pub age: f32,
    pub semitones: f32,
    /// 0 = pitch before the tape, 1 = after the reverb; in between splits it.
    pub pitch_pos: f32,
    pub smear: f32,
    pub decay_s: f32,
    pub reverb_mix: f32,
    pub tape_stop: bool,
    pub stop_time_s: f32,
    /// Read head speed from the Speed parameter (the orb). 1 = normal.
    pub speed: f32,
    /// Catch-up mode: true = crossfade back to live, false = rush.
    pub snap_back: bool,
}

pub struct Chain {
    pitch_pre: PitchShifter,
    wow: WowFlutter,
    sat: Saturation,
    reverb: Reverb,
    pitch_post: PitchShifter,
    transport: Transport,
}

impl Chain {
    /// Allocates all buffers. Never call this from `process`.
    pub fn new(sample_rate: f32) -> Self {
        Self {
            pitch_pre: PitchShifter::new(sample_rate),
            pitch_post: PitchShifter::new(sample_rate),
            wow: WowFlutter::new(sample_rate),
            sat: Saturation::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            transport: Transport::new(sample_rate),
        }
    }

    /// Clears state without allocating (may run on the audio thread).
    pub fn reset(&mut self) {
        self.pitch_pre.reset();
        self.pitch_post.reset();
        self.wow.reset();
        self.sat.reset();
        self.reverb.reset();
        self.transport.reset();
    }

    pub fn latency_samples(&self) -> u32 {
        self.wow.latency_samples()
    }

    /// Speed the read head is actually moving at (for the GUI).
    pub fn transport_speed(&self) -> f32 {
        self.transport.effective_speed()
    }

    /// How far behind live the read head is, in seconds (for the GUI).
    pub fn transport_lag_s(&self) -> f32 {
        self.transport.lag_seconds()
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, p: &FrameParams) -> (f32, f32) {
        let post = p.semitones * p.pitch_pos;
        let (l, r) = self.pitch_pre.process(l, r, p.semitones - post);
        let (l, r) = self.wow.process(l, r, p.wow, p.flutter);
        let (l, r) = self.sat.process(l, r, p.drive_gain, p.age);
        let (l, r) = self.reverb.process(l, r, p.smear, p.decay_s, p.reverb_mix);
        let (l, r) = self.pitch_post.process(l, r, post);
        self.transport.process(l, r, p.speed, p.tape_stop, p.stop_time_s, p.snap_back)
    }
}

#[cfg(test)]
mod tests {
    use super::util::Rng;
    use super::*;

    fn extreme(tape_stop: bool) -> FrameParams {
        FrameParams {
            wow: 1.0,
            flutter: 1.0,
            drive_gain: 16.0,
            age: 1.0,
            semitones: -12.0,
            pitch_pos: 0.5,
            smear: 1.0,
            decay_s: 15.0,
            reverb_mix: 1.0,
            tape_stop,
            stop_time_s: 0.1,
            speed: 1.0,
            snap_back: false,
        }
    }

    #[test]
    fn stays_finite_and_bounded_under_extreme_settings() {
        let sr = 48_000.0;
        let mut chain = Chain::new(sr);
        let mut rng = Rng::new(1);
        let mut peak = 0.0f32;
        // 10 s of full-scale noise, toggling tape stop every second.
        for i in 0..(sr as usize * 10) {
            let mut p = extreme((i / sr as usize) % 2 == 1);
            // Sweep the orb speed through reverse and fast-forward too.
            p.speed = 2.0 * (i as f32 / sr * 0.7).sin();
            p.pitch_pos = 0.5 + 0.5 * (i as f32 / sr * 0.3).sin();
            let (l, r) = chain.process(rng.bipolar(), rng.bipolar(), &p);
            assert!(l.is_finite() && r.is_finite(), "NaN/inf at sample {i}");
            peak = peak.max(l.abs()).max(r.abs());
        }
        assert!(peak < 4.0, "peak {peak} too hot");
    }

    #[test]
    fn reverb_tail_decays_to_silence() {
        let sr = 48_000.0;
        let mut chain = Chain::new(sr);
        let mut p = extreme(false);
        p.decay_s = 1.0;
        chain.process(1.0, 1.0, &p);
        let mut last = 0.0f32;
        for i in 0..(sr as usize * 4) {
            let (l, r) = chain.process(0.0, 0.0, &p);
            if i > sr as usize * 3 {
                last = last.max(l.abs()).max(r.abs());
            }
        }
        assert!(last < 1e-3, "tail still at {last} after 3 s");
    }

    /// Wherever the pitch shift happens, a 440 Hz tone at -12 st should come
    /// out at ~220 Hz. Counts zero crossings with the other effects neutral.
    #[test]
    fn pitch_position_keeps_total_pitch() {
        let sr = 48_000.0;
        for pos in [0.0, 0.5, 1.0] {
            let mut chain = Chain::new(sr);
            let p = FrameParams {
                wow: 0.0,
                flutter: 0.0,
                drive_gain: 1.0,
                age: 0.0,
                semitones: -12.0,
                pitch_pos: pos,
                smear: 0.0,
                decay_s: 0.3,
                reverb_mix: 0.0,
                tape_stop: false,
                stop_time_s: 1.0,
                speed: 1.0,
                snap_back: false,
            };
            let mut crossings = 0;
            let mut last = 0.0f32;
            for n in 0..(sr as usize * 2) {
                let x = 0.5 * (std::f32::consts::TAU * 440.0 * n as f32 / sr).sin();
                let (y, _) = chain.process(x, x, &p);
                // Skip the first second while the shifters fade in.
                if n > sr as usize && (last <= 0.0) != (y <= 0.0) {
                    crossings += 1;
                }
                last = y;
            }
            let hz = crossings as f32 / 2.0;
            assert!((hz - 220.0).abs() < 220.0 * 0.06, "pos {pos}: got {hz} Hz");
        }
    }
}
