//! Smeared reverb: heavy allpass diffusion feeding a 4-line feedback delay
//! network (FDN). The diffusers blur transients into a wash before they hit
//! the tank, so you get "mall atrium" rather than distinct echoes.

use super::delay_line::DelayLine;
use super::util::{one_pole_coeff, OnePole};

const DIFFUSER_MS_L: [f32; 4] = [3.1, 5.3, 8.9, 13.7];
const DIFFUSER_MS_R: [f32; 4] = [3.7, 6.1, 9.7, 14.9];
/// Mutually prime-ish lengths so the echoes don't line up and ring.
const FDN_MS: [f32; 4] = [41.3, 47.9, 56.3, 67.1];
/// High-frequency damping in the tank. Vaporwave verbs are dark.
const DAMP_HZ: f32 = 5_000.0;

/// Schroeder allpass: flat frequency response, but smears phase/time.
struct Allpass {
    line: DelayLine,
    len: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        let len = len.max(1);
        Self { line: DelayLine::new(len), len }
    }

    #[inline]
    fn process(&mut self, x: f32, g: f32) -> f32 {
        let delayed = self.line.tap(self.len);
        let w = x + g * delayed;
        self.line.push(w);
        delayed - g * w
    }
}

pub struct Reverb {
    sample_rate: f32,
    diff_l: [Allpass; 4],
    diff_r: [Allpass; 4],
    lines: [DelayLine; 4],
    lens: [usize; 4],
    damp: [OnePole; 4],
    damp_coeff: f32,
    cached_decay: f32,
    gains: [f32; 4],
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        let samples = |ms: f32| (ms * sample_rate / 1000.0) as usize;
        let lens = FDN_MS.map(samples);
        Self {
            sample_rate,
            diff_l: DIFFUSER_MS_L.map(|ms| Allpass::new(samples(ms))),
            diff_r: DIFFUSER_MS_R.map(|ms| Allpass::new(samples(ms))),
            lines: lens.map(DelayLine::new),
            lens,
            damp: Default::default(),
            damp_coeff: one_pole_coeff(DAMP_HZ, sample_rate),
            cached_decay: f32::NAN,
            gains: [0.0; 4],
        }
    }

    pub fn reset(&mut self) {
        for ap in self.diff_l.iter_mut().chain(self.diff_r.iter_mut()) {
            ap.line.clear();
        }
        self.lines.iter_mut().for_each(DelayLine::clear);
        self.damp.iter_mut().for_each(OnePole::reset);
    }

    #[inline]
    pub fn process(&mut self, l: f32, r: f32, smear: f32, decay_s: f32, mix: f32) -> (f32, f32) {
        if decay_s != self.cached_decay {
            // Per-line feedback so every line decays 60 dB in `decay_s` seconds.
            for (g, &len) in self.gains.iter_mut().zip(&self.lens) {
                *g = 10f32.powf(-3.0 * len as f32 / (decay_s * self.sample_rate));
            }
            self.cached_decay = decay_s;
        }

        let g = 0.35 + 0.4 * smear;
        let dl = self.diff_l.iter_mut().fold(l, |x, ap| ap.process(x, g));
        let dr = self.diff_r.iter_mut().fold(r, |x, ap| ap.process(x, g));

        let mut outs = [0.0; 4];
        for (i, out) in outs.iter_mut().enumerate() {
            *out = self.damp[i].lowpass(self.lines[i].tap(self.lens[i]), self.damp_coeff);
        }

        // Householder matrix (I - 2/N * ones): mixes every line into every
        // other line while preserving energy, so it can't blow up on its own.
        let half_sum = 0.5 * outs.iter().sum::<f32>();
        let inputs = [dl, dr, dl, dr];
        for i in 0..4 {
            let fb = (outs[i] - half_sum) * self.gains[i];
            self.lines[i].push(fb + 0.5 * inputs[i]);
        }

        let wet_l = 0.5 * (outs[0] + outs[2]);
        let wet_r = 0.5 * (outs[1] + outs[3]);
        (l + mix * (wet_l - l), r + mix * (wet_r - r))
    }
}
