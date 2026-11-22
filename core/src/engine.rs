//! The engine: one call per camera frame, from Hands to Pluck events and render data.
//! The synth is not in here: the shells feed it Pluck events on the audio thread.

use crate::filter::{LandmarkFilter, OneEuroSettings};
use crate::frame::{Frame, Handedness, Landmark};
use crate::pinch::{self, Anchor, PinchDetector};
use crate::pitch::{self, Key};
use crate::predict::{DEFAULT_LOOKAHEAD_NS, Prediction};
use crate::strings::{HandState, Segment, Strings, width_fraction};

/// One Pluck, ready for the synth.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PluckEvent {
    /// Index into `Engine::strings()`.
    pub string: usize,
    pub frequency: f32,
    /// How hard, 0..1.
    pub velocity: f32,
    /// When the Plucker crossed (or is predicted to), on the camera timeline.
    pub timestamp_ns: u64,
    pub predicted: bool,
}

/// The whole core but the synth: filter → Pinch → Strings → Plucks (predicted) → pitch.
pub struct Engine {
    aspect: f32,
    key: Key,
    filter: LandmarkFilter,
    /// [left, right]
    pinch: [PinchDetector; 2],
    strings: Strings,
    prediction: Prediction,
    anchors: [Option<Anchor>; 2],
    pluckers: [Option<Landmark>; 2],
}

impl Engine {
    /// `aspect` is screen height / width.
    pub fn new(aspect: f32, key: Key) -> Self {
        Self {
            aspect,
            key,
            filter: LandmarkFilter::new(FILTER),
            pinch: Default::default(),
            strings: Strings::new(aspect),
            prediction: Prediction::new(aspect, DEFAULT_LOOKAHEAD_NS),
            anchors: [None; 2],
            pluckers: [None; 2],
        }
    }

    /// Feeds one camera frame; returns its Plucks in the order they happen.
    pub fn update(&mut self, raw: &Frame) -> Vec<PluckEvent> {
        let frame = self.filter.filter(raw);
        let mut states = [HandState::Missing; 2];
        self.anchors = [None; 2];
        self.pluckers = [None; 2];
        for side in [Handedness::Left, Handedness::Right] {
            let i = side as usize;
            let find = |f: &Frame| f.hands.iter().find(|h| h.handedness == side).cloned();
            let (Some(raw_hand), Some(hand)) = (find(raw), find(&frame)) else {
                continue;
            };
            // Pinch on and off from the raw landmarks: the filter lags slow finger
            // movements, and the Pinch thresholds already ride out jitter. The Anchor
            // is placed from the filtered ones, so it holds steady.
            states[i] = match self.pinch[i].update(&raw_hand).map(|_| pinch::anchor(&hand)) {
                Some(anchor) => {
                    self.anchors[i] = Some(anchor);
                    HandState::Pinching(anchor)
                }
                None => {
                    self.pluckers[i] = Some(hand.landmarks[INDEX_TIP]);
                    HandState::Open
                }
            };
        }
        let [left, right] = states;
        self.strings.update(frame.timestamp_ns, left, right);
        let strings = self.strings.strings();
        let plucks = self
            .prediction
            .update(frame.timestamp_ns, self.pluckers, strings);
        plucks
            .into_iter()
            .map(|pl| {
                let length = width_fraction(&strings[pl.string], self.aspect);
                PluckEvent {
                    string: pl.string,
                    frequency: pitch::frequency(self.key, length, pitch::fret_at(pl.along)),
                    velocity: (pl.velocity / FULL_VELOCITY_SPEED).min(1.0),
                    timestamp_ns: pl.timestamp_ns,
                    predicted: pl.predicted,
                }
            })
            .collect()
    }

    /// Frozen Strings, oldest first.
    pub fn strings(&self) -> &[Segment] {
        self.strings.strings()
    }

    pub fn stretch(&self) -> Option<Segment> {
        self.strings.stretch()
    }

    /// Each Hand's Anchor while it Pinches: [left, right].
    pub fn anchors(&self) -> [Option<Anchor>; 2] {
        self.anchors
    }

    /// Each Hand's Plucker while it is tracked and not Pinching: [left, right].
    pub fn pluckers(&self) -> [Option<Landmark>; 2] {
        self.pluckers
    }

    /// Removes the newest String.
    pub fn undo(&mut self) {
        self.strings.undo();
    }

    /// Removes every String.
    pub fn clear(&mut self) {
        self.strings.clear();
    }

    pub fn set_key(&mut self, key: Key) {
        self.key = key;
    }

    /// Predicted Plucks whose String was never crossed in time, so far.
    pub fn false_plucks(&self) -> u32 {
        self.prediction.false_plucks()
    }
}

/// MediaPipe's index fingertip: the Plucker.
const INDEX_TIP: usize = 8;

/// Landmark jitter filter; a starting point for tuning on the phone.
const FILTER: OneEuroSettings = OneEuroSettings {
    min_cutoff: 1.0,
    beta: 10.0,
    derivative_cutoff: 1.0,
};

