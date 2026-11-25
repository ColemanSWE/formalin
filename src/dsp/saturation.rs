//! Tape saturation: soft clipping plus the high-end loss of worn tape.

use super::util::{one_pole_coeff, DcBlocker, OnePole};

/// Offsetting the curve makes it asymmetric, which adds even harmonics --
/// the "warm" part of tape (and tubes). Zero bias = only odd harmonics.
const BIAS: f32 = 0.15;
/// Lowpass cutoff at Age = 0% and Age = 100%.
const FRESH_HZ: f32 = 20_000.0;
const WORN_HZ: f32 = 2_500.0;

pub struct Saturation {
    sample_rate: f32,
    dc: [DcBlocker; 2],
    tone: [OnePole; 2],
}

impl Saturation {
    pub fn new(sample_rate: f32) -> Self {
        Self {
            sample_rate,
            dc: [DcBlocker::new(sample_rate), DcBlocker::new(sample_rate)],
            tone: Default::default(),
        }
    }

    pub fn reset(&mut self) {
        self.dc.iter_mut().for_each(DcBlocker::reset);
        self.tone.iter_mut().for_each(OnePole::reset);
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, drive_gain: f32, age: f32) -> (f32, f32) {
        // Exponential sweep so the knob feels even across octaves.
        let cutoff = (FRESH_HZ * (WORN_HZ / FRESH_HZ).powf(age)).min(0.45 * self.sample_rate);
        let coeff = one_pole_coeff(cutoff, self.sample_rate);
        // Partial make-up gain: more drive = more squash, without a huge jump.
        let makeup = 1.0 / drive_gain.sqrt();
        let bias_offset = BIAS.tanh();

        let mut ch = |i: usize, x: f32| {
            let y = ((x * drive_gain + BIAS).tanh() - bias_offset) * makeup;
            let y = self.dc[i].process(y);
            self.tone[i].lowpass(y, coeff)
        };
        (ch(0, l), ch(1, r))
    }
}
