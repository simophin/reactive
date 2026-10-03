plugins {
    id("com.android.library") version "9.3.3"
    `maven-publish`
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

    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

// `./gradlew publishToMavenLocal` publishes com.reactive:android-lib:0.1.0.
publishing {
    publications {
        register<MavenPublication>("release") {
            artifactId = "android-lib"
            afterEvaluate { from(components["release"]) }
            pom {
                name.set("Reactive Android support library")
                description.set("Java support classes (ReactiveActivity, ReactiveLayout, NativeCallback) for the reactive Rust UI framework's Android backend.")
                packaging = "aar"
            }
        }
    }
}
