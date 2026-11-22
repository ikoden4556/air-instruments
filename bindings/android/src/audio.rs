//! The AAudio output stream: the Synth runs in its data callback and takes Plucks from the
//! queue. Low-latency mode, exclusive requested (the Oppo grants shared), 1-burst buffer.

use std::sync::atomic::{AtomicI64, AtomicPtr, Ordering::*};

use air_core::synth::Synth;
use ndk::audio::*;

use crate::QUEUE;

/// Requested so the Synth can be made before the stream opens (AAudio converts if needed).
const SAMPLE_RATE: i32 = 48_000;

static STREAM: AtomicPtr<AudioStream> = AtomicPtr::new(std::ptr::null_mut());
/// The last Pluck the callback started: its crossing time, and the stream frame it began at.
static LAST_CROSSING_NS: AtomicI64 = AtomicI64::new(-1);
static LAST_FIRST_FRAME: AtomicI64 = AtomicI64::new(-1);

fn stream() -> Option<&'static AudioStream> {
    unsafe { STREAM.load(Acquire).as_ref() }
}

/// Opens and starts the stream once. Returns `info()`, or a negative error.
pub fn start() -> i32 {
    if stream().is_some() {
        return info();
    }
    let mut synth = Synth::new(SAMPLE_RATE as f32);
    let callback: AudioStreamDataCallback = Box::new(move |s, data, frames| {
        let out = unsafe { std::slice::from_raw_parts_mut(data as *mut f32, frames as usize) };
        while let Some(p) = QUEUE.pop() {
            synth.pluck(p.frequency, p.velocity);
            LAST_CROSSING_NS.store(p.crossing_ns, Release);
            LAST_FIRST_FRAME.store(s.frames_written(), Release);
        }
        synth.render(out);
        AudioCallbackResult::Continue
    });
    let opened = AudioStreamBuilder::new().and_then(|b| {
        b.direction(AudioDirection::Output)
            .performance_mode(AudioPerformanceMode::LowLatency)
            .sharing_mode(AudioSharingMode::Exclusive)
            .format(AudioFormat::PCM_Float)
            .channel_count(1)
            .sample_rate(SAMPLE_RATE)
            .usage(AudioUsage::Game)
            .content_type(AudioContentType::Music)
            .data_callback(callback)
            .open_stream()
    });
    let Ok(s) = opened else { return -1 };
    let _ = s.set_buffer_size_in_frames(s.frames_per_burst());
    if s.request_start().is_err() {
        return -2;
    }
    // Lives for the whole app.
    STREAM.store(Box::into_raw(Box::new(s)), Release);
    info()
}

/// Packed: bit 0 exclusive granted, bit 1 low latency granted, bits 8.. frames per burst.
pub fn info() -> i32 {
    let Some(s) = stream() else { return -1 };
    (s.sharing_mode() == AudioSharingMode::Exclusive) as i32
        | ((s.performance_mode() == AudioPerformanceMode::LowLatency) as i32) << 1
        | s.frames_per_burst() << 8
}

/// When stream frame `frame` reaches the speaker, CLOCK_MONOTONIC ns.
fn presented_ns(s: &AudioStream, frame: i64) -> Option<i64> {
    let ts = s.timestamp(Clockid::Monotonic).ok()?;
    Some(ts.time_nanoseconds + (frame - ts.frame_position) * 1_000_000_000 / s.sample_rate() as i64)
}

/// How far in the future a frame written now reaches the speaker, ms; -1 if unknown.
pub fn output_latency_ms(now_ns: i64) -> f64 {
    stream()
        .and_then(|s| presented_ns(s, s.frames_written()))
        .map_or(-1.0, |p| (p - now_ns) as f64 / 1e6)
}

/// The last Pluck's crossing time and when its first frame reached the speaker (ns), if known.
pub fn last_pluck() -> Option<(i64, i64)> {
    let first = LAST_FIRST_FRAME.load(Acquire);
    if first < 0 {
        return None;
    }
    let presented = presented_ns(stream()?, first)?;
    Some((LAST_CROSSING_NS.load(Acquire), presented))
}
