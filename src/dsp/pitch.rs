//! Real-time pitch shift (the "slowed" vaporwave sound, without slowing time).
//!
//! You can't actually play live audio slower forever -- you'd fall behind.
//! Instead, two read heads sweep through a short delay window at the shifted
//! speed, each jumping back when it reaches the end. They're offset by half a
//! window and crossfaded so one is always silent at its jump. Lo-fi and a bit
//! warbly, which here is a feature.

use super::delay_line::DelayLine;
use super::util::one_pole_coeff;
use std::f32::consts::PI;

const WINDOW_MS: f32 = 50.0;
const MIN_DELAY: f32 = 2.0;

pub struct PitchShifter {
    lines: [DelayLine; 2],
    window: f32,
    /// Position of read head A within the window, 0..1.
    phase: f32,
    /// Crossfade to dry at 0 st (two taps at fixed delays would comb-filter).
    wet: f32,
    wet_coeff: f32,
}

impl PitchShifter {
    pub fn new(sample_rate: f32) -> Self {
        let window = WINDOW_MS * sample_rate / 1000.0;
        let max = (window + MIN_DELAY) as usize + 8;
        Self {
            lines: [DelayLine::new(max), DelayLine::new(max)],
            window,
            phase: 0.0,
            wet: 0.0,
            wet_coeff: one_pole_coeff(20.0, sample_rate),
        }
    }

    pub fn reset(&mut self) {
        self.lines.iter_mut().for_each(DelayLine::clear);
        self.phase = 0.0;
        self.wet = 0.0;
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, semitones: f32) -> (f32, f32) {
        self.lines[0].push(l);
        self.lines[1].push(r);

        let target = if semitones.abs() > 0.005 { 1.0 } else { 0.0 };
        self.wet += self.wet_coeff * (target - self.wet);
        if target == 0.0 && self.wet < 1e-4 {
            self.wet = 0.0;
            return (l, r);
        }

        // Reading at `ratio` speed means the delay grows by (1 - ratio) per sample.
        let ratio = (semitones / 12.0).exp2();
        self.phase = (self.phase + (1.0 - ratio) / self.window).rem_euclid(1.0);

        let pa = self.phase;
        let pb = (pa + 0.5).fract();
        // sin^2 + cos^2 = 1, so the two gains always sum to unity.
        let ga = (PI * pa).sin().powi(2);
        let gb = 1.0 - ga;
        let da = MIN_DELAY + pa * self.window;
        let db = MIN_DELAY + pb * self.window;

        let shift = |line: &DelayLine| line.read(da) * ga + line.read(db) * gb;
        let sl = shift(&self.lines[0]);
        let sr = shift(&self.lines[1]);
        (l + self.wet * (sl - l), r + self.wet * (sr - r))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_semitones_is_transparent() {
        let mut p = PitchShifter::new(48_000.0);
        for i in 0..1000 {
            let x = (i as f32 * 0.1).sin();
            assert_eq!(p.process(x, -x, 0.0), (x, -x));
        }
    }
}
