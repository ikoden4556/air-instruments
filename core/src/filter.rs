//! One Euro jitter filter for hand landmarks, driven by capture timestamps.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OneEuroSettings {
    /// Cutoff (Hz) when the point is still: lower means less jitter, more lag.
    pub min_cutoff: f32,
    /// How fast the cutoff rises with speed: higher means less lag on fast moves.
    pub beta: f32,
    /// Cutoff (Hz) for the speed estimate itself.
    pub derivative_cutoff: f32,
}

/// Filters one coordinate.
#[derive(Debug, Clone)]
pub struct OneEuroFilter {
    settings: OneEuroSettings,
    /// Last sample time, filtered value and filtered speed (units per second).
    last: Option<(u64, f32, f32)>,
}

impl OneEuroFilter {
    pub fn new(settings: OneEuroSettings) -> Self {
        Self { settings, last: None }
    }

    /// Feeds one sample taken at `timestamp_ns` and returns the filtered value.
    /// The smoothing depends on the real time between samples, not on the frame count.
    pub fn filter(&mut self, timestamp_ns: u64, value: f32) -> f32 {
        let Some((last_ns, last_value, last_speed)) = self.last else {
            self.last = Some((timestamp_ns, value, 0.0));
            return value;
        };
        if timestamp_ns <= last_ns {
            return last_value;
        }
        let dt = (timestamp_ns - last_ns) as f32 * 1e-9;
        let s = self.settings;
        let raw_speed = (value - last_value) / dt;
        let speed = lerp(last_speed, raw_speed, alpha(dt, s.derivative_cutoff));
        let cutoff = s.min_cutoff + s.beta * speed.abs();
        let filtered = lerp(last_value, value, alpha(dt, cutoff));
        self.last = Some((timestamp_ns, filtered, speed));
        filtered
    }
}

/// Smoothing factor of a first-order low-pass at `cutoff_hz` over `dt` seconds.
fn alpha(dt: f32, cutoff_hz: f32) -> f32 {
    let tau = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz);
    1.0 / (1.0 + tau / dt)
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    fn settings() -> OneEuroSettings {
        OneEuroSettings { min_cutoff: 1.0, beta: 10.0, derivative_cutoff: 1.0 }
    }

    /// Repeatable noise in -1..1.
    fn noise(i: u64) -> f32 {
        let h = i.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((h >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    }

    #[test]
    fn a_still_point_stays_still_under_noise() {
        let mut f = OneEuroFilter::new(settings());
        let mut outputs = Vec::new();
        for i in 0..120 {
            outputs.push(f.filter(i * 16 * MS, 0.5 + 0.005 * noise(i)));
        }
        let settled = &outputs[60..];
        let spread = settled.iter().cloned().fold(f32::MIN, f32::max)
            - settled.iter().cloned().fold(f32::MAX, f32::min);
        // Input spread is up to 0.01; the filtered point should barely move.
        assert!(spread < 0.002, "spread {spread}");
    }
}
