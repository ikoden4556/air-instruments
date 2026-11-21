// PROTOTYPE: throwaway phase-2 latency spike.
plugins { id("com.android.application") }

android {
    namespace = "com.airinstruments.spike"
    compileSdk = 37
    defaultConfig {
        applicationId = "com.airinstruments.spike"
        minSdk = 29
        targetSdk = 37
        versionCode = 1
        versionName = "spike"
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
}
