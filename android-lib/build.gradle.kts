plugins {
    id("com.android.library") version "9.3.3"
}

group = "com.reactive"
version = "0.1.0"

android {
    namespace = "com.reactive"
    compileSdk = 36

    defaultConfig {
        minSdk = 26
        consumerProguardFiles("consumer-rules.pro")
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
