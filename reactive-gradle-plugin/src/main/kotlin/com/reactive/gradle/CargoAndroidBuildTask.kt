package com.reactive.gradle

import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import java.io.File

abstract class CargoAndroidBuildTask : DefaultTask() {

    @get:InputDirectory
    abstract val rustProjectDir: DirectoryProperty

    @get:Input
    abstract val targets: ListProperty<String>

    @get:Input
    abstract val release: Property<Boolean>

    @get:Input
    abstract val libName: Property<String>

    @get:Input
    abstract val minSdk: Property<Int>

    /** NDK root directory. When absent the task falls back to ANDROID_NDK_HOME. */
    @get:InputDirectory
    @get:Optional
    abstract val ndkDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @TaskAction
    fun build() {
        val isRelease = release.get()
        val profile = if (isRelease) "release" else "debug"
        val rustDir = rustProjectDir.get().asFile
        val outDir = outputDir.get().asFile
        val lib = libName.get()
        val ndk = resolvedNdkDir()

        for (abi in targets.get()) {
            val triple = tripleForAbi(abi)
            val args = mutableListOf("cargo", "build", "--target", triple)
            if (isRelease) args.add("--release")

            project.exec { spec ->
                spec.workingDir = rustDir
                spec.commandLine = args
                if (ndk != null) {
                    spec.environment(linkerEnvVar(triple), ndkLinker(ndk, triple, minSdk.get()))
                }
            }

            val soFile = rustDir.resolve("target/$triple/$profile/lib$lib.so")
            val destDir = outDir.resolve(abi)
            destDir.mkdirs()
            soFile.copyTo(destDir.resolve("lib$lib.so"), overwrite = true)
        }
    }

    private fun resolvedNdkDir(): File? =
        ndkDir.orNull?.asFile
            ?: System.getenv("ANDROID_NDK_HOME")?.let { File(it) }

    private fun ndkLinker(ndkDir: File, triple: String, minSdk: Int): String {
        val hostTag = when {
            System.getProperty("os.name").startsWith("Mac")     -> "darwin-x86_64"
            System.getProperty("os.name").startsWith("Linux")   -> "linux-x86_64"
            else                                                 -> "windows-x86_64"
        }
        return ndkDir.resolve("toolchains/llvm/prebuilt/$hostTag/bin/${triple}${minSdk}-clang")
            .absolutePath
    }
}
