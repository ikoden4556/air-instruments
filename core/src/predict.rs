//! Pluck prediction: fire a Pluck when the Plucker's path will cross a String
//! within the lookahead, instead of waiting to see the crossing (ADR 0004).

use crate::crossing::{crossing_point, Crossing, Pluck};
use crate::frame::Landmark;
use crate::strings::Segment;

/// The lookahead the phase 2 spike measured best at 60 fps.
pub const DEFAULT_LOOKAHEAD_NS: u64 = 60_000_000;

/// Frames of each Plucker's path the velocity is fitted over; fewer than
/// `MIN_HISTORY` and the Plucker is not predicted.
const HISTORY: usize = 4;
const MIN_HISTORY: usize = 3;

/// Wraps `Crossing`: Plucks early on fast approaches, and on the real crossing otherwise.
#[derive(Debug, Clone)]
pub struct Prediction {
    crossing: Crossing,
    aspect: f32,
    lookahead_ns: u64,
    /// Each Plucker's last positions, oldest first; emptied when it goes away.
    history: [Vec<(u64, Landmark)>; 2],
    /// Strings Plucked by prediction and not yet really crossed, with when they fired.
    pending: Vec<(Segment, u64)>,
}

impl Prediction {
    /// `aspect` is screen height / width, as for `Crossing::new`.
    pub fn new(aspect: f32, lookahead_ns: u64) -> Self {
        Self {
            crossing: Crossing::new(aspect),
            aspect,
            lookahead_ns,
            history: Default::default(),
            pending: Vec::new(),
        }
    }

    /// Same as `Crossing::update`, but a Pluck may come before its crossing:
    /// its timestamp is then when the Plucker is predicted to cross.
    pub fn update(
        &mut self,
        timestamp_ns: u64,
        pluckers: [Option<Landmark>; 2],
        strings: &[Segment],
    ) -> Vec<Pluck> {
        let mut plucks = self.crossing.update(timestamp_ns, pluckers, strings);
        for (history, plucker) in self.history.iter_mut().zip(pluckers) {
            let Some(at) = plucker else {
                history.clear();
                continue;
            };
            if history.len() == HISTORY {
                history.remove(0);
            }
            history.push((timestamp_ns, at));
            let Some((vx, vy)) = fitted_velocity(history) else { continue };
            let ahead = self.lookahead_ns as f32 / 1e9;
            let to = Landmark { x: at.x + vx * ahead, y: at.y + vy * ahead };
            let velocity = vx.hypot(vy * self.aspect);
            for (string, s) in strings.iter().enumerate() {
                if self.pending.iter().any(|(p, _)| p == s) {
                    continue;
                }
                let Some(u) = crossing_point(at, to, s) else { continue };
                self.pending.push((*s, timestamp_ns));
                let crosses_ns = timestamp_ns + (self.lookahead_ns as f64 * u as f64).round() as u64;
                plucks.push(Pluck { string, timestamp_ns: crosses_ns, velocity });
            }
        }
        plucks
    }
}

/// Least-squares velocity (screen units per second) through a Plucker's last
/// positions: steadier than the difference of the last two frames.
fn fitted_velocity(history: &[(u64, Landmark)]) -> Option<(f32, f32)> {
    if history.len() < MIN_HISTORY {
        return None;
    }
    let n = history.len() as f64;
    let t0 = history[0].0;
    let ts = history.iter().map(|&(t, _)| (t - t0) as f64 / 1e9);
    let mean_t = ts.clone().sum::<f64>() / n;
    let mean = |f: fn(&Landmark) -> f32| history.iter().map(|(_, p)| f(p) as f64).sum::<f64>() / n;
    let (mean_x, mean_y) = (mean(|p| p.x), mean(|p| p.y));
    let (mut sxx, mut sx, mut sy) = (0.0, 0.0, 0.0);
    for (t, (_, p)) in ts.zip(history) {
        let dt = t - mean_t;
        sxx += dt * dt;
        sx += dt * (p.x as f64 - mean_x);
        sy += dt * (p.y as f64 - mean_y);
    }
    (sxx > 0.0).then(|| ((sx / sxx) as f32, (sy / sxx) as f32))
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;
    /// 60 fps, rounded to whole milliseconds.
    const FRAME: u64 = 16 * MS;

    fn p(x: f32, y: f32) -> Option<Landmark> {
        Some(Landmark { x, y })
    }

    fn across(y: f32) -> Segment {
        Segment { left: Landmark { x: 0.2, y }, right: Landmark { x: 0.8, y } }
    }

    /// Feeds one Plucker moving straight down from `y0` at `speed` screen widths
    /// per second, one frame at a time, and returns each frame's Plucks.
    fn approach(pr: &mut Prediction, strings: &[Segment], y0: f32, speed: f32, frames: u64) -> Vec<Vec<Pluck>> {
        (0..frames)
            .map(|i| {
                let t = i * FRAME;
                pr.update(t, [p(0.5, y0 + speed * t as f32 / 1e9), None], strings)
            })
            .collect()
    }

    #[test]
    fn a_fast_approach_plucks_before_crossing() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        // 4 widths/s from 0.2: reaches 0.5 at 75 ms, between frames 4 (64 ms) and 5 (80 ms).
        let frames = approach(&mut pr, &strings, 0.2, 4.0, 5);
        let plucks: Vec<_> = frames.concat();
        assert_eq!(plucks.len(), 1, "{frames:?}");
        assert_eq!(plucks[0].string, 0);
        assert!((plucks[0].timestamp_ns as f64 / 1e6 - 75.0).abs() < 1.0, "{plucks:?}");
        assert!((plucks[0].velocity - 4.0).abs() < 1e-3, "{plucks:?}");
    }
}
