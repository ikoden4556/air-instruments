//! One Euro jitter filter for hand landmarks, driven by capture timestamps.

use crate::frame::{Frame, Handedness, LANDMARKS_PER_HAND};

/// x and y filters for every landmark of one Hand.
type HandFilters = [[OneEuroFilter; 2]; LANDMARKS_PER_HAND];

/// Filters every landmark coordinate of each Hand, keeping one set of filters per handedness.
#[derive(Debug, Clone)]
pub struct LandmarkFilter {
    left: HandFilters,
    right: HandFilters,
}

impl LandmarkFilter {
    pub fn new(settings: OneEuroSettings) -> Self {
        let hand = || std::array::from_fn(|_| std::array::from_fn(|_| OneEuroFilter::new(settings)));
        Self { left: hand(), right: hand() }
    }

    /// Returns the frame with every landmark smoothed.
    pub fn filter(&mut self, frame: &Frame) -> Frame {
        let mut out = frame.clone();
        for hand in &mut out.hands {
            let filters = match hand.handedness {
                Handedness::Left => &mut self.left,
                Handedness::Right => &mut self.right,
            };
            for (l, [fx, fy]) in hand.landmarks.iter_mut().zip(filters.iter_mut()) {
                l.x = fx.filter(frame.timestamp_ns, l.x);
                l.y = fy.filter(frame.timestamp_ns, l.y);
            }
        }
        out
    }
}

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
    use crate::frame::{Hand, Handedness, LANDMARKS_PER_HAND, Landmark};

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

    #[test]
    fn a_fast_move_is_followed_with_little_lag() {
        // A fingertip sweeping across the screen at 2 screen widths per second.
        let speed = 2.0;
        let mut f = OneEuroFilter::new(settings());
        let mut lag = 0.0;
        for i in 0..30 {
            let t = i * 16 * MS;
            let target = speed * (t as f32 * 1e-9);
            lag = target - f.filter(t, target);
        }
        // Without speed adaptation (beta 0) the lag would be about 0.3.
        assert!(lag < 0.03, "lag {lag}");
    }

    /// Filters a step from 0 to 1 at the given frame gaps; returns the output at 99 ms.
    fn step_response_at_99_ms(gaps_ms: &[u64]) -> f32 {
        let still = OneEuroSettings { beta: 0.0, ..settings() };
        let mut f = OneEuroFilter::new(still);
        f.filter(0, 0.0);
        let mut t = 0;
        let mut out = 0.0;
        for gap in gaps_ms.iter().cycle() {
            t += gap;
            if t > 99 {
                break;
            }
            out = f.filter(t * MS, 1.0);
        }
        out
    }

    #[test]
    fn uneven_frame_gaps_smooth_by_time_not_frame_count() {
        let even_33 = step_response_at_99_ms(&[33]); // 3 frames
        let mixed = step_response_at_99_ms(&[16, 33]); // 16, 49, 65, 98 ms: 4 frames
        let even_16 = step_response_at_99_ms(&[16]); // 6 frames
        // A per-frame filter would give about 0.17 per frame: 0.43 vs 0.54 vs 0.68.
        assert!((even_33 - even_16).abs() < 0.03, "{even_33} vs {even_16}");
        assert!((mixed - even_16).abs() < 0.03, "{mixed} vs {even_16}");
    }

    fn hand(handedness: Handedness, at: f32) -> Hand {
        Hand { handedness, landmarks: [Landmark { x: at, y: at }; LANDMARKS_PER_HAND] }
    }

    #[test]
    fn each_hand_keeps_its_own_filters_for_every_landmark() {
        let mut f = LandmarkFilter::new(settings());
        f.filter(&Frame {
            timestamp_ns: 0,
            hands: vec![hand(Handedness::Left, 0.2), hand(Handedness::Right, 0.8)],
        });
        // Next frame: both hands move by 0.01 and arrive in the other order.
        let out = f.filter(&Frame {
            timestamp_ns: 16 * MS,
            hands: vec![hand(Handedness::Right, 0.81), hand(Handedness::Left, 0.21)],
        });

        assert_eq!(out.timestamp_ns, 16 * MS);
        let handedness: Vec<_> = out.hands.iter().map(|h| h.handedness).collect();
        assert_eq!(handedness, [Handedness::Right, Handedness::Left]);
        for (h, from) in out.hands.iter().zip([0.8, 0.2]) {
            for l in &h.landmarks {
                // Smoothed: part of the way from the old position to the new one.
                for c in [l.x, l.y] {
                    assert!(c > from && c < from + 0.01, "{:?} at {c}", h.handedness);
                }
            }
        }
    }
}
