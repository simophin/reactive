package com.reactive.gradle

import com.android.build.api.variant.AndroidComponentsExtension
import com.android.build.gradle.BaseExtension
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property

abstract class ReactiveExtension {
    abstract val rustProjectDir: DirectoryProperty
    abstract val targets: ListProperty<String>
    abstract val libName: Property<String>
}

class ReactivePlugin : Plugin<Project> {

    override fun apply(project: Project) {
        val ext = project.extensions.create("reactive", ReactiveExtension::class.java)
        ext.targets.convention(listOf("arm64-v8a"))
        ext.libName.convention("reactive_android")

        project.plugins.withId("com.android.application") { configureAndroid(project, ext) }
        project.plugins.withId("com.android.library") { configureAndroid(project, ext) }
    }

    private fun configureAndroid(project: Project, ext: ReactiveExtension) {
        val androidComponents = project.extensions.getByType(AndroidComponentsExtension::class.java)
        val androidBase = project.extensions.getByType(BaseExtension::class.java)

        androidComponents.onVariants { variant ->
            val variantName = variant.name.replaceFirstChar { it.uppercase() }
            val outputDir = project.layout.buildDirectory.dir("rustJniLibs/$variantName")

            val cargoTask = project.tasks.register(
                "cargoAndroidBuild$variantName",
                CargoAndroidBuildTask::class.java,
            ) { task ->
                task.rustProjectDir.set(ext.rustProjectDir)
                task.targets.set(ext.targets)
                task.libName.set(ext.libName)
                task.release.set(variant.buildType == "release")
                task.minSdk.set(variant.minSdk.apiLevel)
                task.ndkDir.set(project.layout.dir(project.provider { androidBase.ndkDirectory }))
                task.outputDir.set(outputDir)
            }

            variant.sources.jniLibs?.addGeneratedSourceDirectory(cargoTask, CargoAndroidBuildTask::outputDir)
        }

        project.dependencies.add("implementation", "com.reactive:android-lib:$ANDROID_LIB_VERSION")
    }

    companion object {
        const val ANDROID_LIB_VERSION = "0.1.0"
    }
}
