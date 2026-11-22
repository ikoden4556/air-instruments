package com.airinstruments

import kotlin.math.PI
import kotlin.math.cos
import kotlin.math.exp
import kotlin.math.hypot
import kotlin.math.sin

/** Pure geometry for the drawing (a port of web/src/scene.ts): Fret ticks, vibration, sparks. */
object Scene {
    const val FRETS = 5

    /** How long a Plucked String visibly vibrates. */
    const val VIBRATION_MS = 900f

    const val SPARK_LIFE = 0.5f

    /** MediaPipe's hand skeleton: pairs of landmark indices. */
    val HAND_CONNECTIONS = listOf(
        0 to 1, 1 to 2, 2 to 3, 3 to 4,
        0 to 5, 5 to 6, 6 to 7, 7 to 8,
        9 to 10, 10 to 11, 11 to 12,
        13 to 14, 14 to 15, 15 to 16,
        0 to 17, 17 to 18, 18 to 19, 19 to 20,
        5 to 9, 9 to 13, 13 to 17,
    )

    /** Where the Fret boundaries sit along a String from `a` to `b` (the inner ticks). */
    fun fretTicks(a: Point, b: Point, frets: Int = FRETS): List<Point> =
        (1 until frets).map { lerp(a, b, it.toFloat() / frets) }

    /**
     * Sideways displacement of a Plucked String's middle, `ageMs` after its Pluck: a fast
     * wobble that fades out, 0 at the Pluck and after it settles.
     */
    fun vibration(ageMs: Float, velocity: Float, maxPixels: Float): Float {
        if (ageMs < 0 || ageMs >= VIBRATION_MS) return 0f
        val fade = exp(-4 * ageMs / VIBRATION_MS) * (1 - ageMs / VIBRATION_MS)
        return maxPixels * velocity * fade * sin(2 * PI.toFloat() * ageMs / 45f)
    }

    /** Points along a String bowed sideways by `offset` at its middle (ends stay put). */
    fun bowedString(a: Point, b: Point, offset: Float, pieces: Int = 16): List<Point> {
        val len = hypot(b.x - a.x, b.y - a.y).takeIf { it > 0 } ?: 1f
        val nx = -(b.y - a.y) / len
        val ny = (b.x - a.x) / len
        return (0..pieces).map {
            val t = it.toFloat() / pieces
            val bow = offset * sin(PI.toFloat() * t)
            val p = lerp(a, b, t)
            Point(p.x + nx * bow, p.y + ny * bow)
        }
    }

    /** The point on segment a–b nearest to p. */
    fun nearestOnSegment(a: Point, b: Point, p: Point): Point {
        val dx = b.x - a.x
        val dy = b.y - a.y
        val lenSq = dx * dx + dy * dy
        val t = if (lenSq > 0) (((p.x - a.x) * dx + (p.y - a.y) * dy) / lenSq).coerceIn(0f, 1f) else 0f
        return lerp(a, b, t)
    }

    data class Spark(val x: Float, val y: Float, val vx: Float, val vy: Float, val life: Float)

    /** A burst of sparks at `at`, flying out in every direction at up to `speed` pixels/s. */
    fun burst(at: Point, count: Int, speed: Float, random: () -> Float = { Math.random().toFloat() }): List<Spark> =
        List(count) {
            val angle = random() * 2 * PI.toFloat()
            val s = speed * (0.4f + 0.6f * random())
            Spark(at.x, at.y, cos(angle) * s, sin(angle) * s, SPARK_LIFE)
        }

    /** Moves the sparks `dt` seconds on, slowing them, and drops the ones that burned out. */
    fun stepSparks(sparks: List<Spark>, dt: Float): List<Spark> {
        val drag = exp(-4 * dt)
        return sparks
            .map { Spark(it.x + it.vx * dt, it.y + it.vy * dt, it.vx * drag, it.vy * drag, it.life - dt) }
            .filter { it.life > 0 }
    }

    private fun lerp(a: Point, b: Point, t: Float) = Point(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
