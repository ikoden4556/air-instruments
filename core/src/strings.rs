//! The String model: Stretches held between two pinching Hands, Frozen into Strings.

use crate::pinch::Anchor;

/// What one Hand is doing in a frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HandState {
    /// Out of tracking this frame.
    Missing,
    /// Tracked, not Pinching.
    Open,
    Pinching(Anchor),
}

/// A straight segment between two points in screen space: a Stretch or a String.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    pub left: Anchor,
    pub right: Anchor,
}

/// The current Stretch, if any, and every Frozen String.
#[derive(Debug, Clone)]
pub struct Strings {
    aspect: f32,
    stretch: Option<Segment>,
    /// Timestamp of the last frame in which both Hands Pinched.
    both_pinched_ns: u64,
    strings: Vec<Segment>,
}

impl Strings {
    /// `aspect` is screen height / width, so lengths can be measured in screen widths.
    pub fn new(aspect: f32) -> Self {
        Self { aspect, stretch: None, both_pinched_ns: 0, strings: Vec::new() }
    }

    /// Feeds one frame: its timestamp and what each Hand is doing.
    pub fn update(&mut self, timestamp_ns: u64, left: HandState, right: HandState) {
        use HandState::{Missing, Pinching};
        match (left, right) {
            (Pinching(left), Pinching(right)) => {
                self.stretch = Some(Segment { left, right });
                self.both_pinched_ns = timestamp_ns;
            }
            // A Hand out of tracking, and no Hand let go: the Stretch waits.
            (Missing | Pinching(_), Missing | Pinching(_)) => {
                if timestamp_ns.saturating_sub(self.both_pinched_ns) > MAX_GAP_NS {
                    self.stretch = None;
                }
            }
            _ => {
                let aspect = self.aspect;
                let long_enough = |s: &Segment| width_fraction(s, aspect) >= MIN_LENGTH;
                let frozen = self.stretch.take().filter(long_enough);
                self.strings.extend(frozen);
                if self.strings.len() > MAX_STRINGS {
                    self.strings.remove(0);
                }
            }
        }
    }

    pub fn stretch(&self) -> Option<Segment> {
        self.stretch
    }

    /// Frozen Strings, oldest first.
    pub fn strings(&self) -> &[Segment] {
        &self.strings
    }

    /// Removes the newest String.
    pub fn undo(&mut self) {
        self.strings.pop();
    }
}

/// The most Strings at once; Freezing another removes the oldest.
const MAX_STRINGS: usize = 12;

/// A Stretch shorter than this fraction of screen width is Discarded, not Frozen.
const MIN_LENGTH: f32 = 0.08;

/// How long a Hand of a Stretch may be out of tracking before the Stretch is Discarded.
const MAX_GAP_NS: u64 = 150_000_000;

/// The Segment's on-screen length as a fraction of screen width;
/// `aspect` is screen height / width.
fn width_fraction(s: &Segment, aspect: f32) -> f32 {
    (s.right.x - s.left.x).hypot((s.right.y - s.left.y) * aspect)
}

#[cfg(test)]
mod tests {
    use super::HandState::{Missing, Open, Pinching};
    use super::*;

    const MS: u64 = 1_000_000;

    fn at(x: f32, y: f32) -> HandState {
        Pinching(Anchor { x, y })
    }

    #[test]
    fn s1_one_pinching_hand_makes_no_stretch_and_no_string() {
        let mut s = Strings::new(1.0);
        s.update(0, at(0.2, 0.5), Open);
        assert_eq!(s.stretch(), None);
        s.update(16 * MS, Open, Open);
        s.update(33 * MS, Open, at(0.8, 0.5));
        assert_eq!(s.stretch(), None);
        s.update(50 * MS, Open, Open);
        assert!(s.strings().is_empty());
        // Both Pinching does make a Stretch between the Anchors.
        s.update(66 * MS, at(0.2, 0.5), at(0.8, 0.5));
        let expected = Segment { left: Anchor { x: 0.2, y: 0.5 }, right: Anchor { x: 0.8, y: 0.5 } };
        assert_eq!(s.stretch(), Some(expected));
    }

