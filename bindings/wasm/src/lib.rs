//! WebAssembly bindings: the Engine and the Synth as plain exported functions
//! over flat buffers in wasm memory. No imports, so the same module can be
//! instantiated on the main thread (Engine) and in an AudioWorklet (Synth).
//!
//! Every pointer returned is an offset into wasm memory; the JS side views it
//! with a typed array, re-created after each call in case memory grew.

use air_core::engine::Engine;
use air_core::frame::{Frame, Hand, Handedness, LANDMARKS_PER_HAND, Landmark};
use air_core::pitch::Key;
use air_core::synth::Synth;

/// Floats per Hand in the input buffer: handedness (0 left, 1 right), then x, y per landmark.
pub const HAND_FLOATS: usize = 1 + 2 * LANDMARKS_PER_HAND;
/// Numbers per Pluck in the output buffer: String, frequency, Velocity, time (ms), predicted (0/1).
pub const PLUCK_NUMBERS: usize = 5;
/// Render buffer header: Stretch (4), Anchors L/R (4), Pluckers L/R (4), String count (1);
/// then 4 per String (left x, y, right x, y). Missing points are NaN.
pub const RENDER_HEADER: usize = 13;

pub struct WebEngine {
    engine: Engine,
    hands: [f32; 2 * HAND_FLOATS],
    plucks: Vec<f64>,
    render: Vec<f32>,
}

#[unsafe(no_mangle)]
pub extern "C" fn engine_new(aspect: f32, key_root: u32) -> *mut WebEngine {
    Box::into_raw(Box::new(WebEngine {
        engine: Engine::new(aspect, key(key_root)),
        hands: [0.0; 2 * HAND_FLOATS],
        plucks: Vec::new(),
        render: Vec::new(),
    }))
}

/// The input buffer for the next frame: up to 2 Hands of `HAND_FLOATS` each.
///
/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_hands(e: *mut WebEngine) -> *mut f32 {
    unsafe { (*e).hands.as_mut_ptr() }
}

/// Feeds the frame in the input buffer (its first `hand_count` Hands), taken at
/// `timestamp_ms`; returns how many Plucks are in `engine_plucks`.
///
/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_update(
    e: *mut WebEngine,
    timestamp_ms: f64,
    hand_count: u32,
) -> u32 {
    let e = unsafe { &mut *e };
    let hands = e
        .hands
        .as_chunks::<HAND_FLOATS>()
        .0
        .iter()
        .take(hand_count.min(2) as usize)
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
        timestamp_ns: (timestamp_ms.max(0.0) * 1e6) as u64,
        hands,
    };
    let plucks = e.engine.update(&frame);
    e.plucks.clear();
    for p in &plucks {
        e.plucks.extend([
            p.string as f64,
            p.frequency as f64,
            p.velocity as f64,
            p.timestamp_ns as f64 / 1e6,
            p.predicted as u8 as f64,
        ]);
    }
    plucks.len() as u32
}

/// The last update's Plucks, `PLUCK_NUMBERS` each.
///
/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_plucks(e: *mut WebEngine) -> *const f64 {
    unsafe { (*e).plucks.as_ptr() }
}

/// Fills and returns the render buffer (layout: `RENDER_HEADER`).
///
/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_render(e: *mut WebEngine) -> *const f32 {
    let e = unsafe { &mut *e };
    let point = |p: Option<Landmark>| p.map_or([f32::NAN; 2], |p| [p.x, p.y]);
    let r = &mut e.render;
    r.clear();
    let stretch = e.engine.stretch();
    r.extend(point(stretch.map(|s| s.left)));
    r.extend(point(stretch.map(|s| s.right)));
    for p in e.engine.anchors().into_iter().chain(e.engine.pluckers()) {
        r.extend(point(p));
    }
    let strings = e.engine.strings();
    r.push(strings.len() as f32);
    for s in strings {
        r.extend([s.left.x, s.left.y, s.right.x, s.right.y]);
    }
    r.as_ptr()
}

