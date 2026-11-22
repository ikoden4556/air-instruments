package com.airinstruments

import android.opengl.GLES30.*
import android.opengl.GLSurfaceView
import android.os.SystemClock
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.FloatBuffer
import java.util.concurrent.ConcurrentLinkedQueue
import javax.microedition.khronos.egl.EGLConfig
import javax.microedition.khronos.opengles.GL10
import kotlin.math.hypot
import kotlin.math.min

/**
 * OpenGL ES 3 drawing over the camera preview, matching web/src/gl-renderer.ts: every element
 * is a glowing capsule (a point is a zero-length one), drawn in pixels with MAX blending.
 */
class GlRenderer(private val activity: MainActivity) : GLSurfaceView.Renderer {
    private class Hit(val string: Segment, val velocity: Float, val at: Point?)

    private val hits = ConcurrentLinkedQueue<Hit>()
    private val plucked = HashMap<Segment, Pair<Long, Float>>()
    private var sparks = emptyList<Scene.Spark>()
    private var lastMs = 0L
    private var width = 1
    private var height = 1
    private var sizeUniform = 0
    private var data = FloatArray(STRIDE * 6 * 512)
    private var buffer: FloatBuffer = direct(data.size)
    private var count = 0

    /** Called on the Engine thread: remembers which Strings were Plucked, and where. */
    fun onPlucks(plucks: DoubleArray, n: Int, scene: RenderData) {
        for (i in 0 until n) {
            val s = scene.strings.getOrNull(plucks[i * Native.PLUCK_NUMBERS].toInt()) ?: continue
            val velocity = plucks[i * Native.PLUCK_NUMBERS + 2].toFloat()
            // The Plucker closest to the String crossed it: spark at its nearest point on the String.
            val at = scene.pluckers.filterNotNull()
                .map { Scene.nearestOnSegment(s.a, s.b, it).let { n -> n to hypot(it.x - n.x, it.y - n.y) } }
                .minByOrNull { it.second }?.first
            hits += Hit(s, velocity, at)
        }
    }

    override fun onSurfaceCreated(unused: GL10?, config: EGLConfig?) {
        val program = glCreateProgram()
        for ((type, src) in listOf(GL_VERTEX_SHADER to VERTEX, GL_FRAGMENT_SHADER to FRAGMENT)) {
            val s = glCreateShader(type)
            glShaderSource(s, src)
            glCompileShader(s)
            val ok = IntArray(1)
            glGetShaderiv(s, GL_COMPILE_STATUS, ok, 0)
            check(ok[0] != 0) { glGetShaderInfoLog(s) }
            glAttachShader(program, s)
        }
        glLinkProgram(program)
        glUseProgram(program)
        sizeUniform = glGetUniformLocation(program, "size")
        val vao = IntArray(1).also { glGenVertexArrays(1, it, 0) }
        glBindVertexArray(vao[0])
        val vbo = IntArray(1).also { glGenBuffers(1, it, 0) }
        glBindBuffer(GL_ARRAY_BUFFER, vbo[0])
        var offset = 0
        for ((name, n) in listOf("pos" to 2, "seg" to 4, "radius" to 1, "color" to 4)) {
            val loc = glGetAttribLocation(program, name)
            glEnableVertexAttribArray(loc)
            glVertexAttribPointer(loc, n, GL_FLOAT, false, STRIDE * 4, offset * 4)
            offset += n
        }
        // Overlapping pieces of one glow keep the brightest instead of doubling into beads.
        glEnable(GL_BLEND)
        glBlendEquation(GL_MAX)
    }

    override fun onSurfaceChanged(unused: GL10?, w: Int, h: Int) {
        width = w
        height = h
        glViewport(0, 0, w, h)
    }