    #[test]
    fn s2_the_first_hand_to_stop_pinching_freezes_the_stretch() {
        let mut s = Strings::new(1.0);
        s.update(0, at(0.2, 0.5), at(0.8, 0.5));
        s.update(16 * MS, at(0.2, 0.4), at(0.8, 0.6));
        // Left lets go; the String is where the Anchors were in the last frame both Pinched.
        s.update(33 * MS, Open, at(0.9, 0.9));
        let frozen = Segment { left: Anchor { x: 0.2, y: 0.4 }, right: Anchor { x: 0.8, y: 0.6 } };
        assert_eq!(s.strings(), &[frozen]);
        assert_eq!(s.stretch(), None);
        // The right Hand still Pinching, then letting go, adds nothing.
        s.update(50 * MS, Open, at(0.9, 0.8));
        s.update(66 * MS, Open, Open);
        assert_eq!(s.strings(), &[frozen]);
    }

    /// Holds a Stretch from `left` to `right`, then lets go with the left Hand.
    fn stretch_then_release(s: &mut Strings, t: u64, left: (f32, f32), right: (f32, f32)) {
        s.update(t, at(left.0, left.1), at(right.0, right.1));
        s.update(t + 16 * MS, Open, at(right.0, right.1));
    }

    #[test]
    fn s3_a_stretch_under_8_percent_of_screen_width_is_discarded() {
        // A phone held upright: the screen is twice as tall as it is wide.
        let mut s = Strings::new(2.0);
        // Across: 0.07 and 0.09 of the width.
        stretch_then_release(&mut s, 0, (0.50, 0.5), (0.57, 0.5));
        assert!(s.strings().is_empty());
        stretch_then_release(&mut s, 50 * MS, (0.50, 0.5), (0.59, 0.5));
        assert_eq!(s.strings().len(), 1);
        // Up and down: 0.045 of the height is 0.09 of the width, so it is kept;
        // 0.035 of the height is 0.07 of the width, so it is Discarded.
        stretch_then_release(&mut s, 100 * MS, (0.5, 0.500), (0.5, 0.545));
        assert_eq!(s.strings().len(), 2);
        stretch_then_release(&mut s, 150 * MS, (0.5, 0.500), (0.5, 0.535));
        assert_eq!(s.strings().len(), 2);
    }

    #[test]
    fn s4_a_stretch_survives_150_ms_out_of_tracking_then_is_discarded() {
        let mut s = Strings::new(1.0);
        // The left Hand drops out for 150 ms and comes back still Pinching.
        s.update(0, at(0.2, 0.5), at(0.8, 0.5));
        s.update(100 * MS, Missing, at(0.8, 0.5));
        s.update(150 * MS, Missing, at(0.8, 0.5));
        assert!(s.stretch().is_some());
        s.update(166 * MS, at(0.3, 0.5), at(0.8, 0.5));
        s.update(183 * MS, at(0.3, 0.5), Open);
        let frozen = Segment { left: Anchor { x: 0.3, y: 0.5 }, right: Anchor { x: 0.8, y: 0.5 } };
        assert_eq!(s.strings(), &[frozen]);

        // This time it stays out a frame longer: Discarded, and nothing Freezes after.
        s.update(200 * MS, at(0.2, 0.5), at(0.8, 0.5));
        s.update(300 * MS, at(0.2, 0.5), Missing);
        s.update(366 * MS, at(0.2, 0.5), Missing);
        assert_eq!(s.stretch(), None);
        s.update(383 * MS, Open, Open);
        assert_eq!(s.strings(), &[frozen]);
    }

    /// Freezes String number `n` (0, 1, 2, ...), each at its own height.
    fn freeze_nth(s: &mut Strings, n: u64) {
        stretch_then_release(s, n * 50 * MS, (0.2, n as f32 / 20.0), (0.8, n as f32 / 20.0));
    }

    fn heights(s: &Strings) -> Vec<u64> {
        s.strings().iter().map(|seg| (seg.left.y * 20.0).round() as u64).collect()
    }

    #[test]
    fn a_13th_string_removes_the_oldest() {
        let mut s = Strings::new(1.0);
        for n in 0..12 {
            freeze_nth(&mut s, n);
        }
        assert_eq!(heights(&s), (0..12).collect::<Vec<_>>());
        freeze_nth(&mut s, 12);
        assert_eq!(heights(&s), (1..13).collect::<Vec<_>>());
    }

    #[test]
    fn undo_removes_the_newest_string() {
        let mut s = Strings::new(1.0);
        for n in 0..3 {
            freeze_nth(&mut s, n);
        }
        s.undo();
        assert_eq!(heights(&s), [0, 1]);
        s.undo();
        s.undo();
        s.undo(); // nothing left: no effect
        assert!(s.strings().is_empty());
    }
}