/// A Plucker this fast, in screen widths per second, Plucks at full Velocity.
const FULL_VELOCITY_SPEED: f32 = 5.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::{Hand, Handedness, LANDMARKS_PER_HAND};

    const FRAME_NS: u64 = 16_666_667;
    const C: Key = Key { root: 0 };

    /// A Hand 0.1 screen widths in size whose index fingertip is at (x, y),
    /// with the thumb tip on it (Pinching) or well away from it (open).
    fn hand(handedness: Handedness, x: f32, y: f32, pinching: bool) -> Hand {
        let mut landmarks = [Landmark { x, y: y + 0.15 }; LANDMARKS_PER_HAND];
        landmarks[0] = Landmark { x, y: y + 0.2 }; // wrist
        landmarks[9] = Landmark { x, y: y + 0.1 }; // middle knuckle
        landmarks[8] = Landmark { x, y }; // index tip
        let thumb_dx = if pinching { 0.0 } else { 0.08 };
        landmarks[4] = Landmark { x: x + thumb_dx, y }; // thumb tip
        Hand {
            handedness,
            landmarks,
        }
    }

    fn left(x: f32, y: f32, pinching: bool) -> Hand {
        hand(Handedness::Left, x, y, pinching)
    }

    fn right(x: f32, y: f32, pinching: bool) -> Hand {
        hand(Handedness::Right, x, y, pinching)
    }

    /// Plays scripted frames, one per `FRAME_NS`.
    struct Script {
        engine: Engine,
        frame: u64,
        plucks: Vec<PluckEvent>,
    }

    impl Script {
        fn new() -> Self {
            Self {
                engine: Engine::new(1.0, C),
                frame: 0,
                plucks: Vec::new(),
            }
        }

        fn play(&mut self, frames: usize, hands: impl Fn(usize) -> Vec<Hand>) {
            for i in 0..frames {
                let frame = Frame {
                    timestamp_ns: self.frame * FRAME_NS,
                    hands: hands(i),
                };
                self.frame += 1;
                self.plucks.extend(self.engine.update(&frame));
            }
        }

        /// Pinches at (0.2, y) and (0.8, y), lets go, and takes both Hands away:
        /// a String 0.6 screen widths long.
        fn build_string(&mut self, y: f32) {
            self.play(10, |_| vec![left(0.2, y, true), right(0.8, y, true)]);
            self.play(10, |_| vec![left(0.2, y, false), right(0.8, y, false)]);
            self.play(5, |_| vec![]);
        }

        /// The right Plucker swipes straight down through (x, 0.3) → (x, 0.7).
        fn swipe_down(&mut self, x: f32, frames: usize) {
            self.play(frames + 1, |i| {
                vec![right(x, 0.3 + 0.4 * i as f32 / frames as f32, false)]
            });
        }
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn a_slow_release_freezes_as_soon_as_the_fingers_part_not_when_the_filter_catches_up() {
        let mut s = Script::new();
        s.play(10, |_| vec![left(0.2, 0.5, true), right(0.8, 0.5, true)]);
        // The left thumb opens slowly: 0.04 of hand size per frame. It passes the
        // 0.32 release point in frame 8 of opening.
        let opening = |i: usize| {
            let mut l = left(0.2, 0.5, true);
            l.landmarks[4].x += 0.004 * (i + 1) as f32;
            vec![l, right(0.8, 0.5, true)]
        };
        s.play(7, opening);
        assert!(s.engine.strings().is_empty(), "froze before the fingers parted");
        s.play(1, |_| opening(7));
        assert_eq!(s.engine.strings().len(), 1);
    }

    #[test]
    fn pinch_release_and_swipe_plucks_once_at_the_right_pitch() {
        let mut s = Script::new();
        s.build_string(0.5);
        assert_eq!(s.engine.strings().len(), 1);
        s.swipe_down(0.5, 10);
        assert_eq!(s.plucks.len(), 1, "{:?}", s.plucks);
        // 0.6 wide is 4 Key steps up (A3); the middle is Fret 2: two more steps, D4.
        assert!(close(s.plucks[0].frequency, 293.66), "{:?}", s.plucks);
        assert_eq!(s.plucks[0].string, 0);
    }

    #[test]
    fn plucking_further_along_the_string_plays_a_higher_fret() {
        let mut s = Script::new();
        s.build_string(0.5);
        // 0.75 is 0.917 along the String: Fret 4, four steps above A3: G4.
        s.swipe_down(0.75, 10);
        assert_eq!(s.plucks.len(), 1, "{:?}", s.plucks);
        assert!(close(s.plucks[0].frequency, 392.00), "{:?}", s.plucks);
    }

    #[test]
    fn s1_a_pinching_hand_neither_stretches_nor_plucks() {
        let mut s = Script::new();
        s.build_string(0.5);
        // The left Hand Pinches and sweeps across the String; the right is open, still.
        s.play(11, |i| {
            vec![
                left(0.5, 0.3 + 0.04 * i as f32, true),
                right(0.9, 0.9, false),
            ]
        });
        assert_eq!(s.engine.stretch(), None);
        assert!(s.plucks.is_empty(), "{:?}", s.plucks);
    }

    #[test]
    fn s2_the_first_hand_to_let_go_freezes_the_stretch() {
        let mut s = Script::new();
        s.play(10, |_| vec![left(0.2, 0.5, true), right(0.8, 0.5, true)]);
        s.play(3, |_| vec![left(0.2, 0.5, false), right(0.8, 0.5, true)]);
        assert_eq!(s.engine.strings().len(), 1);
        assert_eq!(s.engine.stretch(), None);
    }

    #[test]
    fn s3_a_stretch_under_8_percent_of_screen_width_is_discarded() {
        let mut s = Script::new();
        s.play(10, |_| vec![left(0.47, 0.5, true), right(0.52, 0.5, true)]);
        s.play(10, |_| {
            vec![left(0.47, 0.5, false), right(0.52, 0.5, false)]
        });
        assert!(s.engine.strings().is_empty());
    }

    #[test]
    fn s4_a_stretch_survives_a_short_tracking_gap() {
        let mut s = Script::new();
        s.play(10, |_| vec![left(0.2, 0.5, true), right(0.8, 0.5, true)]);
        // The right Hand is out of tracking for 6 frames (100 ms).
        s.play(6, |_| vec![left(0.2, 0.5, true)]);
        assert!(s.engine.stretch().is_some());
        s.play(10, |_| vec![left(0.2, 0.5, true), right(0.8, 0.5, true)]);
        s.play(10, |_| vec![left(0.2, 0.5, false), right(0.8, 0.5, false)]);
        assert_eq!(s.engine.strings().len(), 1);
    }

    #[test]
    fn s5_down_then_back_up_plucks_twice() {
        let mut s = Script::new();
        s.build_string(0.5);
        s.swipe_down(0.5, 10);
        s.play(11, |i| vec![right(0.5, 0.7 - 0.04 * i as f32, false)]);
        assert_eq!(s.plucks.len(), 2, "{:?}", s.plucks);
    }

    #[test]
    fn s7_a_stretch_swept_across_a_string_does_not_pluck_it() {
        let mut s = Script::new();
        s.build_string(0.5);
        s.play(11, |i| {
            let y = 0.3 + 0.04 * i as f32;
            vec![left(0.3, y, true), right(0.7, y, true)]
        });
        assert!(s.plucks.is_empty(), "{:?}", s.plucks);
    }

    #[test]
    fn a_fast_swipe_plucks_by_prediction_and_harder_than_a_slow_one() {
        let mut fast = Script::new();
        fast.build_string(0.5);
        fast.swipe_down(0.5, 5); // 4.8 screen widths per second
        let mut slow = Script::new();
        slow.build_string(0.5);
        slow.swipe_down(0.5, 60); // 0.4 screen widths per second
        assert_eq!((fast.plucks.len(), slow.plucks.len()), (1, 1));
        let (fast, slow) = (fast.plucks[0], slow.plucks[0]);
        assert!(fast.predicted && !slow.predicted, "{fast:?} {slow:?}");
        assert!(
            fast.velocity > slow.velocity && fast.velocity <= 1.0,
            "{fast:?} {slow:?}"
        );
    }

    #[test]
    fn a_fast_swipe_that_stops_short_of_a_string_counts_as_a_false_pluck() {
        let mut s = Script::new();
        s.build_string(0.5);
        // 4.8 widths/s straight at the String, then stops dead at 0.46 for 300 ms.
        s.play(4, |i| vec![right(0.5, 0.3 + 0.04 * i as f32, false)]);
        s.play(18, |_| vec![right(0.5, 0.46, false)]);
        assert_eq!(s.plucks.len(), 1, "predicted: {:?}", s.plucks);
        assert!(s.plucks[0].predicted);
        assert_eq!(s.engine.false_plucks(), 1);
    }

    #[test]
    fn render_data_shows_anchors_while_pinching_and_pluckers_while_open() {
        let mut s = Script::new();
        s.play(5, |_| vec![left(0.2, 0.5, true), right(0.8, 0.5, false)]);
        let [l, r] = s.engine.anchors();
        assert!(l.is_some() && r.is_none());
        let [l, r] = s.engine.pluckers();
        assert!(l.is_none() && r.is_some_and(|p| close(p.x, 0.8) && close(p.y, 0.5)));
    }

    #[test]
    fn undo_clear_and_key_reach_the_strings_and_pitch() {
        let mut s = Script::new();
        s.build_string(0.5);
        s.build_string(0.8);
        s.engine.undo();
        assert_eq!(s.engine.strings().len(), 1);
        s.engine.set_key(Key { root: 2 });
        s.swipe_down(0.5, 10);
        // In D, 0.6 wide is B3; Fret 2 is two steps more: E4.
        assert!(close(s.plucks[0].frequency, 329.63), "{:?}", s.plucks);
        s.engine.clear();
        assert!(s.engine.strings().is_empty());
    }
}
