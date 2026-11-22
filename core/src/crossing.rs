//! Plucks: each Plucker's path between two frames, tested against every String.

use crate::frame::Landmark;
use crate::strings::Segment;

/// One String Plucked once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pluck {
    /// Index into the Strings passed to `Crossing::update`.
    pub string: usize,
    /// When the Plucker crossed, interpolated between the two frames.
    pub timestamp_ns: u64,
    /// The Plucker's speed while crossing, in screen widths per second.
    pub velocity: f32,
}

/// Remembers each Plucker's last position to find the Strings it crosses next.
#[derive(Debug, Clone)]
pub struct Crossing {
    aspect: f32,
    previous_ns: u64,
    previous: [Option<Landmark>; 2],
    /// Strings Plucked in the last 60 ms, with when.
    recent: Vec<(Segment, u64)>,
}

impl Crossing {
    /// `aspect` is screen height / width, so speeds can be measured in screen widths.
    pub fn new(aspect: f32) -> Self {
        Self {
            aspect,
            previous_ns: 0,
            previous: [None; 2],
            recent: Vec::new(),
        }
    }

    /// Feeds one frame: its timestamp, the left and right Hand's Plucker
    /// (`None` while that Hand Pinches or is out of tracking), and the Strings.
    /// Returns the Plucks since the previous frame, in the order they happened.
    pub fn update(
        &mut self,
        timestamp_ns: u64,
        pluckers: [Option<Landmark>; 2],
        strings: &[Segment],
    ) -> Vec<Pluck> {
        let (t0, t1) = (self.previous_ns, timestamp_ns);
        let mut plucks = Vec::new();
        for (from, to) in self.previous.into_iter().zip(pluckers) {
            let (Some(from), Some(to)) = (from, to) else {
                continue;
            };
            let seconds = (t1 - t0) as f32 / 1e9;
            let velocity = (to.x - from.x).hypot((to.y - from.y) * self.aspect) / seconds;
            for (string, s) in strings.iter().enumerate() {
                let Some(u) = crossing_point(from, to, s) else {
                    continue;
                };
                let timestamp_ns = t0 + ((t1 - t0) as f64 * u as f64).round() as u64;
                let pluck = Pluck {
                    string,
                    timestamp_ns,
                    velocity,
                };
                // Both Pluckers crossed it this frame: one Pluck, the faster one's.
                match plucks
                    .iter_mut()
                    .find(|pl: &&mut Pluck| pl.string == string)
                {
                    Some(other) if other.velocity < velocity => *other = pluck,
                    Some(_) => {}
                    None => plucks.push(pluck),
                }
            }
        }
        plucks.sort_by_key(|pl| pl.timestamp_ns);
        plucks.retain(|pl| {
            let s = strings[pl.string];
            let jitter = self
                .recent
                .iter()
                .any(|&(r, t)| r == s && pl.timestamp_ns.saturating_sub(t) < JITTER_NS);
            if !jitter {
                self.recent.push((s, pl.timestamp_ns));
            }
            !jitter
        });
        self.recent
            .retain(|&(_, t)| t1.saturating_sub(t) < JITTER_NS);
        self.previous_ns = timestamp_ns;
        self.previous = pluckers;
        plucks
    }
}

/// Re-crossing a String this soon after it Plucked is jitter, not a Pluck.
const JITTER_NS: u64 = 60_000_000;

