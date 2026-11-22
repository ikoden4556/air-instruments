package com.airinstruments

import kotlin.math.abs

/** From MediaPipe's Hands to the core's input: screen space, mirrored like the front-camera preview. */
object HandFrame {
    const val LANDMARKS = 21

    /** Floats per Hand: handedness (0 left, 1 right), then x, y per landmark. */
    const val HAND_FLOATS = 1 + 2 * LANDMARKS

    /**
     * `points` holds each Hand's landmarks as x0, y0, x1, y1 … in the upright camera image;
     * `labels` MediaPipe's handedness. MediaPipe sees the unmirrored image but labels it as if
     * mirrored, so each label names the Player's other Hand: mirroring x and swapping the label
     * gives the Player's own left and right. Writes up to 2 Hands into `out`; returns how many.
     */
    fun write(points: List<FloatArray>, labels: List<String>, out: FloatArray): Int {
        val count = minOf(points.size, 2)
        for (h in 0 until count) {
            val at = h * HAND_FLOATS
            out[at] = if (labels.getOrNull(h) == "Left") 1f else 0f
            val p = points[h]
            for (i in 0 until LANDMARKS) {
                out[at + 1 + 2 * i] = 1f - p[2 * i]
                out[at + 2 + 2 * i] = p[2 * i + 1]
            }
        }
        return count
    }
}

/** Camera timestamps are on BOOTTIME or MONOTONIC depending on the phone; the core uses MONOTONIC. */
object CameraClock {
    /** Whether `timestampNs` is on BOOTTIME: it is nearer to `bootNowNs` than to `monoNowNs`. */
    fun isBoottime(timestampNs: Long, bootNowNs: Long, monoNowNs: Long): Boolean =
        abs(timestampNs - bootNowNs) < abs(timestampNs - monoNowNs)

    fun toMonotonic(timestampNs: Long, boottime: Boolean, bootNowNs: Long, monoNowNs: Long): Long =
        if (boottime) timestampNs - (bootNowNs - monoNowNs) else timestampNs
}
