/// A ring buffer you push samples into and read back from the past.
///
/// Almost everything in this plugin is a delay line in disguise:
/// wow/flutter (modulated delay), pitch shifting (two sweeping delays),
/// tape stop (a read head slowing down) and reverb (feedback delays).
pub struct DelayLine {
    buf: Vec<f32>,
    /// Length is a power of two, so `& mask` replaces a slow `%`.
    mask: usize,
    /// Index of the slot the *next* push will write to.
    write: usize,
}

impl DelayLine {
    pub fn new(max_delay_samples: usize) -> Self {
        let len = (max_delay_samples + 4).next_power_of_two();
        Self { buf: vec![0.0; len], mask: len - 1, write: 0 }
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
        self.write = 0;
    }

    #[inline]
    pub fn push(&mut self, x: f32) {
        self.buf[self.write] = x;
        self.write = (self.write + 1) & self.mask;
    }

    /// The sample pushed `k` pushes ago (`k = 1` is the most recent).
    /// Use *before* pushing for fixed feedback delays of `k` samples.
    #[inline]
    pub fn tap(&self, k: usize) -> f32 {
        self.buf[self.write.wrapping_sub(k) & self.mask]
    }

    /// Fractional delay read, used *after* pushing; 0 = the sample just pushed.
    /// Interpolates with a 4-point Hermite spline so modulated reads stay smooth.
    #[inline]
    pub fn read(&self, delay: f32) -> f32 {
        let d = delay.clamp(1.0, (self.buf.len() - 3) as f32);
        let di = d as usize;
        let t = d - di as f32;
        let at = |k: usize| self.tap(k + 1);
        hermite(t, at(di - 1), at(di), at(di + 1), at(di + 2))
    }
}

/// 4-point, 3rd-order Hermite interpolation between `x0` and `x1`.
#[inline]
fn hermite(t: f32, xm1: f32, x0: f32, x1: f32, x2: f32) -> f32 {
    let c1 = 0.5 * (x1 - xm1);
    let c2 = xm1 - 2.5 * x0 + 2.0 * x1 - 0.5 * x2;
    let c3 = 0.5 * (x2 - xm1) + 1.5 * (x0 - x1);
    ((c3 * t + c2) * t + c1) * t + x0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_reads_are_exact() {
        let mut d = DelayLine::new(64);
        d.push(1.0);
        for _ in 0..10 {
            d.push(0.0);
        }
        assert_eq!(d.read(10.0), 1.0);
        assert_eq!(d.tap(11), 1.0);
        assert_eq!(d.read(9.0), 0.0);
    }

    #[test]
    fn fractional_read_interpolates_ramp() {
        let mut d = DelayLine::new(64);
        for i in 0..20 {
            d.push(i as f32);
        }
        // Newest is 19, so delay 2.5 should land halfway between 17 and 16.
        assert!((d.read(2.5) - 16.5).abs() < 1e-5);
    }
}
