//! Android bindings: the Engine over JNI (class `com.airinstruments.Native`), and an AAudio
//! stream running the Synth. Plucks go from the Engine straight to the audio callback
//! through a lock-free queue, never through Kotlin.

pub mod flat;
pub mod queue;

#[cfg(target_os = "android")]
mod audio;

use flat::{FlatEngine, HAND_FLOATS, PLUCK_NUMBERS, RENDER_MAX};
use jni_sys::{
    JNIEnv, jboolean, jclass, jdouble, jdoubleArray, jfloat, jfloatArray, jint, jlong, jlongArray,
};
use queue::PluckQueue;

/// Plucks from the Engine (camera thread) to the audio callback.
pub static QUEUE: PluckQueue = PluckQueue::new();

/// The most Plucks one frame reports back to Kotlin (more still reach the synth).
const MAX_PLUCKS: usize = 16;

unsafe fn array_len(env: *mut JNIEnv, array: jni_sys::jarray) -> usize {
    unsafe { ((**env).GetArrayLength.unwrap())(env, array).max(0) as usize }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_Native_engineNew(
    _env: *mut JNIEnv,
    _: jclass,
    aspect: jfloat,
    key_root: jint,
) -> jlong {
    Box::into_raw(Box::new(FlatEngine::new(aspect, key_root))) as jlong
}

/// # Safety
/// `e` must come from `engineNew` and not be used after this.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineFree(
    _env: *mut JNIEnv,
    _: jclass,
    e: jlong,
) {
    drop(unsafe { Box::from_raw(e as *mut FlatEngine) });
}

/// Feeds one frame; `hands` holds `count` Hands of `HAND_FLOATS`. Writes the Plucks into
/// `out` (`PLUCK_NUMBERS` each, as many as fit) and returns how many there were.
///
/// # Safety
/// `e` must come from `engineNew`; `env` and the arrays come from the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineUpdate(
    env: *mut JNIEnv,
    _: jclass,
    e: jlong,
    timestamp_ns: jlong,
    hands: jfloatArray,
    count: jint,
    out: jdoubleArray,
) -> jint {
    let e = unsafe { &mut *(e as *mut FlatEngine) };
    let mut flat_hands = [0.0f32; 2 * HAND_FLOATS];
    let mut plucks = [0.0f64; PLUCK_NUMBERS * MAX_PLUCKS];
    unsafe {
        let n = array_len(env, hands).min(flat_hands.len());
        ((**env).GetFloatArrayRegion.unwrap())(env, hands, 0, n as i32, flat_hands.as_mut_ptr());
        let found = e.update(
            timestamp_ns,
            &flat_hands[..n],
            count.max(0) as usize,
            &QUEUE,
            &mut plucks,
        );
        let fit = array_len(env, out).min(found.min(MAX_PLUCKS) * PLUCK_NUMBERS);
        ((**env).SetDoubleArrayRegion.unwrap())(env, out, 0, fit as i32, plucks.as_ptr());
        found as jint
    }
}

/// Writes the render data (layout in `flat::RENDER_HEADER`) into `out`; returns its length.
///
/// # Safety
/// `e` must come from `engineNew`; `env` and `out` come from the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineRender(
    env: *mut JNIEnv,
    _: jclass,
    e: jlong,
    out: jfloatArray,
) -> jint {
    let e = unsafe { &*(e as *const FlatEngine) };
    let mut buf = [0.0f32; RENDER_MAX];
    let n = e.render(&mut buf);
    unsafe {
        let fit = array_len(env, out).min(n);
        ((**env).SetFloatArrayRegion.unwrap())(env, out, 0, fit as i32, buf.as_ptr());
    }
    n as jint
}

/// # Safety
/// `e` must come from `engineNew`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineUndo(
    _: *mut JNIEnv,
    _: jclass,
    e: jlong,
) {
    unsafe { (*(e as *mut FlatEngine)).engine.undo() }
}

/// # Safety
/// `e` must come from `engineNew`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineClear(
    _: *mut JNIEnv,
    _: jclass,
    e: jlong,
) {
    unsafe { (*(e as *mut FlatEngine)).engine.clear() }
}

/// # Safety
/// `e` must come from `engineNew`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineSetKey(
    _: *mut JNIEnv,
    _: jclass,
    e: jlong,
    key_root: jint,
) {
    unsafe {
        (*(e as *mut FlatEngine))
            .engine
            .set_key(flat::key(key_root))
    }
}

/// Predicted Plucks whose String was never crossed in time, so far.
///
/// # Safety
/// `e` must come from `engineNew`.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_engineFalsePlucks(
    _: *mut JNIEnv,
    _: jclass,
    e: jlong,
) -> jint {
    unsafe { (*(e as *const FlatEngine)).engine.false_plucks() as jint }
}

/// Starts the audio stream (once). Returns packed info (bit 0 exclusive, bit 1 low latency,
/// bits 8.. frames per burst) or a negative error; -100 off Android.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_Native_audioStart(
    _: *mut JNIEnv,
    _: jclass,
) -> jint {
    #[cfg(target_os = "android")]
    return audio::start();
    #[cfg(not(target_os = "android"))]
    -100
}

/// How far ahead a sample written now reaches the speaker, ms; -1 if unknown.
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_airinstruments_Native_audioOutputLatencyMs(
    _: *mut JNIEnv,
    _: jclass,
    _now_ns: jlong,
) -> jdouble {
    #[cfg(target_os = "android")]
    return audio::output_latency_ms(_now_ns);
    #[cfg(not(target_os = "android"))]
    -1.0
}

/// Writes [crossing ns, speaker ns] of the last Pluck the synth started into `out`;
/// returns false if not known yet.
///
/// # Safety
/// `env` and `out` come from the JVM.
#[unsafe(no_mangle)]
pub unsafe extern "system" fn Java_com_airinstruments_Native_audioLastPluck(
    _env: *mut JNIEnv,
    _: jclass,
    _out: jlongArray,
) -> jboolean {
    #[cfg(target_os = "android")]
    if let Some((crossing, presented)) = audio::last_pluck() {
        unsafe {
            ((**_env).SetLongArrayRegion.unwrap())(_env, _out, 0, 2, [crossing, presented].as_ptr())
        };
        return 1;
    }
    0
}
