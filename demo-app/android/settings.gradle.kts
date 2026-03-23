pluginManagement {
    includeBuild("../../reactive-gradle-plugin")
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

// Substitutes com.reactive:android-lib with the local project
includeBuild("../../android-lib")

rootProject.name = "demo-app-android"
include(":app")
