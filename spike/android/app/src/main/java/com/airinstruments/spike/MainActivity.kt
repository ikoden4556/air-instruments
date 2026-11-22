// PROTOTYPE: throwaway phase-2 latency spike. Answers: how fast is camera -> hand tracking -> sound natively?
package com.airinstruments.spike

import android.Manifest
import android.content.Context
import android.content.pm.PackageManager
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.Paint
import android.graphics.PointF
import android.graphics.Typeface
import android.hardware.camera2.CameraCharacteristics
import android.hardware.camera2.CaptureRequest
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.util.Range
import android.util.Size
import android.view.View
import android.view.WindowManager
import android.widget.FrameLayout
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.camera2.interop.Camera2CameraInfo
import androidx.camera.camera2.interop.Camera2Interop
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import com.google.mediapipe.framework.image.BitmapImageBuilder
import com.google.mediapipe.tasks.core.BaseOptions
import com.google.mediapipe.tasks.core.Delegate
import com.google.mediapipe.tasks.vision.core.ImageProcessingOptions
import com.google.mediapipe.tasks.vision.core.RunningMode
import com.google.mediapipe.tasks.vision.handlandmarker.HandLandmarker
import com.google.mediapipe.tasks.vision.handlandmarker.HandLandmarkerResult
import java.util.concurrent.Executors
import kotlin.math.abs

class MainActivity : ComponentActivity() {
    private lateinit var previewView: PreviewView
    private lateinit var overlay: Overlay
    private lateinit var hud: TextView
    private lateinit var landmarker: HandLandmarker
    private val analysisThread = Executors.newSingleThreadExecutor()
    private val ui = Handler(Looper.getMainLooper())

    private var realtimeClock: Boolean? = null
    private val prevX = HashMap<String, Float>()
    private val prevT = HashMap<String, Long>()
    private val predictedAt = HashMap<String, Long>() // capture time of a predicted Pluck awaiting its real crossing
    private val lookaheadMs by lazy { intent.getIntExtra("look", 40) }
    @Volatile private var pendingRef = -1L // camera-timeline time the finger really crossed, for the last Pluck
    private var lastPresented = -1L
    private val felt = ArrayDeque<Double>()
    private val feltPredicted = ArrayDeque<Double>()
    private val feltFallback = ArrayDeque<Double>()
    @Volatile private var pendingPredicted = false
    @Volatile private var predictedHits = 0
    @Volatile private var fallbackPlucks = 0
    @Volatile private var falsePlucks = 0
    @Volatile private var lastPluckNs = 0L
    @Volatile private var frames = 0
    @Volatile private var plucks = 0
    private var readPlucks = 0
    private var camInfo = ""
    private var audioStart = 0
    private val captureToResult = ArrayDeque<Double>()
    private val captureToSpeaker = ArrayDeque<Double>()
    private val captureToArrive = ArrayDeque<Double>()
    private val convert = ArrayDeque<Double>()
    private val inference = ArrayDeque<Double>()
    private val audioOut = ArrayDeque<Double>()
    private val submitted = java.util.concurrent.ConcurrentHashMap<Long, Long>()
    @Volatile private var results = 0
    private var ticks = 0
    private var fps = 0
    private var rps = 0
    private var lastSpeaker = -1.0
    private var reuse: android.graphics.Bitmap? = null

    // variants, chosen with: adb shell am start -n ... --ei bufs 1 --ez preview false --es fps manual --ez reuse true
    private val bufs by lazy { intent.getIntExtra("bufs", 2) }
    private val withPreview by lazy { intent.getBooleanExtra("preview", true) }
    private val fpsMode by lazy { intent.getStringExtra("fps") ?: "range" }
    private val reuseBitmap by lazy { intent.getBooleanExtra("reuse", false) }

    private fun add(a: ArrayDeque<Double>, v: Double) = synchronized(a) { a.addLast(v); if (a.size > 60) a.removeFirst() }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        previewView = PreviewView(this)
        overlay = Overlay(this)
        hud = TextView(this).apply {
            typeface = Typeface.MONOSPACE; textSize = 12f; setTextColor(Color.WHITE)
            setBackgroundColor(0x99000000.toInt()); setPadding(16, 12, 16, 12)
            text = "PROTOTYPE latency spike (native). Starting..."
        }
        setContentView(FrameLayout(this).apply {
            addView(previewView); addView(this@MainActivity.overlay)
            addView(hud, FrameLayout.LayoutParams(FrameLayout.LayoutParams.WRAP_CONTENT, FrameLayout.LayoutParams.WRAP_CONTENT))
        })

        audioStart = Native.start(bufs)
        landmarker = HandLandmarker.createFromOptions(this, HandLandmarker.HandLandmarkerOptions.builder()
            .setBaseOptions(BaseOptions.builder().setModelAssetPath("hand_landmarker.task").setDelegate(Delegate.GPU).build())
            .setRunningMode(RunningMode.LIVE_STREAM)
            .setNumHands(2)
            .setResultListener { r, _ -> onResult(r) }
            .setErrorListener { e -> ui.post { hud.text = "MediaPipe error: ${e.message}" } }
            .build())

