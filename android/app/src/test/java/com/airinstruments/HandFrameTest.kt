package com.airinstruments

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class HandFrameTest {
    private fun hand(x: Float, y: Float) = FloatArray(2 * HandFrame.LANDMARKS) { if (it % 2 == 0) x else y }

    @Test
    fun mirrors_x_like_the_preview_and_keeps_y() {
        val out = FloatArray(2 * HandFrame.HAND_FLOATS)
        HandFrame.write(listOf(hand(0.2f, 0.7f)), listOf("Left"), out)
        assertEquals(0.8f, out[1], 1e-6f)
        assertEquals(0.7f, out[2], 1e-6f)
    }

    @Test
    fun swaps_mediapipes_labels_to_the_players_own_hands_and_writes_at_most_two() {
        val out = FloatArray(2 * HandFrame.HAND_FLOATS)
        val n = HandFrame.write(
            listOf(hand(0.2f, 0.5f), hand(0.8f, 0.5f), hand(0.5f, 0.5f)),
            listOf("Left", "Right", "Right"),
            out,
        )
        assertEquals(2, n)
        assertEquals(1f, out[0]) // MediaPipe "Left" is the Player's right Hand
        assertEquals(0f, out[HandFrame.HAND_FLOATS])
    }

    @Test
    fun camera_timestamps_on_boottime_are_moved_to_monotonic() {
        val boot = 5_000_000_000L
        val mono = 3_000_000_000L
        assertTrue(CameraClock.isBoottime(4_990_000_000L, boot, mono))
        assertEquals(2_990_000_000L, CameraClock.toMonotonic(4_990_000_000L, true, boot, mono))
        assertFalse(CameraClock.isBoottime(2_990_000_000L, boot, mono))
        assertEquals(2_990_000_000L, CameraClock.toMonotonic(2_990_000_000L, false, boot, mono))
    }

    @Test
    fun render_data_decodes_the_cores_layout() {
        val nan = Float.NaN
        val r = floatArrayOf(
            0.2f, 0.5f, 0.8f, 0.5f, // Stretch
            0.2f, 0.5f, nan, nan, // Anchors: left only
            nan, nan, 0.6f, 0.4f, // Pluckers: right only
            1f, // one String
            0.1f, 0.3f, 0.9f, 0.35f,
        )
        val d = RenderData.parse(r, r.size)
        assertEquals(Segment(Point(0.2f, 0.5f), Point(0.8f, 0.5f)), d.stretch)
        assertEquals(Point(0.2f, 0.5f), d.anchors[0])
        assertNull(d.anchors[1])
        assertEquals(Point(0.6f, 0.4f), d.pluckers[1])
        assertEquals(listOf(Segment(Point(0.1f, 0.3f), Point(0.9f, 0.35f))), d.strings)
    }

    @Test
    fun a_4_by_3_image_covering_a_2_by_1_view_is_cropped_top_and_bottom() {
        val m = CoverMapping(640f, 480f, 2000f, 1000f)
        assertEquals(1000f, m.x(Point(0.5f, 0.5f)), 1e-3f)
        assertEquals(500f, m.y(Point(0.5f, 0.5f)), 1e-3f)
        assertEquals(-250f, m.y(Point(0f, 0f)), 1e-3f)
    }
}
