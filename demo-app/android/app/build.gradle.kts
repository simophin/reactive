plugins {
    id("com.android.application")
    kotlin("android") version "2.0.21"
    id("com.reactive")
}

android {
    namespace = "com.reactive.demo"
    compileSdk = 35

    defaultConfig {
        applicationId = "com.reactive.demo"
        minSdk = 26
        targetSdk = 35
        versionCode = 1
        versionName = "1.0"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    kotlinOptions {
        jvmTarget = "1.8"
    }
}

reactive {
    // Points to demo-app/android/ where the Rust crate lives
    rustProjectDir.set(file(".."))
    targets.set(listOf("arm64-v8a", "x86_64"))
}
