//! Values the audio thread publishes for the GUI to draw. Lock-free:
//! the audio thread stores, the GUI loads, nobody waits.

use nih_plug::prelude::AtomicF32;
use std::sync::atomic::Ordering;

pub struct Meters {
    /// Output peak of the last block, linear.
    pub level: AtomicF32,
    /// Speed the transport's read head actually moved at.
    pub speed: AtomicF32,
    /// Seconds behind live.
    pub lag_s: AtomicF32,
}

impl Default for Meters {
    fn default() -> Self {
        Self { level: AtomicF32::new(0.0), speed: AtomicF32::new(1.0), lag_s: AtomicF32::new(0.0) }
    }
}

impl Meters {
    pub fn store(&self, level: f32, speed: f32, lag_s: f32) {
        self.level.store(level, Ordering::Relaxed);
        self.speed.store(speed, Ordering::Relaxed);
        self.lag_s.store(lag_s, Ordering::Relaxed);
    }

    pub fn load(&self) -> (f32, f32, f32) {
        (
            self.level.load(Ordering::Relaxed),
            self.speed.load(Ordering::Relaxed),
            self.lag_s.load(Ordering::Relaxed),
        )
    }
}
