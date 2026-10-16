use std::f64::consts::TAU;
use std::time::Duration;

pub(crate) const SETTLE: Duration = Duration::from_millis(300);
const DAMPING: f64 = 0.9;
const RESPONSE: f64 = 0.28;
const FADE_FROM: f64 = 0.35;
const FADE_OVER: f64 = 0.5;
const FADE_OUT: f64 = 2.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Size {
    pub(crate) width: f64,
    pub(crate) height: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Motion {
    pub(crate) from: Size,
    pub(crate) to: Size,
    pub(crate) opening: bool,
    pub(crate) fresh: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Frame {
    pub(crate) size: Size,
    pub(crate) alpha: f64,
    pub(crate) done: bool,
}

pub(crate) fn spring(t: f64) -> f64 {
    if t <= 0.0 {
        return 0.0;
    }
    if t >= SETTLE.as_secs_f64() {
        return 1.0;
    }
    let natural = TAU / RESPONSE;
    let decay = DAMPING * natural;
    let damped = natural * (1.0 - DAMPING * DAMPING).sqrt();
    1.0 - (-decay * t).exp() * ((damped * t).cos() + decay / damped * (damped * t).sin())
}

impl Motion {
    pub(crate) fn at(&self, elapsed: Duration) -> Frame {
        let t = elapsed.as_secs_f64();
        let p = spring(t);
        let mix = |a: f64, b: f64| a + (b - a) * p;
        let size = Size { width: mix(self.from.width, self.to.width), height: mix(self.from.height, self.to.height) };
        let alpha = if self.opening && !self.fresh {
            1.0
        } else if self.opening {
            ((t / SETTLE.as_secs_f64() - FADE_FROM) / FADE_OVER).clamp(0.0, 1.0)
        } else {
            (1.0 - p * FADE_OUT).clamp(0.0, 1.0)
        };
        Frame { size, alpha, done: elapsed >= SETTLE }
    }
}

#[cfg(test)]
mod tests;
