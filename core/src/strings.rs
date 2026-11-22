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
    strings: Vec<Segment>,
}

impl Strings {
    /// `aspect` is screen height / width, so lengths can be measured in screen widths.
    pub fn new(aspect: f32) -> Self {
        Self { aspect, stretch: None, strings: Vec::new() }
    }

    /// Feeds one frame: its timestamp and what each Hand is doing.
    pub fn update(&mut self, _timestamp_ns: u64, left: HandState, right: HandState) {
        match (left, right) {
            (HandState::Pinching(left), HandState::Pinching(right)) => {
                self.stretch = Some(Segment { left, right });
            }
            _ => {
                let aspect = self.aspect;
                let long_enough = |s: &Segment| width_fraction(s, aspect) >= MIN_LENGTH;
                let frozen = self.stretch.take().filter(long_enough);
                self.strings.extend(frozen);
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
}

/// A Stretch shorter than this fraction of screen width is Discarded, not Frozen.
const MIN_LENGTH: f32 = 0.08;

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
        let _ = Missing;
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
}
