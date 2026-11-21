package com.airinstruments.spike

// PROTOTYPE: bindings to the Rust spikeaudio library (AAudio beep).
object Native {
    init { System.loadLibrary("spikeaudio") }

    @JvmStatic external fun start(): Int
    @JvmStatic external fun pluck(captureNs: Long)
    @JvmStatic external fun lastCaptureToSpeakerMs(): Double
    @JvmStatic external fun info(): Int
}
