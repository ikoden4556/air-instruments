//! The Engine over flat number arrays, the shape JNI passes cheaply. Host-testable.

use air_core::engine::Engine;
use air_core::frame::{Frame, Hand, Handedness, LANDMARKS_PER_HAND, Landmark};
use air_core::pitch::Key;

use crate::queue::{PluckQueue, QueuedPluck};

/// Floats per Hand in the input: handedness (0 left, 1 right), then x, y per landmark.
pub const HAND_FLOATS: usize = 1 + 2 * LANDMARKS_PER_HAND;
/// Numbers per Pluck in the output: String, frequency, Velocity, crossing time (ns), predicted (0/1).
pub const PLUCK_NUMBERS: usize = 5;
/// Render header: Stretch (4), Anchors L/R (4), Pluckers L/R (4), String count (1); then
/// 4 per String (left x, y, right x, y). Missing points are NaN. Same layout as the web bindings.
pub const RENDER_HEADER: usize = 13;
/// Room for every String (the Engine keeps at most 12).
pub const RENDER_MAX: usize = RENDER_HEADER + 4 * 12;

pub fn key(root: i32) -> Key {
    Key {
        root: root.rem_euclid(12) as u8,
    }
}

pub struct FlatEngine {
    pub engine: Engine,
}

impl FlatEngine {
    pub fn new(aspect: f32, key_root: i32) -> Self {
        Self {
            engine: Engine::new(aspect, key(key_root)),
        }
    }

    /// Feeds one frame (`hand_count` Hands from `hands`, taken at `timestamp_ns` on
    /// CLOCK_MONOTONIC), sends its Plucks to `queue`, and writes as many as fit into `out`.
    /// Returns how many Plucks there were.
    pub fn update(
        &mut self,
        timestamp_ns: i64,
        hands: &[f32],
        hand_count: usize,
        queue: &PluckQueue,
        out: &mut [f64],
    ) -> usize {
        let hands = hands
            .as_chunks::<HAND_FLOATS>()
            .0
            .iter()
            .take(hand_count.min(2))
            .map(|h| Hand {
                handedness: if h[0] < 0.5 {
                    Handedness::Left
                } else {
                    Handedness::Right
                },
                landmarks: std::array::from_fn(|i| Landmark {
                    x: h[1 + 2 * i],
                    y: h[2 + 2 * i],
                }),
            })
            .collect();
        let frame = Frame {
            timestamp_ns: timestamp_ns.max(0) as u64,
            hands,
        };
        let plucks = self.engine.update(&frame);
        for (p, slot) in plucks
            .iter()
            .zip(out.as_chunks_mut::<PLUCK_NUMBERS>().0.iter_mut())
        {
            slot.copy_from_slice(&[
                p.string as f64,
                p.frequency as f64,
                p.velocity as f64,
                p.timestamp_ns as f64,
                p.predicted as u8 as f64,
            ]);
        }
        for p in &plucks {
            queue.push(QueuedPluck {
                frequency: p.frequency,
                velocity: p.velocity,
                crossing_ns: p.timestamp_ns as i64,
            });
        }
        plucks.len()
    }

    /// Writes the render data into `out` (at least `RENDER_MAX` long); returns its length.
    pub fn render(&self, out: &mut [f32]) -> usize {
        let point = |p: Option<Landmark>| p.map_or([f32::NAN; 2], |p| [p.x, p.y]);
        let stretch = self.engine.stretch();
        let mut n = 0;
        let mut put = |vals: &[f32]| {
            out[n..n + vals.len()].copy_from_slice(vals);
            n += vals.len();
        };
        put(&point(stretch.map(|s| s.left)));
        put(&point(stretch.map(|s| s.right)));
        for p in self
            .engine
            .anchors()
            .into_iter()
            .chain(self.engine.pluckers())
        {
            put(&point(p));
        }
        let strings = self.engine.strings();
        put(&[strings.len() as f32]);
        for s in strings {
            put(&[s.left.x, s.left.y, s.right.x, s.right.y]);
        }
        n
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FRAME_NS: i64 = 16_666_667;

    /// A Hand 0.1 wide with its index fingertip at (x, y), Pinching or open, as flat numbers.
    fn hand(right: bool, x: f32, y: f32, pinching: bool) -> Vec<f32> {
        let mut l = [(x, y + 0.15); LANDMARKS_PER_HAND];
        l[0] = (x, y + 0.2);
        l[9] = (x, y + 0.1);
        l[8] = (x, y);
        l[4] = (if pinching { x } else { x + 0.08 }, y);
        let mut v = vec![right as u8 as f32];
        for (px, py) in l {
            v.extend([px, py]);
        }
        v
    }

    fn feed(e: &mut FlatEngine, q: &PluckQueue, frame: i64, hands: &[Vec<f32>]) -> Vec<f64> {
        let flat: Vec<f32> = hands.concat();
        let mut out = [0.0; PLUCK_NUMBERS * 8];
        let n = e.update(frame * FRAME_NS, &flat, hands.len(), q, &mut out);
        out[..n * PLUCK_NUMBERS].to_vec()
    }

    #[test]
    fn a_string_built_and_swiped_plucks_once_into_the_output_and_the_queue() {
        let (mut e, q) = (FlatEngine::new(1.0, 0), PluckQueue::new());
        let mut plucks = Vec::new();
        let mut f = 0;
        let mut run = |n: usize, hands: &dyn Fn(usize) -> Vec<Vec<f32>>, plucks: &mut Vec<f64>| {
            for i in 0..n {
                plucks.extend(feed(&mut e, &q, f, &hands(i)));
                f += 1;
            }
        };
        run(
            10,
            &|_| vec![hand(false, 0.2, 0.5, true), hand(true, 0.8, 0.5, true)],
            &mut plucks,
        );
        run(
            10,
            &|_| vec![hand(false, 0.2, 0.5, false), hand(true, 0.8, 0.5, false)],
            &mut plucks,
        );
        run(5, &|_| vec![], &mut plucks);
        run(
            11,
            &|i| vec![hand(true, 0.5, 0.3 + 0.04 * i as f32, false)],
            &mut plucks,
        );

        assert_eq!(plucks.len(), PLUCK_NUMBERS, "{plucks:?}");
        assert!((plucks[1] - 293.66).abs() < 0.05, "D4: {plucks:?}");
        let queued = q.pop().expect("the Pluck reached the audio queue");
        assert!((queued.frequency as f64 - plucks[1]).abs() < 1e-3);
        assert_eq!(queued.crossing_ns as f64, plucks[3]);
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn render_data_has_the_web_layout() {
        let (mut e, q) = (FlatEngine::new(1.0, 0), PluckQueue::new());
        for f in 0..5 {
            feed(
                &mut e,
                &q,
                f,
                &[hand(false, 0.2, 0.5, true), hand(true, 0.8, 0.5, true)],
            );
        }
        let mut out = [0.0; RENDER_MAX];
        let n = e.render(&mut out);
        assert_eq!(n, RENDER_HEADER);
        assert_eq!(&out[0..4], &[0.2, 0.5, 0.8, 0.5]);
        assert_eq!(&out[4..8], &[0.2, 0.5, 0.8, 0.5]);
        assert!(out[8..12].iter().all(|v| v.is_nan()));
        assert_eq!(out[12], 0.0);
    }

    #[test]
    fn key_roots_wrap_round_the_octave() {
        assert_eq!(key(14).root, 2);
        assert_eq!(key(-1).root, 11);
    }
}
