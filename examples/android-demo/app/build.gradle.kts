plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.plugin.compose")
    id("com.reactive")
}

android {
    namespace = "com.reactive.demo"
    compileSdk = 37
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "com.reactive.demo"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
    }

    buildTypes {
        release {
            // Signed with the debug key so the optimised build can be installed for profiling.
            signingConfig = signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

reactive {
    rustProjectDir.set(layout.projectDirectory.dir("../rust"))
    targets.set(listOf("arm64-v8a"))
}
