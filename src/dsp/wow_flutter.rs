//! Wow & flutter: speed wobble of the tape transport.
//!
//! Trick: if you read from a delay line whose delay time is changing, the
//! pitch changes (Doppler). Slow wobble = wow, fast jitter = flutter.

use super::delay_line::DelayLine;
use super::util::Drift;
use std::f32::consts::TAU;

/// Center delay. Must exceed the max modulation depth so delay never goes < 0.
const BASE_DELAY_MS: f32 = 12.0;
/// Peak delay swing at 100%. 6 ms at ~0.5 Hz is roughly +/-30 cents of warble.
const WOW_DEPTH_MS: f32 = 6.0;
const FLUTTER_DEPTH_MS: f32 = 0.25;
const WOW_HZ: f32 = 0.55;
const FLUTTER_HZ: f32 = 7.3;

pub struct WowFlutter {
    lines: [DelayLine; 2],
    samples_per_ms: f32,
    wow_inc: f32,
    flutter_inc: f32,
    flutter2_inc: f32,
    wow_phase: f32,
    flutter_phase: f32,
    flutter2_phase: f32,
    /// Random wander so the wow doesn't sound like a perfect sine LFO.
    drift: Drift,
}

impl WowFlutter {
    pub fn new(sample_rate: f32) -> Self {
        let samples_per_ms = sample_rate / 1000.0;
        let max = (BASE_DELAY_MS * 2.0 * samples_per_ms) as usize;
        Self {
            lines: [DelayLine::new(max), DelayLine::new(max)],
            samples_per_ms,
            wow_inc: WOW_HZ / sample_rate,
            flutter_inc: FLUTTER_HZ / sample_rate,
            flutter2_inc: FLUTTER_HZ * 1.618 / sample_rate,
            wow_phase: 0.0,
            flutter_phase: 0.0,
            flutter2_phase: 0.0,
            drift: Drift::new(sample_rate, 0.8, 0x5EED),
        }
    }

    pub fn reset(&mut self) {
        self.lines.iter_mut().for_each(DelayLine::clear);
        self.wow_phase = 0.0;
        self.flutter_phase = 0.0;
        self.flutter2_phase = 0.0;
        self.drift.reset();
    }

    pub fn latency_samples(&self) -> u32 {
        (BASE_DELAY_MS * self.samples_per_ms).round() as u32
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, wow: f32, flutter: f32) -> (f32, f32) {
        self.wow_phase = (self.wow_phase + self.wow_inc).fract();
        self.flutter_phase = (self.flutter_phase + self.flutter_inc).fract();
        self.flutter2_phase = (self.flutter2_phase + self.flutter2_inc).fract();

        let wow_mod = 0.6 * (TAU * self.wow_phase).sin() + 0.4 * self.drift.next();
        let flutter_mod =
            0.7 * (TAU * self.flutter_phase).sin() + 0.3 * (TAU * self.flutter2_phase).sin();

        // Squared amounts give finer control at the subtle end of the knob.
        let delay_ms = BASE_DELAY_MS
            + wow * wow * WOW_DEPTH_MS * wow_mod
            + flutter * flutter * FLUTTER_DEPTH_MS * flutter_mod;
        let d = delay_ms * self.samples_per_ms;

        // Same modulation on both channels: one tape, one transport.
        self.lines[0].push(l);
        self.lines[1].push(r);
        (self.lines[0].read(d), self.lines[1].read(d))
    }
}
