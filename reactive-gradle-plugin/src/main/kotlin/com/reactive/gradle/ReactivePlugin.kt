package com.reactive.gradle

import com.android.build.api.variant.AndroidComponentsExtension
import org.gradle.api.Plugin
import org.gradle.api.Project

/**
 * `com.reactive.android`: builds a Rust crate with cargo-ndk for each Android
 * variant, packages the resulting `.so` files, and adds the
 * `com.reactive:android-lib` support library.
 *
 * Apply together with `com.android.application` or `com.android.library`.
 */
class ReactivePlugin : Plugin<Project> {

    override fun apply(project: Project) {
        val ext = project.extensions.create("reactive", ReactiveExtension::class.java)
        ext.abis.convention(
            project.providers.gradleProperty("reactive.abis")
                .orElse(DEFAULT_ABI)
                .map(::parseAbis)
        )

        var configured = false
        val configure = {
            if (!configured) {
                configured = true
                configureAndroid(project, ext)
            }
        }
        project.pluginManager.withPlugin("com.android.application") { configure() }
        project.pluginManager.withPlugin("com.android.library") { configure() }

        project.afterEvaluate {
            if (!configured) {
                throw org.gradle.api.GradleException(
                    "Plugin com.reactive.android requires com.android.application or com.android.library " +
                        "to be applied to ${project.path}."
                )
            }
        }
    }

    private fun configureAndroid(project: Project, ext: ReactiveExtension) {
        val androidComponents = project.extensions.getByType(AndroidComponentsExtension::class.java)

        // AGP's NDK, used as ANDROID_NDK_HOME if the environment lacks one. Resolution may fail
        // (for example no NDK installed); cargo-ndk then falls back to its own lookup.
        val ndkDir = project.provider {
            runCatching { androidComponents.sdkComponents.ndkDirectory.orNull?.asFile?.absolutePath }.getOrNull()
        }

        androidComponents.onVariants(androidComponents.selector().all()) { variant ->
            val name = variant.name.replaceFirstChar { it.uppercase() }
            val task = project.tasks.register("cargoNdkBuild$name", CargoNdkBuild::class.java) {
                crateDir.set(ext.crateDir)
                packageName.set(ext.packageName)
                abis.set(ext.abis)
                platform.set(variant.minSdk.apiLevel)
                debuggable.set(variant.debuggable)
                cargoProfile.set(ext.cargoProfile)
                this.ndkDir.set(ndkDir)
                outputDir.set(project.layout.buildDirectory.dir("reactive/jniLibs/${variant.name}"))
            }
            variant.sources.jniLibs?.addGeneratedSourceDirectory(task, CargoNdkBuild::outputDir)
        }

        project.dependencies.add("implementation", "com.reactive:android-lib:$PLUGIN_VERSION")
    }
}
