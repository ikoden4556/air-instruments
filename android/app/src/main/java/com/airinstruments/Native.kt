package com.airinstruments

/** The Rust core over JNI (bindings/android, libair.so). Layouts are in [HandFrame] and [RenderData]. */
object Native {
    init {
        System.loadLibrary("air")
    }

    /** `aspect` is the camera image's height / width; `keyRoot` 0..11 (C = 0). */
    @JvmStatic external fun engineNew(aspect: Float, keyRoot: Int): Long
    @JvmStatic external fun engineFree(engine: Long)

    /**
     * Feeds one frame: `count` Hands of [HandFrame.HAND_FLOATS] in `hands`, captured at
     * `timestampNs` (CLOCK_MONOTONIC). Its Plucks also go straight to the audio stream.
     * Writes them into `out` ([PLUCK_NUMBERS] each) and returns how many there were.
     */
    @JvmStatic external fun engineUpdate(engine: Long, timestampNs: Long, hands: FloatArray, count: Int, out: DoubleArray): Int

    /** Writes the render data into `out` (at least [RenderData.MAX] long); returns its length. */
    @JvmStatic external fun engineRender(engine: Long, out: FloatArray): Int
    @JvmStatic external fun engineUndo(engine: Long)
    @JvmStatic external fun engineClear(engine: Long)
    @JvmStatic external fun engineSetKey(engine: Long, keyRoot: Int)

    /** Starts the AAudio stream; packed info (bit 0 exclusive, bit 1 low latency, bits 8.. burst) or < 0. */
    @JvmStatic external fun audioStart(): Int

    /** How far ahead a sample written now reaches the speaker, ms; -1 if unknown. */
    @JvmStatic external fun audioOutputLatencyMs(nowNs: Long): Double

    /** [crossing ns, speaker ns] of the last Pluck the synth started; false if none yet. */
    @JvmStatic external fun audioLastPluck(out: LongArray): Boolean

    /** Numbers per Pluck: String, frequency, Velocity, crossing ns, predicted (0/1). */
    const val PLUCK_NUMBERS = 5
    const val MAX_PLUCKS = 16
}