/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_undo(e: *mut WebEngine) {
    unsafe { (*e).engine.undo() }
}

/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_clear(e: *mut WebEngine) {
    unsafe { (*e).engine.clear() }
}

/// # Safety
/// `e` must come from `engine_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn engine_set_key(e: *mut WebEngine, key_root: u32) {
    unsafe { (*e).engine.set_key(key(key_root)) }
}

fn key(root: u32) -> Key {
    Key {
        root: (root % 12) as u8,
    }
}

/// The most frames one `synth_render` call can fill.
pub const MAX_RENDER_FRAMES: usize = 1024;

pub struct WebSynth {
    synth: Synth,
    out: Vec<f32>,
}

#[unsafe(no_mangle)]
pub extern "C" fn synth_new(sample_rate: f32) -> *mut WebSynth {
    Box::into_raw(Box::new(WebSynth {
        synth: Synth::new(sample_rate),
        out: vec![0.0; MAX_RENDER_FRAMES],
    }))
}

/// # Safety
/// `s` must come from `synth_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_pluck(s: *mut WebSynth, frequency: f32, velocity: f32) {
    unsafe { (*s).synth.pluck(frequency, velocity) }
}

/// Renders the next `frames` (at most `MAX_RENDER_FRAMES`) mono samples; returns them.
///
/// # Safety
/// `s` must come from `synth_new`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn synth_render(s: *mut WebSynth, frames: u32) -> *const f32 {
    let s = unsafe { &mut *s };
    let n = (frames as usize).min(MAX_RENDER_FRAMES);
    s.synth.render(&mut s.out[..n]);
    s.out.as_ptr()
}

#[cfg(test)]
mod tests {
    use super::*;
    use air_core::engine::PluckEvent;

    const FRAME_MS: f64 = 1000.0 / 60.0;

    /// A Hand 0.1 wide with its index fingertip at (x, y), Pinching or open.
    fn hand(handedness: Handedness, x: f32, y: f32, pinching: bool) -> Hand {
        let mut landmarks = [Landmark { x, y: y + 0.15 }; LANDMARKS_PER_HAND];
        landmarks[0] = Landmark { x, y: y + 0.2 };
        landmarks[9] = Landmark { x, y: y + 0.1 };
        landmarks[8] = Landmark { x, y };
        let dx = if pinching { 0.0 } else { 0.08 };
        landmarks[4] = Landmark { x: x + dx, y };
        Hand {
            handedness,
            landmarks,
        }
    }

    /// Build a String between (0.2, 0.5) and (0.8, 0.5), then swipe down through its middle.
    fn script() -> Vec<Vec<Hand>> {
        use Handedness::{Left, Right};
        let mut frames = Vec::new();
        for _ in 0..10 {
            frames.push(vec![
                hand(Left, 0.2, 0.5, true),
                hand(Right, 0.8, 0.5, true),
            ]);
        }
        for _ in 0..10 {
            frames.push(vec![
                hand(Left, 0.2, 0.5, false),
                hand(Right, 0.8, 0.5, false),
            ]);
        }
        for _ in 0..5 {
            frames.push(vec![]);
        }
        for i in 0..=10 {
            frames.push(vec![hand(Right, 0.5, 0.3 + 0.04 * i as f32, false)]);
        }
        frames
    }

    /// Writes `hands` into the wrapper's input buffer and updates it.
    fn feed(e: *mut WebEngine, ms: f64, hands: &[Hand]) -> Vec<f64> {
        unsafe {
            let buf = std::slice::from_raw_parts_mut(engine_hands(e), 2 * HAND_FLOATS);
            for (h, chunk) in hands
                .iter()
                .zip(buf.as_chunks_mut::<HAND_FLOATS>().0.iter_mut())
            {
                chunk[0] = match h.handedness {
                    Handedness::Left => 0.0,
                    Handedness::Right => 1.0,
                };
                for (i, l) in h.landmarks.iter().enumerate() {
                    chunk[1 + 2 * i] = l.x;
                    chunk[2 + 2 * i] = l.y;
                }
            }
            let n = engine_update(e, ms, hands.len() as u32) as usize;
            std::slice::from_raw_parts(engine_plucks(e), n * PLUCK_NUMBERS).to_vec()
        }
    }

