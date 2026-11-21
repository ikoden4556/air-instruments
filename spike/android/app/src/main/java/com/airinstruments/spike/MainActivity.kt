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
    @Volatile private var lastPluckNs = 0L
    @Volatile private var frames = 0
    @Volatile private var plucks = 0
    private var readPlucks = 0
    private var camInfo = ""
    private var audioStart = 0
    private val captureToResult = ArrayDeque<Double>()
    private val captureToSpeaker = ArrayDeque<Double>()
    private var lastSpeaker = -1.0

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

        audioStart = Native.start()
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

        ui.post(object : Runnable { override fun run() { render(); ui.postDelayed(this, 500) } })
    }

    private fun startCamera() {
        val future = ProcessCameraProvider.getInstance(this)
        future.addListener({
            val provider = future.get()
            val front = CameraSelector.DEFAULT_FRONT_CAMERA
            val info = front.filter(provider.availableCameraInfos).first()
            val ranges = Camera2CameraInfo.from(info).getCameraCharacteristic(CameraCharacteristics.CONTROL_AE_AVAILABLE_TARGET_FPS_RANGES)
            val range = ranges?.filter { it.upper >= 60 }?.maxByOrNull { it.lower } ?: ranges?.maxByOrNull { it.upper } ?: Range(30, 30)
            camInfo = "fps range $range of ${ranges?.joinToString()}"

            val previewBuilder = Preview.Builder()
            Camera2Interop.Extender(previewBuilder).setCaptureRequestOption(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, range)
            val preview = previewBuilder.build().also { it.surfaceProvider = previewView.surfaceProvider }

            val analysisBuilder = ImageAnalysis.Builder()
                .setResolutionSelector(ResolutionSelector.Builder().setResolutionStrategy(
                    ResolutionStrategy(Size(640, 480), ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER)).build())
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .setOutputImageFormat(ImageAnalysis.OUTPUT_IMAGE_FORMAT_RGBA_8888)
            Camera2Interop.Extender(analysisBuilder).setCaptureRequestOption(CaptureRequest.CONTROL_AE_TARGET_FPS_RANGE, range)
            val analysis = analysisBuilder.build().also { it.setAnalyzer(analysisThread, ::analyze) }

            provider.unbindAll()
            provider.bindToLifecycle(this, front, preview, analysis)
        }, mainExecutor)
    }

    private fun analyze(img: ImageProxy) {
        frames++
        val ts = img.imageInfo.timestamp
        // camera timestamps are either BOOTTIME (elapsedRealtime) or MONOTONIC (nanoTime); normalise to MONOTONIC
        val realtime = realtimeClock ?: (abs(ts - SystemClock.elapsedRealtimeNanos()) < abs(ts - System.nanoTime())).also { realtimeClock = it }
        val captureMono = if (realtime) ts - (SystemClock.elapsedRealtimeNanos() - System.nanoTime()) else ts
        val bitmap = img.toBitmap()
        val rotation = img.imageInfo.rotationDegrees
        img.close()
        landmarker.detectAsync(BitmapImageBuilder(bitmap).build(),
            ImageProcessingOptions.builder().setRotationDegrees(rotation).build(), captureMono / 1_000_000)
    }

    private fun onResult(r: HandLandmarkerResult) {
        val captureNs = r.timestampMs() * 1_000_000
        val now = System.nanoTime()
        synchronized(captureToResult) { captureToResult.addLast((now - captureNs) / 1e6); if (captureToResult.size > 60) captureToResult.removeFirst() }
        val dots = ArrayList<PointF>()
        val seen = HashSet<String>()
        r.landmarks().forEachIndexed { i, lm ->
            val hand = r.handedness()[i][0].categoryName()
            seen += hand
            val x = lm[8].x()
            dots += PointF(1 - x, lm[8].y()) // front camera preview is mirrored
            val p = prevX[hand]
            if (p != null && (p - 0.5f) * (x - 0.5f) < 0 && now - lastPluckNs > 60_000_000) {
                lastPluckNs = now
                plucks++
                Native.pluck(captureNs)
            }
            prevX[hand] = x
        }
        prevX.keys.retainAll(seen)
        overlay.dots = dots
        overlay.postInvalidate()
    }

    private fun stat(a: ArrayDeque<Double>): String = synchronized(a) {
        if (a.isEmpty()) return "n/a"
        val s = a.sorted()
        "avg %.1f  p95 %.1f ms".format(s.average(), s[(s.size * 0.95).toInt().coerceAtMost(s.size - 1)])
    }

    private fun render() {
        val fps = frames * 2; frames = 0
        if (plucks != readPlucks) {
            readPlucks = plucks
            lastSpeaker = Native.lastCaptureToSpeakerMs()
            if (lastSpeaker > 0) synchronized(captureToSpeaker) { captureToSpeaker.addLast(lastSpeaker); if (captureToSpeaker.size > 60) captureToSpeaker.removeFirst() }
        }
        val i = Native.info()
        hud.text = """
PROTOTYPE latency spike (native)  ${android.os.Build.MODEL}
camera          $camInfo
frames/s        $fps
capture->result ${stat(captureToResult)}
capture->SPEAKER ${stat(captureToSpeaker)}  last ${"%.0f".format(lastSpeaker)} ms
audio           start=$audioStart exclusive=${i and 1 == 1} lowlatency=${i shr 1 and 1 == 1} burst=${i shr 8}
plucks          $plucks
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
