plugins {
    id("com.android.library") version "9.0.0"
    kotlin("android") version "2.0.21"
}

group = "com.reactive"
version = "0.1.0"

android {
    namespace = "com.reactive"
    compileSdk = 35

    defaultConfig {
        minSdk = 26
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    kotlinOptions {
        jvmTarget = "1.8"
    }
}
