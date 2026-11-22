import java.util.zip.ZipFile

plugins { id("com.android.application") }

android {
    namespace = "com.airinstruments"
    compileSdk = 37
    defaultConfig {
        applicationId = "com.airinstruments"
        minSdk = 29
        targetSdk = 37
        versionCode = 1
        versionName = "0.1"
        ndk { abiFilters += "arm64-v8a" }
    }
    androidResources { noCompress += "task" }
}

dependencies {
    val camerax = "1.7.0"
    implementation("androidx.camera:camera-camera2:$camerax")
    implementation("androidx.camera:camera-lifecycle:$camerax")
    implementation("androidx.camera:camera-view:$camerax")
    implementation("androidx.activity:activity:1.13.0")
    implementation("com.google.mediapipe:tasks-vision:1.2.3")
    testImplementation("junit:junit:4.13.2")
}

// The Rust core and Android bindings, built into jniLibs/ before every build (needs cargo-ndk).
val rustLibs = tasks.register<Exec>("buildRust") {
    workingDir = rootProject.file("../bindings/android")
    commandLine(
        "cargo", "ndk", "-t", "arm64-v8a", "-P", "29",
        "-o", project.file("src/main/jniLibs").absolutePath, "build", "--release",
    )
}

// MediaPipe's hand model, downloaded once into assets/ (the app itself has no internet permission).
val handModel = tasks.register("fetchHandModel") {
    val model = project.file("src/main/assets/hand_landmarker.task")
    outputs.file(model)
    onlyIf { !model.exists() }
    doLast {
        model.parentFile.mkdirs()
        uri("https://storage.googleapis.com/mediapipe-models/hand_landmarker/hand_landmarker/float16/latest/hand_landmarker.task")
            .toURL().openStream().use { input -> model.outputStream().use { input.copyTo(it) } }
    }
}

// The same bundle with MediaPipe's lite landmark model (#48), given the full model's metadata by
// core/examples/lite_hand_model.rs. Picked at start with `--es model hand_landmarker_lite.task`.
val liteDir = layout.buildDirectory.dir("liteHandModel")
val liteLandmarks = tasks.register("liteLandmarks") {
    dependsOn(handModel)
    val out = liteDir.map { it.file("bundle/hand_landmarks_detector.tflite") }
    outputs.file(out)
    onlyIf { !out.get().asFile.exists() }
    doLast {
        val dir = liteDir.get().asFile.apply { mkdirs() }
        val lite = File(dir, "hand_landmark_lite.tflite")
        uri("https://storage.googleapis.com/mediapipe-assets/hand_landmark_lite.tflite")
            .toURL().openStream().use { input -> lite.outputStream().use { input.copyTo(it) } }
        val full = File(dir, "full.tflite")
        val bundle = ZipFile(project.file("src/main/assets/hand_landmarker.task"))
        bundle.getInputStream(bundle.getEntry("hand_landmarks_detector.tflite")).use { input -> full.outputStream().use { input.copyTo(it) } }
        bundle.close()
        out.get().asFile.parentFile.mkdirs()
        providers.exec {
            workingDir = rootProject.file("../core")
            commandLine("cargo", "run", "-q", "--example", "lite_hand_model", "--", full.path, lite.path, out.get().asFile.path)
        }.result.get().assertNormalExitValue()
    }
}
val liteHandModel = tasks.register<Zip>("liteHandModel") {
    dependsOn(liteLandmarks)
    from(liteDir.map { it.file("bundle/hand_landmarks_detector.tflite") })
    from(zipTree("src/main/assets/hand_landmarker.task")) { include("hand_detector.tflite") }
    entryCompression = ZipEntryCompression.STORED // MediaPipe refuses compressed bundles
    destinationDirectory = project.file("src/main/assets")
    archiveFileName = "hand_landmarker_lite.task"
}

tasks.named("preBuild") { dependsOn(rustLibs, handModel, liteHandModel) }
