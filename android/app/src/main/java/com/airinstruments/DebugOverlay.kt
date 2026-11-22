package com.airinstruments

import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.view.View

/** Plain drawing of the render data over the preview; the OpenGL ES drawing replaces it. */
class DebugOverlay(private val activity: MainActivity) : View(activity) {
    private val string = Paint().apply { color = Color.CYAN; strokeWidth = 8f; strokeCap = Paint.Cap.ROUND }
    private val stretch = Paint().apply { color = Color.argb(128, 255, 255, 255); strokeWidth = 4f }
    private val anchor = Paint().apply { color = Color.YELLOW }
    private val plucker = Paint().apply { color = Color.MAGENTA }

    override fun onDraw(canvas: Canvas) {
        val r = activity.scene
        val size = activity.imageSize
        val m = CoverMapping(size.width.toFloat(), size.height.toFloat(), width.toFloat(), height.toFloat())
        fun line(s: Segment, p: Paint) = canvas.drawLine(m.x(s.a), m.y(s.a), m.x(s.b), m.y(s.b), p)
        for (s in r.strings) line(s, string)
        r.stretch?.let { line(it, stretch) }
        for (a in r.anchors) if (a != null) canvas.drawCircle(m.x(a), m.y(a), 20f, anchor)
        for (p in r.pluckers) if (p != null) canvas.drawCircle(m.x(p), m.y(p), 16f, plucker)
    }
}
