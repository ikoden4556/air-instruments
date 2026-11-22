package com.airinstruments

import android.Manifest
import android.content.pm.PackageManager
import android.graphics.PixelFormat
import android.opengl.GLSurfaceView
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.View
import android.widget.TextView
import java.util.concurrent.atomic.AtomicInteger
import android.util.Range
import android.util.Size
import android.view.WindowManager
import android.widget.FrameLayout
import androidx.activity.ComponentActivity
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.CameraSelector
import androidx.camera.core.ImageAnalysis
import androidx.camera.core.ImageProxy
import androidx.camera.core.Preview
import androidx.camera.core.resolutionselector.AspectRatioStrategy
import androidx.camera.core.resolutionselector.ResolutionSelector
import androidx.camera.core.resolutionselector.ResolutionStrategy
import androidx.camera.lifecycle.ProcessCameraProvider
import androidx.camera.view.PreviewView
import com.google.mediapipe.framework.image.BitmapImageBuilder
import com.google.mediapipe.framework.image.MPImage
import com.google.mediapipe.tasks.core.BaseOptions
import com.google.mediapipe.tasks.core.Delegate
import com.google.mediapipe.tasks.vision.core.ImageProcessingOptions
import com.google.mediapipe.tasks.vision.core.RunningMode
import com.google.mediapipe.tasks.vision.handlandmarker.HandLandmarker
import com.google.mediapipe.tasks.vision.handlandmarker.HandLandmarkerResult
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.Executors

/**
 * Camera (front, 60 fps) → MediaPipe (GPU, LIVE_STREAM) → the Rust Engine, which sends its
 * Plucks straight to the AAudio synth. The Engine lives on MediaPipe's result thread; anything
 * else that touches it is queued with [onEngine] and run there.
 */
class MainActivity : ComponentActivity() {
    private lateinit var preview: PreviewView
    private lateinit var glView: GLSurfaceView
    private var landmarker: HandLandmarker? = null
    private val analysisThread = Executors.newSingleThreadExecutor()

    private var engine = 0L
    private val engineActions = ConcurrentLinkedQueue<(Long) -> Unit>()
    private val hands = FloatArray(2 * HandFrame.HAND_FLOATS)
    private val plucks = DoubleArray(Native.PLUCK_NUMBERS * Native.MAX_PLUCKS)
    private val render = FloatArray(RenderData.MAX)

    /** Called on the Engine thread with each frame's Plucks (array, count) and capture time. */
    val pluckListeners = mutableListOf<(DoubleArray, Int, Long) -> Unit>()

    /** The upright camera image size, for mapping screen space to the view. */
    @Volatile var imageSize = Size(640, 480)
        private set
    @Volatile var scene = RenderData.EMPTY
        private set
    /** Each tracked Hand's landmarks in screen space (x0, y0, x1, y1 …), for the skeleton. */
    @Volatile var handPoints: List<FloatArray> = emptyList()
        private set
    @Volatile var audioInfo = 0
        private set

    private var boottime: Boolean? = null
    private var lastSubmittedMs = -1L

    // Latency readout (BLUEPRINT §4, Measuring latency).
    private lateinit var readout: TextView
    private val ui = Handler(Looper.getMainLooper())
    private val cameraFrames = AtomicInteger()
    private val results = AtomicInteger()
    private val captureToResult = Rolling()
    private val felt = FeltLatency()
    @Volatile private var falsePlucks = 0
    private val lastPluck = LongArray(2)
    private val engineControls = object : Controls.Controlled {
        override fun undo() = onEngine { Native.engineUndo(it) }
        override fun clear() = onEngine { Native.engineClear(it) }
        override fun setKey(root: Int) = onEngine { Native.engineSetKey(it, root) }
    }