    fn flat(p: &PluckEvent) -> [f64; PLUCK_NUMBERS] {
        [
            p.string as f64,
            p.frequency as f64,
            p.velocity as f64,
            p.timestamp_ns as f64 / 1e6,
            p.predicted as u8 as f64,
        ]
    }

    #[test]
    fn scripted_frames_through_the_wrapper_give_the_engines_plucks() {
        let wrapped = engine_new(1.0, 2);
        let mut direct = Engine::new(1.0, Key { root: 2 });
        let (mut got, mut want) = (Vec::new(), Vec::new());
        for (i, hands) in script().into_iter().enumerate() {
            let ms = i as f64 * FRAME_MS;
            got.extend(feed(wrapped, ms, &hands));
            let frame = Frame {
                timestamp_ns: (ms * 1e6) as u64,
                hands,
            };
            want.extend(direct.update(&frame).iter().flat_map(flat));
        }
        assert_eq!(
            want.len(),
            PLUCK_NUMBERS,
            "the script should Pluck once: {want:?}"
        );
        assert_eq!(got, want);
    }

    fn render(e: *mut WebEngine) -> Vec<f32> {
        unsafe {
            let ptr = engine_render(e);
            let count = *ptr.add(RENDER_HEADER - 1) as usize;
            std::slice::from_raw_parts(ptr, RENDER_HEADER + 4 * count).to_vec()
        }
    }

    #[test]
    fn render_data_shows_the_stretch_anchors_pluckers_and_strings() {
        use Handedness::{Left, Right};
        let e = engine_new(1.0, 0);
        let both = [hand(Left, 0.2, 0.5, true), hand(Right, 0.8, 0.5, true)];
        for i in 0..5 {
            feed(e, i as f64 * FRAME_MS, &both);
        }
        let r = render(e);
        assert_eq!(&r[0..4], &[0.2, 0.5, 0.8, 0.5], "Stretch");
        assert_eq!(&r[4..8], &[0.2, 0.5, 0.8, 0.5], "Anchors");
        assert!(
            r[8..12].iter().all(|v| v.is_nan()),
            "no Pluckers while Pinching"
        );
        assert_eq!(r[12], 0.0);

        let open = [hand(Left, 0.2, 0.5, false), hand(Right, 0.8, 0.5, false)];
        for i in 5..15 {
            feed(e, i as f64 * FRAME_MS, &open);
        }
        let r = render(e);
        assert!(r[0..8].iter().all(|v| v.is_nan()), "no Stretch, no Anchors");
        assert_eq!(r[12], 1.0);
        // Opening moves the thumb a little before the Pinch ends, so the ends shift slightly.
        let ends = [0.2, 0.5, 0.8, 0.5];
        assert!(
            r[13..17]
                .iter()
                .zip(ends)
                .all(|(a, b)| (a - b).abs() < 0.03),
            "{r:?}"
        );

        unsafe { engine_undo(e) };
        assert_eq!(render(e)[12], 0.0);
    }

    #[test]
    fn the_wrapped_synth_renders_what_the_synth_renders() {
        let s = synth_new(48_000.0);
        let mut direct = Synth::new(48_000.0);
        unsafe { synth_pluck(s, 220.0, 0.8) };
        direct.pluck(220.0, 0.8);
        let mut want = vec![0.0; 128];
        for _ in 0..4 {
            direct.render(&mut want);
            let got = unsafe { std::slice::from_raw_parts(synth_render(s, 128), 128) };
            assert_eq!(got, &want[..]);
        }
        assert!(want.iter().any(|v| v.abs() > 0.01));
    }
}
