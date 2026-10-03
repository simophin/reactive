plugins {
    `kotlin-dsl`
    `maven-publish`
}

group = "com.reactive"
version = "0.1.0"

repositories {
    google()
    mavenCentral()
}

dependencies {
    // Only the public variant API is needed; the consumer supplies AGP at runtime.
    compileOnly("com.android.tools.build:gradle-api:9.3.3")

    testImplementation("com.android.tools.build:gradle-api:9.3.3")
    testImplementation(gradleTestKit())
    testImplementation("org.junit.jupiter:junit-jupiter:5.11.4")
    testRuntimeOnly("org.junit.platform:junit-platform-launcher")
}

// AGP 9 runs on JDK 17+, so target 17 bytecode.
java {
    sourceCompatibility = JavaVersion.VERSION_17
    targetCompatibility = JavaVersion.VERSION_17
}

kotlin {
    compilerOptions.jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17)
}

gradlePlugin {
    plugins {
        create("reactiveAndroid") {
            id = "com.reactive.android"
            displayName = "Reactive Android"
            description = "Builds a Rust crate with cargo-ndk and packages it with the com.reactive Android support library."
            implementationClass = "com.reactive.gradle.ReactivePlugin"
        }
    }
}

// Expose the plugin version at runtime so it can add the matching android-lib dependency.
val generateVersionSource = tasks.register("generateVersionSource") {
    val outDir = layout.buildDirectory.dir("generated/reactiveVersion")
    val pluginVersion = project.version.toString()
    inputs.property("version", pluginVersion)
    outputs.dir(outDir)
    doLast {
        val file = outDir.get().file("com/reactive/gradle/PluginVersion.kt").asFile
        file.parentFile.mkdirs()
        file.writeText(
            """
            |package com.reactive.gradle
            |
            |internal const val PLUGIN_VERSION = "$pluginVersion"
            |""".trimMargin()
        )
    }
}

kotlin.sourceSets.named("main") {
    kotlin.srcDir(generateVersionSource)
}

tasks.test {
    useJUnitPlatform()
}
