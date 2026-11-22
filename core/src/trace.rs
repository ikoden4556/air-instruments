//! Recorded hand traces from the phone, replayed through the Engine to tune Pluck
//! prediction offline (#44).
//!
//! Text, one line per frame: `timestamp_ns hand_count` then each Hand as its
//! handedness (0 Left, 1 Right) and 21 landmarks as x y. The first line is
//! `aspect <height / width>`.

use crate::engine::Engine;
use crate::frame::{Frame, Hand, Handedness, LANDMARKS_PER_HAND, Landmark};
use crate::pitch::Key;
use crate::strings::Segment;

pub struct Trace {
    /// Screen height / width, as for `Engine::new`.
    pub aspect: f32,
    pub frames: Vec<Frame>,
}

/// Reads a recorded trace; the error names the first bad line.
pub fn parse(text: &str) -> Result<Trace, String> {
    let mut lines = text.lines().enumerate().filter(|(_, l)| !l.trim().is_empty());
    let aspect = lines
        .next()
        .and_then(|(_, l)| l.strip_prefix("aspect "))
        .and_then(|a| a.trim().parse().ok())
        .ok_or("line 1: expected `aspect <number>`")?;
    let mut lines = lines.peekable();
    let mut frames = Vec::new();
    while let Some((n, line)) = lines.next() {
        match frame(line) {
            Some(f) => frames.push(f),
            // The app can stop mid-line: an unfinished last line is dropped.
            None if lines.peek().is_none() => {}
            None => return Err(format!("line {}: bad frame", n + 1)),
        }
    }
    Ok(Trace { aspect, frames })
}

/// How a lookahead did on a trace, against the real crossings (the same trace with
/// prediction off).
#[derive(Debug, Default)]
pub struct Score {
    /// Real crossings that Plucked.
    pub crossings: usize,
    /// Real crossings that got no Pluck.
    pub missed: usize,
    pub false_plucks: u32,
    /// Per matched Pluck, oldest first: real crossing time minus the time of the frame
    /// that fired it. Positive means it fired before the crossing.
    pub leads_ms: Vec<f64>,
    /// Per matched predicted Pluck: predicted crossing time minus the real one.
    pub errors_ms: Vec<f64>,
}

/// A Pluck in a replay: its String, the crossing time it gives, the frame it came on.
struct Fired {
    string: Segment,
    crossing_ns: u64,
    frame_ns: u64,
    predicted: bool,
}

fn replay(trace: &Trace, lookahead_ns: u64) -> (Vec<Fired>, u32) {
    let mut engine = Engine::new(trace.aspect, Key { root: 0 });
    engine.set_lookahead(lookahead_ns);
    let mut fired = Vec::new();
    for frame in &trace.frames {
        for p in engine.update(frame) {
            fired.push(Fired {
                string: engine.strings()[p.string],
                crossing_ns: p.timestamp_ns,
                frame_ns: frame.timestamp_ns,
                predicted: p.predicted,
            });
        }
    }
    (fired, engine.false_plucks())
}

/// A Pluck this close to a real crossing of its String is that crossing's Pluck.
const MATCH_NS: u64 = 250_000_000;

pub fn score(trace: &Trace, lookahead_ns: u64) -> Score {
    let (real, _) = replay(trace, 0);
    let (mut fired, false_plucks) = replay(trace, lookahead_ns);
    let mut s = Score {
        crossings: real.len(),
        false_plucks,
        ..Score::default()
    };
    for r in &real {
        let matched = fired
            .iter()
            .position(|f| f.string == r.string && f.crossing_ns.abs_diff(r.crossing_ns) < MATCH_NS);
        let Some(i) = matched else {
            s.missed += 1;
            continue;
        };
        let f = fired.remove(i);
        let ms = |a: u64, b: u64| (a as f64 - b as f64) / 1e6;
        s.leads_ms.push(ms(r.crossing_ns, f.frame_ns));
        if f.predicted {
            s.errors_ms.push(ms(f.crossing_ns, r.crossing_ns));
        }
    }
    s
}

