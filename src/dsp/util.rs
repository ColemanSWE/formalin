use std::f32::consts::TAU;

/// Tiny xorshift PRNG. Real-time safe, no dependencies, deterministic.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform in [-1, 1].
    #[inline]
    pub fn bipolar(&mut self) -> f32 {
        (self.next_u32() as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Coefficient for a one-pole lowpass with the given cutoff.
#[inline]
pub fn one_pole_coeff(cutoff_hz: f32, sample_rate: f32) -> f32 {
    1.0 - (-TAU * cutoff_hz / sample_rate).exp()
}

#[derive(Default)]
pub struct OnePole {
    z: f32,
}

impl OnePole {
    #[inline]
    pub fn lowpass(&mut self, x: f32, coeff: f32) -> f32 {
        self.z += coeff * (x - self.z);
        self.z
    }

    pub fn reset(&mut self) {
        self.z = 0.0;
    }
}

/// Removes DC offset (asymmetric saturation creates some).
pub struct DcBlocker {
    r: f32,
    x1: f32,
    y1: f32,
}

impl DcBlocker {
    pub fn new(sample_rate: f32) -> Self {
        // ~10 Hz highpass
        Self { r: 1.0 - TAU * 10.0 / sample_rate, x1: 0.0, y1: 0.0 }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }
}

/// Smooth random wander in [-1, 1] -- a new random target every
/// `1 / rate_hz` seconds, glided to through two one-pole filters.
pub struct Drift {
    seed: u32,
    rng: Rng,
    target: f32,
    a: f32,
    b: f32,
    counter: u32,
    period: u32,
    coeff: f32,
}

impl Drift {
    pub fn new(sample_rate: f32, rate_hz: f32, seed: u32) -> Self {
        Self {
            seed,
            rng: Rng::new(seed),
            target: 0.0,
            a: 0.0,
            b: 0.0,
            counter: 0,
            period: ((sample_rate / rate_hz) as u32).max(1),
            coeff: one_pole_coeff(rate_hz, sample_rate),
        }
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if self.counter == 0 {
            self.target = self.rng.bipolar();
            self.counter = self.period;
        }
        self.counter -= 1;
        self.a += self.coeff * (self.target - self.a);
        self.b += self.coeff * (self.a - self.b);
        self.b
    }

    pub fn reset(&mut self) {
        *self = Self { rng: Rng::new(self.seed), target: 0.0, a: 0.0, b: 0.0, counter: 0, ..*self };
    }
}
