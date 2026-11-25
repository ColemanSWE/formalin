//! Transport: a read head that can slow, stop, reverse and catch back up.
//!
//! Audio is always written at normal speed. The read head moves at
//! `speed` samples per sample, so anything other than 1.0 makes it drift
//! away from "live" -- that distance is the `lag`. You can't read the
//! future, so lag never goes below zero.
//!
//! Two things drive the speed:
//! - the `Speed` parameter (the orb in the GUI writes this; -2..2)
//! - the `Tape Stop` switch, which coasts the head to a halt over `Stop Time`
//!
//! Once you're back at normal speed with lag left over, the head either
//! *rushes* (plays faster until it's caught up, a pitch-up swoop) or *snaps*
//! (crossfades straight back to live).

use super::delay_line::DelayLine;
use super::util::one_pole_coeff;

/// How far behind live we can fall. Reverse playback eats this fastest.
const MAX_LAG_S: f32 = 8.0;
const XFADE_MS: f32 = 15.0;
const SPIN_UP_S: f32 = 0.25;
/// Rush catch-up: extra speed per second of lag, capped at +1 (an octave up).
const RUSH_PER_S: f32 = 2.0;

pub struct Transport {
    lines: [DelayLine; 2],
    sample_rate: f32,
    max_lag: f32,
    lag: f32,
    /// 1 = running, 0 = stopped by the Tape Stop switch.
    stop_ramp: f32,
    /// Smoothed extra speed used while rushing back to live.
    rush: f32,
    rush_coeff: f32,
    /// Speed the read head actually moved at on the last sample.
    effective_speed: f32,
    xfade: f32,
    xfade_inc: f32,
    old_lag: f32,
}

impl Transport {
    pub fn new(sample_rate: f32) -> Self {
        let max_lag = MAX_LAG_S * sample_rate;
        Self {
            lines: [DelayLine::new(max_lag as usize + 4), DelayLine::new(max_lag as usize + 4)],
            sample_rate,
            max_lag,
            lag: 0.0,
            stop_ramp: 1.0,
            rush: 0.0,
            rush_coeff: one_pole_coeff(6.0, sample_rate),
            effective_speed: 1.0,
            xfade: 0.0,
            xfade_inc: 1.0 / (XFADE_MS * 0.001 * sample_rate),
            old_lag: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.lines.iter_mut().for_each(DelayLine::clear);
        self.lag = 0.0;
        self.stop_ramp = 1.0;
        self.rush = 0.0;
        self.effective_speed = 1.0;
        self.xfade = 0.0;
    }

    pub fn effective_speed(&self) -> f32 {
        self.effective_speed
    }

    pub fn lag_seconds(&self) -> f32 {
        self.lag / self.sample_rate
    }

    #[inline]
    fn read(&self, lag: f32) -> (f32, f32) {
        (self.lines[0].read(lag), self.lines[1].read(lag))
    }

    #[inline]
    pub fn process(
        &mut self,
        l: f32,
        r: f32,
        speed: f32,
        stop_engaged: bool,
        stop_time_s: f32,
        snap_back: bool,
    ) -> (f32, f32) {
        self.lines[0].push(l);
        self.lines[1].push(r);

        // Tape Stop switch: coast down linearly, spin back up quickly.
        self.stop_ramp = if stop_engaged {
            (self.stop_ramp - 1.0 / (stop_time_s * self.sample_rate)).max(0.0)
        } else {
            (self.stop_ramp + 1.0 / (SPIN_UP_S * self.sample_rate)).min(1.0)
        };
        let base = speed * self.stop_ramp;

        // Back at (or above) normal speed but still behind live: catch up.
        let mut rush_target = 0.0;
        if !stop_engaged && base >= 0.98 && self.lag >= 1.0 {
            if snap_back {
                self.old_lag = self.lag;
                self.xfade = 1.0;
                self.lag = 0.0;
            } else {
                rush_target = (self.lag_seconds() * RUSH_PER_S).min(1.0);
            }
        }
        self.rush += self.rush_coeff * (rush_target - self.rush);
        if self.rush < 1e-6 {
            self.rush = 0.0;
        }

        let speed = base + self.rush;
        self.effective_speed = speed;
        self.lag = (self.lag + 1.0 - speed).clamp(0.0, self.max_lag);

        // Fade out near standstill so we don't emit sub-sonic mud / DC.
        let gain = (speed.abs() * 8.0).min(1.0);
        let (mut ol, mut or) = if self.lag < 1.0 {
            (l * gain, r * gain)
        } else {
            let (a, b) = self.read(self.lag);
            (a * gain, b * gain)
        };

        if self.xfade > 0.0 {
            let (a, b) = self.read(self.old_lag);
            let x = self.xfade;
            ol = ol * (1.0 - x) + a * x;
            or = or * (1.0 - x) + b * x;
            self.xfade = (self.xfade - self.xfade_inc).max(0.0);
        }

        (ol, or)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SR: f32 = 48_000.0;

    fn run(t: &mut Transport, seconds: f32, speed: f32, stop: bool, snap: bool) {
        for _ in 0..(seconds * SR) as usize {
            t.process(0.5, 0.5, speed, stop, 0.5, snap);
        }
    }

    #[test]
    fn normal_speed_is_passthrough() {
        let mut t = Transport::new(SR);
        for i in 0..1000 {
            let x = (i as f32 * 0.01).sin();
            assert_eq!(t.process(x, -x, 1.0, false, 1.0, false), (x, -x));
        }
    }

    #[test]
    fn slowing_down_builds_lag_and_rush_catches_up() {
        let mut t = Transport::new(SR);
        run(&mut t, 1.0, 0.5, false, false);
        assert!((t.lag_seconds() - 0.5).abs() < 0.01, "lag {}", t.lag_seconds());
        run(&mut t, 3.0, 1.0, false, false);
        assert!(t.lag_seconds() < 0.01, "still {} s behind", t.lag_seconds());
    }

    #[test]
    fn reverse_moves_away_from_live() {
        let mut t = Transport::new(SR);
        run(&mut t, 0.5, -1.0, false, false);
        assert!((t.lag_seconds() - 1.0).abs() < 0.01);
    }

    #[test]
    fn snap_returns_to_live_immediately() {
        let mut t = Transport::new(SR);
        run(&mut t, 1.0, 0.2, false, true);
        run(&mut t, 0.01, 1.0, false, true);
        assert_eq!(t.lag_seconds(), 0.0);
    }

    #[test]
    fn tape_stop_halts_then_recovers() {
        let mut t = Transport::new(SR);
        run(&mut t, 1.0, 1.0, true, false);
        assert_eq!(t.effective_speed(), 0.0);
        run(&mut t, 4.0, 1.0, false, false);
        assert!(t.lag_seconds() < 0.01);
        assert!((t.effective_speed() - 1.0).abs() < 0.01);
    }
}