/// How far along the path `from` → `to` it crosses String `s` (0..=1), if it does.
/// A Plucker resting exactly on the String counts as on its upper side,
/// so the crossing is found in one frame only.
fn crossing_point(from: Landmark, to: Landmark, s: &Segment) -> Option<f32> {
    let (a, b) = (s.left, s.right);
    let side = |p: Landmark| (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x);
    let (d0, d1) = (side(from), side(to));
    if (d0 > 0.0) == (d1 > 0.0) {
        return None;
    }
    let u = d0 / (d0 - d1);
    let (qx, qy) = (from.x + u * (to.x - from.x), from.y + u * (to.y - from.y));
    let (sx, sy) = (b.x - a.x, b.y - a.y);
    let v = ((qx - a.x) * sx + (qy - a.y) * sy) / (sx * sx + sy * sy);
    (0.0..=1.0).contains(&v).then_some(u)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: u64 = 1_000_000;

    fn p(x: f32, y: f32) -> Option<Landmark> {
        Some(Landmark { x, y })
    }

    /// A String across the screen at height `y`.
    fn across(y: f32) -> Segment {
        Segment {
            left: Landmark { x: 0.2, y },
            right: Landmark { x: 0.8, y },
        }
    }

    /// Pluck times to the nearest microsecond.
    fn micros(plucks: &[Pluck]) -> Vec<u64> {
        plucks
            .iter()
            .map(|pl| (pl.timestamp_ns + 500) / 1000)
            .collect()
    }

    #[test]
    fn a_strum_plucks_each_string_in_order() {
        let strings = [across(0.6), across(0.4), across(0.5)];
        let mut c = Crossing::new(1.0);
        assert!(c.update(0, [p(0.5, 0.3), None], &strings).is_empty());
        // Down 0.4 of the width in 100 ms: 4 screen widths per second.
        let plucks = c.update(100 * MS, [p(0.5, 0.7), None], &strings);
        let order: Vec<_> = plucks.iter().map(|pl| pl.string).collect();
        assert_eq!(order, [1, 2, 0]);
        // y = 0.4 is a quarter of the way down the path, 0.5 half, 0.6 three quarters.
        assert_eq!(micros(&plucks), [25_000, 50_000, 75_000]);
        for pl in &plucks {
            assert!((pl.velocity - 4.0).abs() < 1e-4, "{pl:?}");
        }
    }

    #[test]
    fn s5_down_then_up_plucks_twice_but_recrossing_within_60_ms_is_jitter() {
        let strings = [across(0.5)];
        let mut c = Crossing::new(1.0);
        c.update(0, [p(0.5, 0.4), None], &strings);
        // Down: crosses at 8 ms.
        assert_eq!(
            micros(&c.update(16 * MS, [p(0.5, 0.6), None], &strings)),
            [8_000]
        );
        // Jitters back up across it at 24 ms: ignored.
        assert!(c.update(33 * MS, [p(0.5, 0.4), None], &strings).is_empty());
        c.update(50 * MS, [p(0.5, 0.6), None], &strings); // crosses at 41 ms: ignored
        c.update(100 * MS, [p(0.5, 0.6), None], &strings);
        // Back up, crossing at 108 ms, 100 ms after the first Pluck: a second Pluck.
        assert_eq!(
            micros(&c.update(116 * MS, [p(0.5, 0.4), None], &strings)),
            [108_000]
        );
    }

    #[test]
    fn s6_passing_through_where_two_strings_cross_plucks_both() {
        let x = |a: (f32, f32), b: (f32, f32)| Segment {
            left: Landmark { x: a.0, y: a.1 },
            right: Landmark { x: b.0, y: b.1 },
        };
        // An X: both Strings meet at (0.5, 0.5).
        let strings = [x((0.2, 0.2), (0.8, 0.8)), x((0.2, 0.8), (0.8, 0.2))];
        let mut c = Crossing::new(1.0);
        c.update(0, [None, p(0.5, 0.3)], &strings);
        let plucks = c.update(20 * MS, [None, p(0.5, 0.7)], &strings);
        let mut hit: Vec<_> = plucks.iter().map(|pl| pl.string).collect();
        hit.sort();
        assert_eq!(hit, [0, 1]);
        assert_eq!(micros(&plucks), [10_000, 10_000]);
    }

    #[test]
    fn s7_a_stretch_sweeping_across_a_string_plucks_nothing() {
        use crate::strings::{HandState::Open, HandState::Pinching, Strings};
        let at = |x: f32, y: f32| Pinching(Landmark { x, y });
        let mut s = Strings::new(1.0);
        let mut c = Crossing::new(1.0);
        // Freeze a String at height 0.5.
        s.update(0, at(0.2, 0.5), at(0.8, 0.5));
        s.update(16 * MS, Open, at(0.8, 0.5));
        // A new Stretch sweeps down across it. Both Hands Pinch, so neither is a Plucker.
        for (i, y) in [0.3, 0.45, 0.55, 0.7].into_iter().enumerate() {
            let t = (100 + 16 * i as u64) * MS;
            s.update(t, at(0.1, y), at(0.9, y));
            assert!(s.stretch().is_some());
            assert!(c.update(t, [None, None], s.strings()).is_empty());
        }
    }

    #[test]
    fn s8_both_pluckers_crossing_one_string_in_a_frame_pluck_once_at_the_faster_velocity() {
        let strings = [across(0.5)];
        for slow_first in [true, false] {
            let mut c = Crossing::new(1.0);
            // Left moves 0.2 in 100 ms (2 widths/s), right moves 0.6 (6 widths/s).
            let (slow, fast) = ([p(0.3, 0.4), p(0.3, 0.6)], [p(0.7, 0.2), p(0.7, 0.8)]);
            let pair = |i: usize| {
                if slow_first {
                    [slow[i], fast[i]]
                } else {
                    [fast[i], slow[i]]
                }
            };
            c.update(0, pair(0), &strings);
            let plucks = c.update(100 * MS, pair(1), &strings);
            assert_eq!(plucks.len(), 1, "{plucks:?}");
            assert!((plucks[0].velocity - 6.0).abs() < 1e-4, "{plucks:?}");
        }
    }
}