        if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) startCamera()
        else registerForActivityResult(ActivityResultContracts.RequestPermission()) { if (it) startCamera() }
            .launch(Manifest.permission.CAMERA)

        ui.post(object : Runnable { override fun run() { render(); ui.postDelayed(this, 100) } })
    }

    private fun startCamera() {
        val future = ProcessCameraProvider.getInstance(this)
        future.addListener({
            val provider = future.get()
            val front = if (intent.getStringExtra("cam") == "back") CameraSelector.DEFAULT_BACK_CAMERA else CameraSelector.DEFAULT_FRONT_CAMERA
            val info = front.filter(provider.availableCameraInfos).first()
            val ranges = Camera2CameraInfo.from(info).getCameraCharacteristic(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES)
            val range = ranges?.filter { it.upper >= 60 }?.maxByOrNull { it.lower } ?: ranges?.maxByOrNull { it.upper } ?: Range(30, 30)
            camInfo = "${intent.getStringExtra("cam") ?: "front"} fps=$fpsMode range $range of ${ranges?.joinToString()}\n                preview=$withPreview reuse=$reuseBitmap bufs=$bufs"

            // "manual": auto-exposure off, 8 ms exposure, 16.7 ms frame duration -> forces 60 fps regardless of light
            fun tune(e: Camera2Interop.Extender<*>) {
                if (fpsMode == "manual") {
                    e.setCaptureRequestOption(CaptureRequest.CONTROL_AE_MODE, CaptureRequest.CONTROL_AE_MODE_OFF)
                    e.setCaptureRequestOption(CaptureRequest.SENSOR_EXPOSURE_TIME, 8_000_000L)
                    e.setCaptureRequestOption(CaptureRequest.SENSOR_FRAME_DURATION, 16_666_667L)
                    e.setCaptureRequestOption(CaptureRequest.SENSOR_SENSITIVITY, 1600)
                } else e.setCaptureRequestOption(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, range)
            }

            val previewBuilder = Preview.Builder()
            if (fpsMode == "target") previewBuilder.setTargetFrameRate(range)
            tune(Camera2Interop.Extender(previewBuilder))
            val preview = previewBuilder.build().also { it.surfaceProvider = previewView.surfaceProvider }

            val analysisBuilder = ImageAnalysis.Builder()
                .setResolutionSelector(ResolutionSelector.Builder().setResolutionStrategy(
                    ResolutionStrategy(Size(640, 480), ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER)).build())
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .setOutputImageFormat(ImageAnalysis.OUTPUT_IMAGE_FORMAT_RGBA_8888)
            tune(Camera2Interop.Extender(analysisBuilder))
            val analysis = analysisBuilder.build().also { it.setAnalyzer(analysisThread, ::analyze) }

            provider.unbindAll()
            if (withPreview) provider.bindToLifecycle(this, front, preview, analysis)
            else provider.bindToLifecycle(this, front, analysis)
        }, mainExecutor)
    }

    private fun analyze(img: ImageProxy) {
        frames++
        val ts = img.imageInfo.timestamp
        // camera timestamps are either BOOTTIME (elapsedRealtime) or MONOTONIC (nanoTime); normalise to MONOTONIC
        val realtime = realtimeClock ?: (abs(ts - SystemClock.elapsedRealtimeNanos()) < abs(ts - System.nanoTime())).also { realtimeClock = it }
        val captureMono = if (realtime) ts - (SystemClock.elapsedRealtimeNanos() - System.nanoTime()) else ts
        val t0 = System.nanoTime()
        add(captureToArrive, (t0 - captureMono) / 1e6)
        val plane = img.planes[0]
        val bitmap = if (reuseBitmap && plane.rowStride == img.width * 4) {
            val b = reuse?.takeIf { it.width == img.width && it.height == img.height }
                ?: android.graphics.Bitmap.createBitmap(img.width, img.height, android.graphics.Bitmap.Config.ARGB_8888).also { reuse = it }
            plane.buffer.rewind(); b.copyPixelsFromBuffer(plane.buffer); b
        } else img.toBitmap()
        val rotation = img.imageInfo.rotationDegrees
        img.close()
        val t1 = System.nanoTime()
        add(convert, (t1 - t0) / 1e6)
        val tsMs = captureMono / 1_000_000
        submitted[tsMs] = t1
        landmarker.detectAsync(BitmapImageBuilder(bitmap).build(),
            ImageProcessingOptions.builder().setRotationDegrees(rotation).build(), tsMs)
    }

    private fun onResult(r: HandLandmarkerResult) {
        val captureNs = r.timestampMs() * 1_000_000
        val now = System.nanoTime()
        results++
        add(captureToResult, (now - captureNs) / 1e6)
        submitted.remove(r.timestampMs())?.let { add(inference, (now - it) / 1e6) }
        val dots = ArrayList<PointF>()
        val seen = HashSet<String>()
        r.landmarks().forEachIndexed { i, lm ->
            val hand = r.handedness()[i][0].categoryName()
            seen += hand
            val x = lm[8].x()
            dots += PointF(1 - x, lm[8].y()) // front camera preview is mirrored
            val p = prevX[hand]
            val pt = prevT[hand]
            val pred = predictedAt[hand]
            if (pred != null && captureNs - pred > 250_000_000) { predictedAt.remove(hand); falsePlucks++ } // never crossed
            if (p != null && pt != null && captureNs > pt) {
                val v = (x - p) / (captureNs - pt) // screen widths per ns
                if ((p - 0.5f) * (x - 0.5f) < 0) {
                    // real crossing: interpolate when it happened on the camera timeline
                    val tc = pt + ((0.5f - p) / (x - p) * (captureNs - pt)).toLong()
                    if (predictedAt.remove(hand) != null) { predictedHits++; pendingPredicted = true; pendingRef = tc }
                    else if (now - lastPluckNs > 60_000_000) {
                        lastPluckNs = now; plucks++; fallbackPlucks++
                        Native.pluck(captureNs); pendingPredicted = false; pendingRef = tc
                    }
                } else if (pred == null && lookaheadMs > 0 && abs(v) * 1e9 > 0.5 && (0.5f - x) / v > 0 &&
                    (0.5f - x) / v <= lookaheadMs * 1e6 && now - lastPluckNs > 60_000_000) {
                    // moving fast toward the String and will cross within the lookahead: Pluck now
                    lastPluckNs = now; plucks++
                    predictedAt[hand] = captureNs
                    Native.pluck(captureNs)
                }
            }
            prevX[hand] = x
            prevT[hand] = captureNs
        }
        prevX.keys.retainAll(seen); prevT.keys.retainAll(seen)
        predictedAt.keys.filter { it !in seen }.forEach { predictedAt.remove(it); falsePlucks++ }
        overlay.dots = dots
        overlay.postInvalidate()
    }

    private fun stat(a: ArrayDeque<Double>): String = synchronized(a) {
        if (a.isEmpty()) return "n/a"
        val s = a.sorted()
        "avg %.1f  p95 %.1f ms".format(s.average(), s[(s.size * 0.95).toInt().coerceAtMost(s.size - 1)])
    }

    private fun render() {
        if (++ticks % 5 == 0) { fps = frames * 2; frames = 0; rps = results * 2; results = 0 }
        if (submitted.size > 100) submitted.clear() // dropped frames never get a result
        if (plucks != readPlucks) {
            readPlucks = plucks
            lastSpeaker = Native.lastCaptureToSpeakerMs()
            if (lastSpeaker > 0) add(captureToSpeaker, lastSpeaker)
        }
        Native.outputLatencyMs(System.nanoTime()).let { if (it > 0) add(audioOut, it) }
        // felt latency = when the beep hit the speaker minus when the finger really crossed (negative = early)
        val presented = Native.beepPresentedNs()
        val ref = pendingRef
        if (ref > 0 && presented > 0 && presented != lastPresented) {
            val ms = (presented - ref) / 1e6
            add(felt, ms); add(if (pendingPredicted) feltPredicted else feltFallback, ms); lastPresented = presented; pendingRef = -1
        }
        val i = Native.info()
        hud.text = """
PROTOTYPE latency spike (native)  ${android.os.Build.MODEL}
mode            $camInfo
frames/s        camera $fps  results $rps
capture->arrive ${stat(captureToArrive)}
convert         ${stat(convert)}
inference       ${stat(inference)}
capture->result ${stat(captureToResult)}
audio out       ${stat(audioOut)}
capture->SPEAKER ${stat(captureToSpeaker)}  last ${"%.0f".format(lastSpeaker)} ms
audio           start=$audioStart exclusive=${i and 1 == 1} lowlatency=${i shr 1 and 1 == 1} burst=${i shr 8} buffer=${Native.bufferFrames()}
plucks          $plucks  (predicted+crossed $predictedHits, late fallback $fallbackPlucks, FALSE $falsePlucks)
lookahead       $lookaheadMs ms
FELT latency    ${stat(felt)}  min ${synchronized(felt) { felt.minOrNull()?.let { "%.0f".format(it) } ?: "-" }} ms
  predicted     ${stat(feltPredicted)}  min ${synchronized(feltPredicted) { feltPredicted.minOrNull()?.let { "%.0f".format(it) } ?: "-" }} ms
  late fallback ${stat(feltFallback)}
Swipe an index fingertip across the cyan line.""".trim()
    }

    class Overlay(context: Context) : View(context) {
        @Volatile var dots: List<PointF> = emptyList()
        private val line = Paint().apply { color = Color.CYAN; strokeWidth = 6f }
        private val dot = Paint().apply { color = Color.MAGENTA }
        override fun onDraw(canvas: Canvas) {
            canvas.drawLine(width / 2f, 0f, width / 2f, height.toFloat(), line)
            for (d in dots) canvas.drawCircle(d.x * width, d.y * height, 18f, dot)
        }
    }
}
