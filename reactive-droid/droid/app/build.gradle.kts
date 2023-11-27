plugins {
    id("com.android.application")
    id("org.mozilla.rust-android-gradle.rust-android") version "0.9.3"
//    id("io.github.MatrixDev.android-rust") version "0.3.2"
}

android {
    namespace = "dev.fanchao.reactive"
    compileSdk = 34
    ndkVersion = "26.1.10909125"

    defaultConfig {
        applicationId = "dev.fanchao.reactive"
        minSdk = 21
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        vectorDrawables {
            useSupportLibrary = true
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
        }

        debug {
            packaging {
                jniLibs {
                    keepDebugSymbols += "**/*.so"
                }
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
        }
    }
}


cargo {
    module  = "../../../"       // Or whatever directory contains your Cargo.toml
    libname = "reactive_droid"          // Or whatever matches Cargo.toml's [package] name.
    targets = listOf("arm64", "arm")  // See bellow for a longer list of options
    pythonCommand = "python3" // Optional, defaults to "python"
    targetIncludes = arrayOf("libreactive_droid.so")
    targetDirectory = "../../../target"
    profile = "debug"
}

tasks.whenTaskAdded {
    if ((name.contains("Jni"))) {
        dependsOn("cargoBuild")
        inputs.dir(buildDir.resolve("rustJniLibs/android"))
    }
}

dependencies {
//    implementation("androidx.activity:activity:1.8.0")
//    testImplementation("junit:junit:4.13.2")
//    androidTestImplementation("androidx.test.ext:junit:1.1.5")
//    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
}