    /** Runs `action` with the Engine on the Engine thread, before the next frame. */
    fun onEngine(action: (Long) -> Unit) {
        engineActions += action
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        window.addFlags(WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON)
        // The preview is a SurfaceView; the GL drawing is a translucent SurfaceView just above it.
        preview = PreviewView(this).apply {
            scaleType = PreviewView.ScaleType.FILL_CENTER
            implementationMode = PreviewView.ImplementationMode.PERFORMANCE
        }
        val renderer = GlRenderer(this)
        pluckListeners += { plucks, n, _ -> renderer.onPlucks(plucks, n, scene) }
        glView = GLSurfaceView(this).apply {
            setEGLContextClientVersion(3)
            setEGLConfigChooser(8, 8, 8, 8, 0, 0)
            holder.setFormat(PixelFormat.TRANSLUCENT)
            setZOrderMediaOverlay(true)
            setRenderer(renderer)
            renderMode = GLSurfaceView.RENDERMODE_WHEN_DIRTY
        }
        pluckListeners += { plucks, n, _ ->
            for (i in 0 until n) {
                val at = i * Native.PLUCK_NUMBERS
                felt.onPluck(plucks[at + 3].toLong(), plucks[at + 4] == 1.0)
            }
        }
        readout = readoutView(this)
        setContentView(FrameLayout(this).apply {
            addView(this@MainActivity.preview)
            addView(this@MainActivity.glView)
            addView(this@MainActivity.readout)
            addView(controlBar(this@MainActivity, engineControls) {
                readout.visibility = if (readout.visibility == View.VISIBLE) View.GONE else View.VISIBLE
            })
        })
        ui.post(object : Runnable {
            override fun run() {
                updateReadout()
                ui.postDelayed(this, READOUT_MS)
            }
        })

        audioInfo = Native.audioStart()
        landmarker = HandLandmarker.createFromOptions(
            this,
            HandLandmarker.HandLandmarkerOptions.builder()
                .setBaseOptions(
                    BaseOptions.builder().setModelAssetPath("hand_landmarker.task").setDelegate(Delegate.GPU).build(),
                )
                .setRunningMode(RunningMode.LIVE_STREAM)
                .setNumHands(2)
                .setResultListener(::onResult)
                .build(),
        )

        if (checkSelfPermission(Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            startCamera()
        } else {
            registerForActivityResult(ActivityResultContracts.RequestPermission()) { if (it) startCamera() }
                .launch(Manifest.permission.CAMERA)
        }
    }

    private fun startCamera() {
        val future = ProcessCameraProvider.getInstance(this)
        future.addListener({
            val provider = future.get()
            // Preview and analysis share the 4:3 frame, so landmarks line up with the preview.
            val ratio = AspectRatioStrategy.RATIO_4_3_FALLBACK_AUTO_STRATEGY
            val analysis = ImageAnalysis.Builder()
                .setResolutionSelector(
                    ResolutionSelector.Builder()
                        .setAspectRatioStrategy(ratio)
                        .setResolutionStrategy(
                            ResolutionStrategy(Size(640, 480), ResolutionStrategy.FALLBACK_RULE_CLOSEST_HIGHER_THEN_LOWER),
                        )
                        .build(),
                )
                .setBackpressureStrategy(ImageAnalysis.STRATEGY_KEEP_ONLY_LATEST)
                .setOutputImageFormat(ImageAnalysis.OUTPUT_IMAGE_FORMAT_RGBA_8888)
                .build()
                .also { it.setAnalyzer(analysisThread, ::analyze) }
            fun previewAt(fps: Range<Int>?) = Preview.Builder()
                .setResolutionSelector(ResolutionSelector.Builder().setAspectRatioStrategy(ratio).build())
                .apply { if (fps != null) setTargetFrameRate(fps) }
                .build()
                .also { it.surfaceProvider = preview.surfaceProvider }
            provider.unbindAll()
            // The front camera reaches 60 fps only this way, with the preview bound (phase 2 spike).
            try {
                provider.bindToLifecycle(this, CameraSelector.DEFAULT_FRONT_CAMERA, previewAt(Range(60, 60)), analysis)
            } catch (e: IllegalArgumentException) {
                provider.unbindAll()
                provider.bindToLifecycle(this, CameraSelector.DEFAULT_FRONT_CAMERA, previewAt(null), analysis)
            }
        }, mainExecutor)
    }

    private fun updateReadout() {
        // Always follow the audio stream, so felt latency is recorded even while hidden.
        if (Native.audioLastPluck(lastPluck)) felt.onSpeaker(lastPluck[0], lastPluck[1])
        if (readout.visibility != View.VISIBLE) return
        onEngine { falsePlucks = Native.engineFalsePlucks(it) }
        val perSecond = 1000.0 / READOUT_MS
        val info = audioInfo
        readout.text = """
            camera          ${(cameraFrames.getAndSet(0) * perSecond).toInt()} fps   results ${(results.getAndSet(0) * perSecond).toInt()}/s
            capture→result  ${captureToResult.summary()}
            audio out       ${"%.0f".format(Native.audioOutputLatencyMs(System.nanoTime()))} ms   ${if (info < 0) "error $info" else "exclusive=${info and 1 == 1} lowLatency=${info shr 1 and 1 == 1} burst=${info shr 8}"}
            FELT            ${felt.all.summary()}
              predicted     ${felt.predicted.summary()}
              real          ${felt.real.summary()}
            FALSE Plucks    $falsePlucks
            (felt = speaker time − crossing time; predicted Plucks use the predicted crossing)
        """.trimIndent()
    }

    private fun analyze(image: ImageProxy) {
        cameraFrames.incrementAndGet()
        val ts = image.imageInfo.timestamp
        val (bootNow, monoNow) = SystemClock.elapsedRealtimeNanos() to System.nanoTime()
        val boot = boottime ?: CameraClock.isBoottime(ts, bootNow, monoNow).also { boottime = it }
        val captureMs = CameraClock.toMonotonic(ts, boot, bootNow, monoNow) / 1_000_000
        val rotation = image.imageInfo.rotationDegrees
        imageSize = if (rotation % 180 == 0) Size(image.width, image.height) else Size(image.height, image.width)
        val bitmap = image.toBitmap()
        image.close()
        if (captureMs <= lastSubmittedMs) return // MediaPipe needs increasing timestamps
        lastSubmittedMs = captureMs
        landmarker?.detectAsync(
            BitmapImageBuilder(bitmap).build(),
            ImageProcessingOptions.builder().setRotationDegrees(rotation).build(),
            captureMs,
        )
    }

    private fun onResult(result: HandLandmarkerResult, @Suppress("UNUSED_PARAMETER") input: MPImage) {
        if (engine == 0L) {
            val size = imageSize
            engine = Native.engineNew(size.height.toFloat() / size.width, 0)
        }
        while (true) engineActions.poll()?.invoke(engine) ?: break

        val captureNs = result.timestampMs() * 1_000_000
        results.incrementAndGet()
        captureToResult.add((System.nanoTime() - captureNs) / 1e6)
        val points = result.landmarks().map { lm ->
            FloatArray(2 * HandFrame.LANDMARKS) { i -> if (i % 2 == 0) lm[i / 2].x() else lm[i / 2].y() }
        }
        val labels = result.handedness().map { it.firstOrNull()?.categoryName() ?: "" }
        val count = HandFrame.write(points, labels, hands)
        val found = Native.engineUpdate(engine, captureNs, hands, count, plucks)
        // The scene first, so Pluck listeners see the Strings the Pluck indices refer to.
        scene = RenderData.parse(render, Native.engineRender(engine, render))
        handPoints = (0 until count).map { h ->
            FloatArray(2 * HandFrame.LANDMARKS) { hands[h * HandFrame.HAND_FLOATS + 1 + it] }
        }
        if (found > 0) for (listen in pluckListeners) listen(plucks, minOf(found, Native.MAX_PLUCKS), captureNs)
        glView.requestRender()
    }

    override fun onResume() {
        super.onResume()
        glView.onResume()
    }

    override fun onPause() {
        glView.onPause()
        super.onPause()
    }

    override fun onDestroy() {
        super.onDestroy()
        ui.removeCallbacksAndMessages(null)
        landmarker?.close()
        landmarker = null
        analysisThread.shutdown()
        if (engine != 0L) Native.engineFree(engine)
        engine = 0L
    }

    private companion object {
        const val READOUT_MS = 250L
    }
}
