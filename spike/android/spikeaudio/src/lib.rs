//! PROTOTYPE: throwaway phase-2 latency spike. A beep over AAudio, triggered from Kotlin via JNI.
//! Only primitive arguments cross JNI, so no JNIEnv access (and no jni crate) is needed.
use ndk::audio::*;
use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicPtr, Ordering::*};

type Env = *mut c_void;
type Class = *mut c_void;

static STREAM: AtomicPtr<AudioStream> = AtomicPtr::new(std::ptr::null_mut());
static PLUCK: AtomicBool = AtomicBool::new(false);
static PENDING_CAPTURE_NS: AtomicI64 = AtomicI64::new(0); // camera capture time of the frame that plucked
static BEEP_CAPTURE_NS: AtomicI64 = AtomicI64::new(0);
static BEEP_FIRST_FRAME: AtomicI64 = AtomicI64::new(-1); // stream frame index where the beep starts

fn stream() -> Option<&'static AudioStream> {
    unsafe { STREAM.load(Acquire).as_ref() }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_spike_Native_start(_: Env, _: Class, bursts: i32) -> i32 {
    if stream().is_some() {
        return 0;
    }
    let mut phase = 0f32;
    let mut env = 0f32;
    let mut step = 0f32;
    let callback: AudioStreamDataCallback = Box::new(move |s, data, frames| {
        let out = unsafe { std::slice::from_raw_parts_mut(data as *mut f32, frames as usize) };
        if PLUCK.swap(false, AcqRel) {
            BEEP_CAPTURE_NS.store(PENDING_CAPTURE_NS.load(Acquire), Release);
            BEEP_FIRST_FRAME.store(s.frames_written(), Release);
            env = 0.6;
            step = 880.0 / s.sample_rate() as f32;
        }
        for o in out.iter_mut() {
            *o = (phase * std::f32::consts::TAU).sin() * env;
            phase = (phase + step).fract();
            env *= 0.9993; // ~120 ms decay at 48 kHz
        }
        AudioCallbackResult::Continue
    });
    let opened = AudioStreamBuilder::new().and_then(|b| {
        b.direction(AudioDirection::Output)
            .performance_mode(AudioPerformanceMode::LowLatency)
            .sharing_mode(AudioSharingMode::Exclusive)
            .format(AudioFormat::PCM_Float)
            .channel_count(1)
            .usage(AudioUsage::Game)
            .content_type(AudioContentType::Sonification)
            .data_callback(callback)
            .open_stream()
    });
    let Ok(s) = opened else { return -1 };
    let _ = s.set_buffer_size_in_frames(s.frames_per_burst() * bursts);
    if s.request_start().is_err() {
        return -2;
    }
    STREAM.store(Box::into_raw(Box::new(s)), Release); // leaked on purpose: lives for the whole app
    0
}

/// `capture_ns` is the camera capture time of the frame that crossed, on CLOCK_MONOTONIC (System.nanoTime()).
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_spike_Native_pluck(_: Env, _: Class, capture_ns: i64) {
    PENDING_CAPTURE_NS.store(capture_ns, Release);
    PLUCK.store(true, Release);
}

/// Capture-to-speaker time of the last beep in ms, or -1 if not known yet.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_spike_Native_lastCaptureToSpeakerMs(_: Env, _: Class) -> f64 {
    let (Some(s), first) = (stream(), BEEP_FIRST_FRAME.load(Acquire)) else { return -1.0 };
    if first < 0 {
        return -1.0;
    }
    // AAudio timestamp = (frame index, time it was presented at the speaker); extrapolate to the beep's first frame.
    let Ok(ts) = s.timestamp(Clockid::Monotonic) else { return -1.0 };
    let presented_ns = ts.time_nanoseconds + (first - ts.frame_position) * 1_000_000_000 / s.sample_rate() as i64;
    (presented_ns - BEEP_CAPTURE_NS.load(Acquire)) as f64 / 1e6
}

/// Audio-only latency in ms: a frame written now reaches the speaker this far in the future.
/// `now_ns` is System.nanoTime() (CLOCK_MONOTONIC) at the call.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_spike_Native_outputLatencyMs(_: Env, _: Class, now_ns: i64) -> f64 {
    let Some(s) = stream() else { return -1.0 };
    let Ok(ts) = s.timestamp(Clockid::Monotonic) else { return -1.0 };
    let presented_ns = ts.time_nanoseconds + (s.frames_written() - ts.frame_position) * 1_000_000_000 / s.sample_rate() as i64;
    (presented_ns - now_ns) as f64 / 1e6
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_spike_Native_bufferFrames(_: Env, _: Class) -> i32 {
    stream().map_or(-1, |s| s.buffer_size_in_frames())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_spike_Native_info(_: Env, _: Class) -> i32 {
    // packed: bit0 exclusive granted, bit1 low-latency granted, bits 8.. burst size in frames
    let Some(s) = stream() else { return -1 };
    (s.sharing_mode() == AudioSharingMode::Exclusive) as i32
        | ((s.performance_mode() == AudioPerformanceMode::LowLatency) as i32) << 1
        | s.frames_per_burst() << 8
}