    override fun onDrawFrame(unused: GL10?) {
        val now = SystemClock.uptimeMillis()
        val dt = if (lastMs == 0L) 0f else min(0.1f, (now - lastMs) / 1000f)
        lastMs = now
        val u = min(width, height) / 720f
        val size = activity.imageSize
        val m = CoverMapping(size.width.toFloat(), size.height.toFloat(), width.toFloat(), height.toFloat())
        fun map(p: Point) = Point(m.x(p), m.y(p))

        while (true) {
            val h = hits.poll() ?: break
            plucked[h.string] = now to h.velocity
            h.at?.let { sparks = sparks + Scene.burst(map(it), 10, 500 * u) }
        }
        sparks = Scene.stepSparks(sparks, dt)
        count = 0

        val scene = activity.scene
        for (hand in activity.handPoints) {
            for ((a, b) in Scene.HAND_CONNECTIONS) {
                capsule(map(Point(hand[2 * a], hand[2 * a + 1])), map(Point(hand[2 * b], hand[2 * b + 1])), 5 * u, SKELETON)
            }
        }
        for (s in scene.strings) {
            val (a, b) = map(s.a) to map(s.b)
            val offset = plucked[s]?.let { (at, v) -> Scene.vibration((now - at).toFloat(), v, 14 * u) } ?: 0f
            val pts = if (offset != 0f) Scene.bowedString(a, b, offset) else listOf(a, b)
            for (i in 1 until pts.size) capsule(pts[i - 1], pts[i], 11 * u, STRING)
            for (t in Scene.fretTicks(a, b)) capsule(t, t, 20 * u, TICK)
        }
        plucked.entries.removeAll { (s, hit) -> s !in scene.strings || now - hit.first > Scene.VIBRATION_MS }
        scene.stretch?.let { capsule(map(it.a), map(it.b), 7 * u, STRETCH) }
        for (a in scene.anchors) if (a != null) map(a).let { capsule(it, it, 22 * u, ANCHOR) }
        for (p in scene.pluckers) if (p != null) map(p).let { capsule(it, it, 18 * u, PLUCKER) }
        for (s in sparks) {
            capsule(Point(s.x, s.y), Point(s.x - s.vx * 0.02f, s.y - s.vy * 0.02f), 6 * u,
                floatArrayOf(SPARK[0], SPARK[1], SPARK[2], s.life * 2))
        }

        glClearColor(0f, 0f, 0f, 0f)
        glClear(GL_COLOR_BUFFER_BIT)
        glUniform2f(sizeUniform, width.toFloat(), height.toFloat())
        if (buffer.capacity() < data.size) buffer = direct(data.size)
        buffer.clear()
        buffer.put(data, 0, count * STRIDE).flip()
        glBufferData(GL_ARRAY_BUFFER, count * STRIDE * 4, buffer, GL_STREAM_DRAW)
        glDrawArrays(GL_TRIANGLES, 0, count)
    }

    /** Adds a capsule from a to b as two triangles covering it and its glow. */
    private fun capsule(a: Point, b: Point, radius: Float, color: FloatArray) {
        val len = hypot(b.x - a.x, b.y - a.y)
        val (dx, dy) = if (len > 1e-6f) (b.x - a.x) / len to (b.y - a.y) / len else 1f to 0f
        val (ax, ay) = a.x - dx * radius to a.y - dy * radius
        val (bx, by) = b.x + dx * radius to b.y + dy * radius
        val (nx, ny) = -dy * radius to dx * radius
        if ((count + 6) * STRIDE > data.size) data = data.copyOf(data.size * 2)
        for ((x, y) in listOf(ax + nx to ay + ny, ax - nx to ay - ny, bx + nx to by + ny,
            bx + nx to by + ny, ax - nx to ay - ny, bx - nx to by - ny)) {
            val at = count * STRIDE
            data[at] = x; data[at + 1] = y
            data[at + 2] = a.x; data[at + 3] = a.y; data[at + 4] = b.x; data[at + 5] = b.y
            data[at + 6] = radius
            color.copyInto(data, at + 7)
            count++
        }
    }

    companion object {
        /** Floats per vertex: pos 2, seg 4, radius 1, color 4. */
        private const val STRIDE = 11

        private val STRING = floatArrayOf(0.35f, 0.95f, 1f, 1f)
        private val STRETCH = floatArrayOf(1f, 1f, 1f, 0.45f)
        private val TICK = floatArrayOf(0.85f, 1f, 1f, 1f)
        private val ANCHOR = floatArrayOf(1f, 0.85f, 0.35f, 1f)
        private val PLUCKER = floatArrayOf(1f, 0.4f, 0.95f, 1f)
        private val SKELETON = floatArrayOf(1f, 1f, 1f, 0.12f)
        private val SPARK = floatArrayOf(1f, 0.9f, 0.6f, 1f)

        private fun direct(floats: Int): FloatBuffer =
            ByteBuffer.allocateDirect(floats * 4).order(ByteOrder.nativeOrder()).asFloatBuffer()

        private const val VERTEX = """#version 300 es
in vec2 pos;
in vec4 seg;
in float radius;
in vec4 color;
uniform vec2 size;
out vec2 vPos;
flat out vec4 vSeg;
flat out float vRadius;
flat out vec4 vColor;
void main() {
  vPos = pos; vSeg = seg; vRadius = radius; vColor = color;
  vec2 clip = pos / size * 2.0 - 1.0;
  gl_Position = vec4(clip.x, -clip.y, 0.0, 1.0);
}"""

        private const val FRAGMENT = """#version 300 es
precision highp float;
in vec2 vPos;
flat in vec4 vSeg;
flat in float vRadius;
flat in vec4 vColor;
out vec4 outColor;
void main() {
  vec2 pa = vPos - vSeg.xy, ba = vSeg.zw - vSeg.xy;
  float h = clamp(dot(pa, ba) / max(dot(ba, ba), 1e-6), 0.0, 1.0);
  float d = length(pa - ba * h) / vRadius;
  float core = smoothstep(0.3, 0.15, d);
  float glow = 0.55 * exp(-5.0 * d * d);
  float i = (core + glow) * smoothstep(1.0, 0.85, d) * vColor.a;
  outColor = vec4(vColor.rgb * i, i);
}"""
    }
}
