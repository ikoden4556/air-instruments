package com.airinstruments

/**
 * A manual exposure short enough for 60 fps (BLUEPRINT §4, deferred fix 2). Auto-exposure on the
 * Oppo's front camera stretches exposure to 30–60 ms indoors, and the frame rate drops with it
 * (16–33 fps). So once auto-exposure settles, we keep its brightness (exposure × ISO) but cap
 * the exposure and raise the ISO instead.
 */
data class ShortExposure(val exposureNs: Long, val iso: Int) {
    companion object {
        const val FRAME_NS = 1_000_000_000L / 60
        const val CAP_NS = 16_000_000L

        /** The manual setting for what auto-exposure chose, or null if it is already short enough. */
        fun from(aeExposureNs: Long, aeIso: Int, maxIso: Int): ShortExposure? {
            if (aeExposureNs <= CAP_NS) return null
            val iso = (aeIso.toLong() * aeExposureNs / CAP_NS).coerceAtMost(maxIso.toLong()).toInt()
            return ShortExposure(CAP_NS, iso)
        }
    }
}
