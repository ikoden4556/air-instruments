package com.airinstruments.spike

// PROTOTYPE: bindings to the Rust spikeaudio library (AAudio beep).
object Native {
    init { System.loadLibrary("spikeaudio") }

    @JvmStatic external fun start(bursts: Int): Int
    @JvmStatic external fun pluck(captureNs: Long)
    @JvmStatic external fun lastCaptureToSpeakerMs(): Double
    @JvmStatic external fun outputLatencyMs(nowNs: Long): Double
    @JvmStatic external fun bufferFrames(): Int
    @JvmStatic external fun beepPresentedNs(): Long
    @JvmStatic external fun info(): Int
}
