package com.airinstruments

import kotlin.math.max

data class Point(val x: Float, val y: Float)

data class Segment(val a: Point, val b: Point)

/** What to draw this frame, decoded from the core's render buffer (same layout as the web app). */
data class RenderData(
    val stretch: Segment?,
    /** [left, right] while Pinching. */
    val anchors: List<Point?>,
    /** [left, right] while open. */
    val pluckers: List<Point?>,
    val strings: List<Segment>,
) {
    companion object {
        const val HEADER = 13
        const val MAX = HEADER + 4 * 12
        val EMPTY = RenderData(null, listOf(null, null), listOf(null, null), emptyList())

        fun parse(r: FloatArray, length: Int): RenderData {
            fun point(i: Int) = if (r[i].isNaN()) null else Point(r[i], r[i + 1])
            fun segment(i: Int): Segment? {
                val a = point(i) ?: return null
                val b = point(i + 2) ?: return null
                return Segment(a, b)
            }
            val count = minOf(r[HEADER - 1].toInt(), (length - HEADER) / 4)
            return RenderData(
                stretch = segment(0),
                anchors = listOf(point(4), point(6)),
                pluckers = listOf(point(8), point(10)),
                strings = (0 until count).mapNotNull { segment(HEADER + 4 * it) },
            )
        }
    }
}

/**
 * Maps screen-space points (0..1 of the camera image) to view pixels when the preview fills
 * the view like `FILL_CENTER` (scaled up, centred, edges cropped).
 */
class CoverMapping(imageWidth: Float, imageHeight: Float, viewWidth: Float, viewHeight: Float) {
    private val scale = max(viewWidth / imageWidth, viewHeight / imageHeight)
    private val offsetX = (viewWidth - imageWidth * scale) / 2
    private val offsetY = (viewHeight - imageHeight * scale) / 2
    private val w = imageWidth * scale
    private val h = imageHeight * scale

    fun x(p: Point) = offsetX + p.x * w
    fun y(p: Point) = offsetY + p.y * h
}
