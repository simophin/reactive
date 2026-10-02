package com.reactive.gradle

import org.gradle.api.DefaultTask
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import java.io.File
import javax.inject.Inject

/**
 * Builds the Rust library for each Android ABI and copies the `.so` files into [outputDir].
 *
 * Cargo tracks its own dependencies (including path dependencies outside [rustProjectDir]),
 * so this task always runs and leaves incrementality to Cargo.
 */
abstract class CargoAndroidBuildTask @Inject constructor(
    private val execOperations: ExecOperations,
) : DefaultTask() {

    @get:Internal
    abstract val rustProjectDir: DirectoryProperty

    /** Cargo `--target-dir`, so the library location doesn't depend on workspace layout. */
    @get:Internal
    abstract val cargoTargetDir: DirectoryProperty

    @get:Input
    abstract val targets: ListProperty<String>

    @get:Input
    abstract val release: Property<Boolean>

    @get:Input
    abstract val libName: Property<String>

    @get:Input
    abstract val minSdk: Property<Int>

    /** NDK root directory. When absent the task falls back to ANDROID_NDK_HOME. */
    @get:Internal
    abstract val ndkDir: DirectoryProperty

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    init {
        outputs.upToDateWhen { false }
    }

    @TaskAction
    fun build() {
        val isRelease = release.get()
        val profile = if (isRelease) "release" else "debug"
        val targetDir = cargoTargetDir.get().asFile
        val outDir = outputDir.get().asFile
        val lib = libName.get()
        val ndk = resolvedNdkDir()

        for (abi in targets.get()) {
            val triple = tripleForAbi(abi)
            val args = mutableListOf(cargoExecutable(), "build", "--target", triple, "--target-dir", targetDir.absolutePath)
            if (isRelease) args.add("--release")

            execOperations.exec { spec ->
                spec.workingDir = rustProjectDir.get().asFile
                spec.commandLine = args
                if (ndk != null) {
                    spec.environment(linkerEnvVar(triple), ndkLinker(ndk, triple, minSdk.get()))
                }
            }

            val soFile = targetDir.resolve("$triple/$profile/lib$lib.so")
            val destDir = outDir.resolve(abi)
            destDir.mkdirs()
            soFile.copyTo(destDir.resolve("lib$lib.so"), overwrite = true)
        }
    }

    private fun cargoExecutable(): String {
        val home = File(System.getProperty("user.home"), ".cargo/bin/cargo")
        return if (home.canExecute()) home.absolutePath else "cargo"
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
        val clangTriple = if (triple == "armv7-linux-androideabi") "armv7a-linux-androideabi" else triple
        return ndkDir.resolve("toolchains/llvm/prebuilt/$hostTag/bin/${clangTriple}${minSdk}-clang")
            .absolutePath
    }
}
