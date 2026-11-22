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

tasks.named("preBuild") { dependsOn(rustLibs, handModel) }
