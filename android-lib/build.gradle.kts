plugins {
    id("com.android.library") version "9.3.3"
    id("org.jetbrains.kotlin.plugin.compose") version "2.4.20"
}

group = "com.reactive"
version = "0.1.0"

repositories {
    google()
    mavenCentral()
}

android {
    namespace = "com.reactive"
    compileSdk = 37

    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2026.09.00")
    api(composeBom)
    api("androidx.compose.runtime:runtime")
    api("androidx.compose.ui:ui")
    api("androidx.compose.foundation:foundation")
    api("androidx.compose.material3:material3")
    api("androidx.activity:activity-compose:1.13.0")
}
