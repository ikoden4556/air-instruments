//! Per-Hand Pinch detection with hysteresis, and the Anchor of a pinching Hand.

use crate::frame::{Hand, Landmark};

/// The point a pinching Hand holds one end of a Stretch by.
pub type Anchor = Landmark;

/// Tracks whether one Hand Pinches.
#[derive(Debug, Clone, Default)]
pub struct PinchDetector {
    pinching: bool,
}

impl PinchDetector {
    pub fn new() -> Self {
        Self::default()
    }

    /// Feeds the Hand's latest landmarks; returns its Anchor while it Pinches.
    pub fn update(&mut self, hand: &Hand) -> Option<Anchor> {
        let l = &hand.landmarks;
        let (thumb, index) = (l[THUMB_TIP], l[INDEX_TIP]);
        let ratio = distance(thumb, index) / distance(l[WRIST], l[MIDDLE_KNUCKLE]);
        let threshold = if self.pinching { RELEASE_BELOW } else { START_BELOW };
        self.pinching = ratio < threshold;
        self.pinching.then(|| Landmark { x: (thumb.x + index.x) / 2.0, y: (thumb.y + index.y) / 2.0 })
    }
}

// MediaPipe hand landmark indices.
const WRIST: usize = 0;
const THUMB_TIP: usize = 4;
const INDEX_TIP: usize = 8;
const MIDDLE_KNUCKLE: usize = 9;

/// Thumb-to-index distance, as a fraction of hand size, that starts a Pinch.
const START_BELOW: f32 = 0.22;
/// The Pinch holds until the distance rises above this.
const RELEASE_BELOW: f32 = 0.32;

fn distance(a: Landmark, b: Landmark) -> f32 {
    (a.x - b.x).hypot(a.y - b.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{Handedness, LANDMARKS_PER_HAND};

    /// A Hand whose size (wrist to middle knuckle) is `size`, with thumb tip and
    /// index tip `gap` × size apart, side by side around (0.5, 0.5).
    fn hand(size: f32, gap: f32) -> Hand {
        let mut landmarks = [Landmark { x: 0.5, y: 0.5 }; LANDMARKS_PER_HAND];
        landmarks[0] = Landmark { x: 0.5, y: 0.5 + size }; // wrist
        landmarks[9] = Landmark { x: 0.5, y: 0.5 }; // middle-finger knuckle
        landmarks[4] = Landmark { x: 0.5 - gap * size / 2.0, y: 0.5 }; // thumb tip
        landmarks[8] = Landmark { x: 0.5 + gap * size / 2.0, y: 0.5 }; // index tip
        Hand { handedness: Handedness::Right, landmarks }
    }

    fn pinches(d: &mut PinchDetector, size: f32, gap: f32) -> bool {
        d.update(&hand(size, gap)).is_some()
    }

    #[test]
    fn a_hand_enters_pinch_below_0_22_and_leaves_above_0_32() {
        let mut d = PinchDetector::new();
        assert!(!pinches(&mut d, 0.2, 0.5));
        assert!(pinches(&mut d, 0.2, 0.2));
        assert!(!pinches(&mut d, 0.2, 0.4));
    }

    #[test]
    fn a_gap_between_the_thresholds_keeps_the_current_state() {
        let mut d = PinchDetector::new();
        // Not pinching: wobbling between 0.22 and 0.32 does not start a Pinch.
        for gap in [0.25, 0.30, 0.23, 0.31] {
            assert!(!pinches(&mut d, 0.2, gap), "started at {gap}");
        }
        assert!(pinches(&mut d, 0.2, 0.1));
        // Pinching: the same wobble does not end it.
        for gap in [0.25, 0.30, 0.23, 0.31] {
            assert!(pinches(&mut d, 0.2, gap), "released at {gap}");
        }
    }

    #[test]
    fn near_and_far_hands_pinch_alike() {
        // Same hand shapes, one close to the camera (size 0.4), one far away (size 0.08).
        for gap in [0.5, 0.2, 0.27, 0.4] {
            let (mut near, mut far) = (PinchDetector::new(), PinchDetector::new());
            assert_eq!(pinches(&mut near, 0.4, gap), pinches(&mut far, 0.08, gap), "gap {gap}");
        }
        // A tip distance of 0.03 screen units is a Pinch for the near hand only.
        let (mut near, mut far) = (PinchDetector::new(), PinchDetector::new());
        assert!(pinches(&mut near, 0.4, 0.03 / 0.4));
        assert!(!pinches(&mut far, 0.08, 0.03 / 0.08));
    }

    #[test]
    fn the_anchor_is_midway_between_thumb_tip_and_index_tip() {
        let mut h = hand(0.2, 0.1);
        h.landmarks[4] = Landmark { x: 0.30, y: 0.40 }; // thumb tip
        h.landmarks[8] = Landmark { x: 0.32, y: 0.42 }; // index tip
        let anchor = PinchDetector::new().update(&h).expect("pinching");
        assert!((anchor.x - 0.31).abs() < 1e-6 && (anchor.y - 0.41).abs() < 1e-6, "{anchor:?}");
    }
}
