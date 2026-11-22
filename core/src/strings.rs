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
        self.stretch = match (left, right) {
            (HandState::Pinching(left), HandState::Pinching(right)) => Some(Segment { left, right }),
            _ => None,
        };
    }

    pub fn stretch(&self) -> Option<Segment> {
        self.stretch
    }

    /// Frozen Strings, oldest first.
    pub fn strings(&self) -> &[Segment] {
        &self.strings
    }
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
}
