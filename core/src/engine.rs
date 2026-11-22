//! The engine: one call per camera frame, from Hands to Pluck events and render data.
//! The synth is not in here: the shells feed it Pluck events on the audio thread.

use crate::filter::{LandmarkFilter, OneEuroSettings};
use crate::frame::{Frame, Handedness, Landmark};
use crate::pinch::{Anchor, PinchDetector};
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
    pub fn update(&mut self, frame: &Frame) -> Vec<PluckEvent> {
        let frame = self.filter.filter(frame);
        let mut states = [HandState::Missing; 2];
        self.anchors = [None; 2];
        self.pluckers = [None; 2];
        for side in [Handedness::Left, Handedness::Right] {
            let i = side as usize;
            let Some(hand) = frame.hands.iter().find(|h| h.handedness == side) else {
                continue;
            };
            states[i] = match self.pinch[i].update(hand) {
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
        Hand { handedness, landmarks }
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
}
