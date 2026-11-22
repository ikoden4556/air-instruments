//! Pluck prediction: fire a Pluck when the Plucker's path will cross a String
//! within the lookahead, instead of waiting to see the crossing (ADR 0004).

use crate::crossing::{Crossing, JITTER_NS, Pluck, crossing_point};
use crate::frame::Landmark;
use crate::strings::Segment;

/// The lookahead the phase 2 spike measured best at 60 fps.
pub const DEFAULT_LOOKAHEAD_NS: u64 = 60_000_000;

/// Frames of each Plucker's path the velocity is fitted over; fewer than
/// `MIN_HISTORY` and the Plucker is not predicted.
const HISTORY: usize = 5;
const MIN_HISTORY: usize = 3;

/// Pluckers slower than this, in screen widths per second, are left to the real crossing.
const MIN_SPEED: f32 = 1.0;

/// A predicted Pluck whose String is not crossed this soon after was FALSE.
const FALSE_AFTER_NS: u64 = 250_000_000;

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
    /// When each String last Plucked or was really crossed, for S5.
    recent: Vec<(Segment, u64)>,
    false_plucks: u32,
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
            recent: Vec::new(),
            false_plucks: 0,
        }
    }

    /// Predicted Plucks whose String was never crossed in time, so far.
    pub fn false_plucks(&self) -> u32 {
        self.false_plucks
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
        // Predicted crossings run up to a lookahead ahead of now.
        let window = JITTER_NS + self.lookahead_ns;
        self.recent.retain(|&(_, t)| t + window > timestamp_ns);
        // A real crossing of a String already Plucked by prediction is that Pluck.
        plucks.retain(|pl| {
            let s = strings[pl.string];
            self.recent.push((s, pl.timestamp_ns));
            let predicted = self.pending.iter().position(|(p, _)| *p == s);
            predicted.map(|i| self.pending.remove(i)).is_none()
        });
        let before = self.pending.len();
        self.pending
            .retain(|&(_, fired)| timestamp_ns - fired < FALSE_AFTER_NS);
        self.false_plucks += (before - self.pending.len()) as u32;
        for (i, (history, plucker)) in self.history.iter_mut().zip(pluckers).enumerate() {
            let Some(at) = plucker else {
                history.clear();
                continue;
            };
            if history.len() == HISTORY {
                history.remove(0);
            }
            history.push((timestamp_ns, at));
            let Some((vx, vy)) = fitted_velocity(history) else {
                continue;
            };
            let ahead = self.lookahead_ns as f32 / 1e9;
            let to = Landmark {
                x: at.x + vx * ahead,
                y: at.y + vy * ahead,
            };
            let velocity = vx.hypot(vy * self.aspect);
            if velocity < MIN_SPEED {
                continue;
            }
            // How far it can still go if it keeps braking as now (screen widths).
            let stops_within = stopping_distance(history, self.aspect, (vx, vy));
            for (string, s) in strings.iter().enumerate() {
                if self.pending.iter().any(|(p, _)| p == s) || !self.crossing.armed(i, s) {
                    continue;
                }
                let Some((u, along)) = crossing_point(at, to, s) else {
                    continue;
                };
                // Braking to a stop short of the String: a near miss, not a Pluck.
                let to_string = u * (to.x - at.x).hypot((to.y - at.y) * self.aspect);
                if stops_within.is_some_and(|d| d < to_string) {
                    continue;
                }
                let crosses_ns =
                    timestamp_ns + (self.lookahead_ns as f64 * u as f64).round() as u64;
                let jitter = self
                    .recent
                    .iter()
                    .any(|&(r, t)| r == *s && crosses_ns.abs_diff(t) < JITTER_NS);
                if jitter {
                    continue;
                }
                self.pending.push((*s, timestamp_ns));
                self.recent.push((*s, crosses_ns));
                plucks.push(Pluck {
                    string,
                    timestamp_ns: crosses_ns,
                    velocity,
                    along,
                    predicted: true,
                });
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

/// If the Plucker is braking along `dir` (its fitted velocity), how far (screen widths)
/// it goes before it stops, from a quadratic fit (position, velocity, acceleration)
/// through its last positions.
fn stopping_distance(history: &[(u64, Landmark)], aspect: f32, dir: (f32, f32)) -> Option<f32> {
    if history.len() < MIN_HISTORY {
        return None;
    }
    // Times relative to the newest frame, so the fit's velocity is the current one.
    let now = history[history.len() - 1].0;
    let ts: Vec<f64> = history.iter().map(|&(t, _)| -((now - t) as f64) / 1e9).collect();
    // Least squares for p(t) = c + v t + a t²/2: normal equations, solved by Cramer's rule.
    let basis = |t: f64| [1.0, t, t * t / 2.0];
    let mut m = [[0.0f64; 3]; 3];
    let (mut rx, mut ry) = ([0.0f64; 3], [0.0f64; 3]);
    for (&t, (_, p)) in ts.iter().zip(history) {
        let b = basis(t);
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] += b[i] * b[j];
            }
            rx[i] += b[i] * p.x as f64;
            ry[i] += b[i] * p.y as f64 * aspect as f64;
        }
    }
    let det = |m: &[[f64; 3]; 3]| {
        m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
            - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
            + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
    };
    let d = det(&m);
    if d.abs() < 1e-18 {
        return None;
    }
    let solve = |r: [f64; 3], k: usize| {
        let mut mk = m;
        for i in 0..3 {
            mk[i][k] = r[i];
        }
        det(&mk) / d
    };
    // Current velocity and acceleration along the direction of travel.
    let norm = (dir.0 as f64).hypot(dir.1 as f64 * aspect as f64);
    let along = |x: f64, y: f64| (x * dir.0 as f64 + y * dir.1 as f64 * aspect as f64) / norm;
    let speed = along(solve(rx, 1), solve(ry, 1));
    let accel = along(solve(rx, 2), solve(ry, 2));
    if speed <= 0.0 {
        return Some(0.0); // already stopped, or turning back
    }
    (accel < 0.0).then(|| (speed * speed / (-2.0 * accel)) as f32)
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
        Segment {
            left: Landmark { x: 0.2, y },
            right: Landmark { x: 0.8, y },
        }
    }

    /// Feeds one Plucker moving straight down from `y0` at `speed` screen widths
    /// per second, one frame at a time, and returns each frame's Plucks.
    fn approach(
        pr: &mut Prediction,
        strings: &[Segment],
        y0: f32,
        speed: f32,
        frames: u64,
    ) -> Vec<Vec<Pluck>> {
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
        assert!(
            (plucks[0].timestamp_ns as f64 / 1e6 - 75.0).abs() < 1.0,
            "{plucks:?}"
        );
        assert!((plucks[0].velocity - 4.0).abs() < 1e-3, "{plucks:?}");
    }

    #[test]
    fn the_real_crossing_after_a_predicted_pluck_does_not_pluck_again() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        // Same approach, carried on past the String (crossed between frames 4 and 5).
        let frames = approach(&mut pr, &strings, 0.2, 4.0, 8);
        assert_eq!(frames.concat().len(), 1, "{frames:?}");
        assert_eq!(pr.false_plucks(), 0);
    }

    #[test]
    fn a_near_miss_that_stops_short_is_a_false_pluck() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        let mut plucks = approach(&mut pr, &strings, 0.2, 4.0, 4).concat();
        // Stops dead at 0.48, just short of the String, from frame 4 on.
        for i in 4..20 {
            plucks.extend(pr.update(i * FRAME, [p(0.5, 0.48), None], &strings));
            // Still within 250 ms of the prediction: not yet known to be FALSE.
            if i * FRAME < 32 * MS + 250 * MS {
                assert_eq!(pr.false_plucks(), 0, "frame {i}");
            }
        }
        assert_eq!(plucks.len(), 1, "{plucks:?}");
        assert_eq!(pr.false_plucks(), 1);
    }

    #[test]
    fn a_fast_approach_braking_to_stop_short_is_not_predicted() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        // 4 widths/s from 0.40, braking at 100 widths/s² so it stops at 0.48 after 40 ms.
        let y = |t: f32| {
            let t = t.min(0.04);
            0.40 + 4.0 * t - 50.0 * t * t
        };
        let mut plucks = Vec::new();
        for i in 0..20 {
            let t = i * FRAME;
            plucks.extend(pr.update(t, [p(0.5, y(t as f32 / 1e9)), None], &strings));
        }
        assert!(plucks.is_empty(), "{plucks:?}");
        assert_eq!(pr.false_plucks(), 0);
    }

    #[test]
    fn a_slow_approach_plucks_only_on_the_real_crossing() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        // 0.5 widths/s from 0.45: crosses at 100 ms, between frames 6 (96 ms) and 7 (112 ms).
        let frames = approach(&mut pr, &strings, 0.45, 0.5, 10);
        for (i, f) in frames.iter().enumerate() {
            assert_eq!(f.len(), usize::from(i == 7), "frame {i}: {frames:?}");
        }
        assert!(
            (frames[7][0].timestamp_ns as f64 / 1e6 - 100.0).abs() < 1.0,
            "{frames:?}"
        );
    }

    #[test]
    fn s5_a_prediction_within_60_ms_of_the_strings_last_pluck_is_jitter() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        // Left crosses down at 13.3 ms, then rests. Right comes up at 4 widths/s from 0.7:
        // predicted from frame 2 to cross at 50 ms, only 37 ms after the left's Pluck.
        let left = [0.4, 0.52, 0.52, 0.52, 0.52, 0.52];
        let mut plucks = Vec::new();
        for (i, &y) in left.iter().enumerate() {
            let t = i as u64 * FRAME;
            let right = 0.7 - 4.0 * t as f32 / 1e9;
            plucks.extend(pr.update(t, [p(0.3, y), p(0.7, right)], &strings));
        }
        assert_eq!(plucks.len(), 1, "{plucks:?}");
        assert!(
            (plucks[0].timestamp_ns as f64 / 1e6 - 13.3).abs() < 1.0,
            "{plucks:?}"
        );
        assert_eq!(pr.false_plucks(), 0);
    }

    #[test]
    fn s9_a_plucker_appearing_on_a_string_is_not_predicted_to_pluck_it() {
        let strings = [across(0.5)];
        let mut pr = Prediction::new(1.0, DEFAULT_LOOKAHEAD_NS);
        // Appears just below the String and moves up fast: it would cross at once.
        let plucks = approach(&mut pr, &strings, 0.52, -1.5, 3).concat();
        assert!(plucks.is_empty(), "{plucks:?}");
        assert_eq!(pr.false_plucks(), 0);
    }

    #[test]
    fn the_lookahead_sets_how_early_a_pluck_fires() {
        let strings = [across(0.5)];
        // The fast approach crosses at 75 ms. The first frame with 3 frames of path
        // is 2 (32 ms); with a 20 ms lookahead it waits for frame 4 (64 ms).
        for (lookahead, frame) in [(DEFAULT_LOOKAHEAD_NS, 2), (20 * MS, 4)] {
            let mut pr = Prediction::new(1.0, lookahead);
            let frames = approach(&mut pr, &strings, 0.2, 4.0, 8);
            let fired: Vec<_> = (0..8).filter(|&i| !frames[i].is_empty()).collect();
            assert_eq!(fired, [frame], "lookahead {lookahead}: {frames:?}");
        }
    }

    #[test]
    fn velocity_is_fitted_over_several_frames_not_the_last_two() {
        // 4 screen widths/s down, with 0.01 of jitter on the newest frame.
        let ys = [0.0, 0.064, 0.128, 0.202];
        let history: Vec<_> = ys
            .iter()
            .enumerate()
            .map(|(i, &y)| (i as u64 * FRAME, Landmark { x: 0.5, y }))
            .collect();
        let (vx, vy) = fitted_velocity(&history).unwrap();
        // The last two frames alone would say 4.625; the fit, worked by hand, says 4.1875.
        assert!(vx.abs() < 1e-4);
        assert!((vy - 4.1875).abs() < 1e-3, "{vy}");
        assert_eq!(
            fitted_velocity(&history[..2]),
            None,
            "two frames are not enough"
        );
    }
}
