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

includeBuild("../../android-lib")

rootProject.name = "android-demo"
include(":app")