fn frame(line: &str) -> Option<Frame> {
    let mut words = line.split_ascii_whitespace();
    let timestamp_ns = words.next()?.parse().ok()?;
    let count: usize = words.next()?.parse().ok()?;
    let numbers: Vec<f32> = words.map(str::parse).collect::<Result<_, _>>().ok()?;
    let per_hand = 1 + 2 * LANDMARKS_PER_HAND;
    if count > 2 || numbers.len() != count * per_hand {
        return None;
    }
    let hands = numbers
        .chunks(per_hand)
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
    Some(Frame {
        timestamp_ns,
        hands,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::Handedness;

    const FRAME_NS: u64 = 16_666_667;

    /// A Hand whose index fingertip is at (x, y), thumb tip on it (Pinching) or away.
    fn hand(handedness: Handedness, x: f32, y: f32, pinching: bool) -> Hand {
        let mut landmarks = [Landmark { x, y: y + 0.15 }; LANDMARKS_PER_HAND];
        landmarks[0] = Landmark { x, y: y + 0.2 };
        landmarks[9] = Landmark { x, y: y + 0.1 };
        landmarks[8] = Landmark { x, y };
        let thumb_dx = if pinching { 0.0 } else { 0.08 };
        landmarks[4] = Landmark { x: x + thumb_dx, y };
        Hand {
            handedness,
            landmarks,
        }
    }

    /// Builds a String across y = 0.5, then the right Plucker: a fast swipe down
    /// through it, a slow one back up, and a fast approach that stops dead at 0.46.
    fn played() -> Trace {
        let mut fingers: Vec<Vec<Hand>> = Vec::new();
        let both = |pinching| {
            vec![
                hand(Handedness::Left, 0.2, 0.5, pinching),
                hand(Handedness::Right, 0.8, 0.5, pinching),
            ]
        };
        fingers.extend((0..10).map(|_| both(true)));
        fingers.extend((0..10).map(|_| both(false)));
        fingers.extend((0..5).map(|_| vec![]));
        let right = |y: f32| vec![hand(Handedness::Right, 0.5, y, false)];
        fingers.extend((0..=5).map(|i| right(0.3 + 0.08 * i as f32)));
        fingers.extend((0..10).map(|_| right(0.7)));
        fingers.extend((0..=60).map(|i| right(0.7 - 0.4 * i as f32 / 60.0)));
        fingers.extend((0..10).map(|_| right(0.3)));
        fingers.extend((0..4).map(|i| right(0.3 + 0.04 * i as f32)));
        fingers.extend((0..20).map(|_| right(0.46)));
        Trace {
            aspect: 1.0,
            frames: fingers
                .into_iter()
                .enumerate()
                .map(|(i, hands)| Frame {
                    timestamp_ns: i as u64 * FRAME_NS,
                    hands,
                })
                .collect(),
        }
    }

    #[test]
    fn the_score_counts_crossings_false_plucks_and_how_early_each_pluck_fired() {
        let s = score(&played(), crate::predict::DEFAULT_LOOKAHEAD_NS);
        assert_eq!((s.crossings, s.missed, s.false_plucks), (2, 0, 1), "{s:?}");
        assert_eq!(s.leads_ms.len(), 2, "{s:?}");
        // The fast swipe fires before its crossing; the slow one on seeing it.
        assert!(s.leads_ms[0] > 0.0 && s.leads_ms[1] <= 0.0, "{s:?}");
        assert_eq!(s.errors_ms.len(), 1, "{s:?}");
    }

    #[test]
    fn with_no_lookahead_every_pluck_fires_after_its_crossing() {
        let s = score(&played(), 0);
        assert_eq!((s.crossings, s.missed, s.false_plucks), (2, 0, 0), "{s:?}");
        assert!(s.leads_ms.iter().all(|&l| l <= 0.0), "{s:?}");
        assert!(s.errors_ms.is_empty(), "{s:?}");
    }

    #[test]
    fn a_last_line_cut_off_when_the_app_stopped_is_skipped() {
        let trace = parse("aspect 0.75\n1000 0\n2000 1 1 0.5 0.").unwrap();
        assert_eq!(trace.frames.len(), 1);
        assert!(parse("aspect 0.75\n2000 1 1 0.5\n1000 0\n").is_err());
    }

    #[test]
    fn a_recorded_trace_parses_into_frames() {
        let right: Vec<String> = (0..42).map(|i| format!("{}", i as f32 / 100.0)).collect();
        let text = format!("aspect 0.75\n1000 0\n2000 1 1 {}\n", right.join(" "));
        let trace = parse(&text).unwrap();
        assert_eq!(trace.aspect, 0.75);
        assert_eq!(trace.frames.len(), 2);
        assert_eq!(trace.frames[0].timestamp_ns, 1000);
        assert!(trace.frames[0].hands.is_empty());
        let hand = &trace.frames[1].hands[0];
        assert_eq!(hand.handedness, Handedness::Right);
        assert_eq!((hand.landmarks[8].x, hand.landmarks[8].y), (0.16, 0.17));
    }
}
