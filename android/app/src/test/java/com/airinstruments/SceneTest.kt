package com.airinstruments

import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test
import kotlin.math.abs
import kotlin.math.hypot

/** The same checks as web/src/scene.test.ts, so both apps draw alike. */
class SceneTest {
    @Test
    fun a_string_is_split_into_5_equal_frets_by_4_inner_ticks() {
        val ticks = Scene.fretTicks(Point(0f, 0f), Point(100f, 50f))
        val want = listOf(Point(20f, 10f), Point(40f, 20f), Point(60f, 30f), Point(80f, 40f))
        assertEquals(want.size, ticks.size)
        for ((got, w) in ticks.zip(want)) {
            assertEquals(w.x, got.x, 1e-4f)
            assertEquals(w.y, got.y, 1e-4f)
        }
    }

    @Test
    fun vibration_starts_still_wobbles_and_settles() {
        assertEquals(0f, Scene.vibration(0f, 1f, 10f), 1e-4f)
        val early = (0 until 50).maxOf { abs(Scene.vibration(it.toFloat(), 1f, 10f)) }
        assertTrue(early > 5f)
        assertEquals(0f, Scene.vibration(Scene.VIBRATION_MS, 1f, 10f))
        assertEquals(0f, Scene.vibration(-5f, 1f, 10f))
    }

    @Test
    fun a_softer_pluck_wobbles_less_and_never_past_max() {
        var t = 0f
        while (t < Scene.VIBRATION_MS) {
            assertTrue(abs(Scene.vibration(t, 0.3f, 10f)) <= abs(Scene.vibration(t, 1f, 10f)) + 1e-6f)
            assertTrue(abs(Scene.vibration(t, 1f, 10f)) <= 10f)
            t += 7f
        }
    }

    @Test
    fun a_bowed_string_keeps_its_ends_and_bows_its_middle() {
        val pts = Scene.bowedString(Point(0f, 0f), Point(100f, 0f), 8f, 4)
        assertEquals(Point(0f, 0f), pts[0])
        assertEquals(100f, pts[4].x, 1e-4f)
        assertEquals(0f, pts[4].y, 1e-4f)
        assertEquals(50f, pts[2].x, 1e-4f)
        assertEquals(8f, abs(pts[2].y), 1e-4f)
    }

    @Test
    fun nearest_point_projects_and_clamps() {
        val (a, b) = Point(0f, 0f) to Point(10f, 0f)
        assertEquals(Point(4f, 0f), Scene.nearestOnSegment(a, b, Point(4f, 3f)))
        assertEquals(Point(0f, 0f), Scene.nearestOnSegment(a, b, Point(-5f, 1f)))
        assertEquals(Point(10f, 0f), Scene.nearestOnSegment(a, b, Point(50f, 1f)))
    }

    @Test
    fun sparks_fly_out_slow_down_and_burn_out() {
        var sparks = Scene.burst(Point(50f, 50f), 10, 100f) { 0.5f }
        assertEquals(10, sparks.size)
        val moved = Scene.stepSparks(sparks, 0.1f)
        assertTrue(hypot(moved[0].x - 50f, moved[0].y - 50f) > 0f)
        assertTrue(hypot(moved[0].vx, moved[0].vy) < hypot(sparks[0].vx, sparks[0].vy))
        var t = 0f
        while (t < Scene.SPARK_LIFE + 0.05f) {
            sparks = Scene.stepSparks(sparks, 0.05f)
            t += 0.05f
        }
        assertTrue(sparks.isEmpty())
    }

    @Test
    fun the_hand_skeleton_joins_all_21_landmarks() {
        val joined = Scene.HAND_CONNECTIONS.flatMap { listOf(it.first, it.second) }.toSet()
        assertEquals((0..20).toSet(), joined)
    }
}
