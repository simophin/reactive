package com.reactive.gradle

import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.TaskAction
import org.gradle.process.ExecOperations
import org.gradle.work.DisableCachingByDefault
import java.io.File
import javax.inject.Inject

/** Builds a Rust cdylib with `cargo ndk` into a jniLibs layout (`<outputDir>/<abi>/lib*.so`). */
@DisableCachingByDefault(because = "Cargo does its own change tracking")
abstract class CargoNdkBuild : DefaultTask() {
    /** Directory cargo runs in. Not an input: cargo tracks its own sources. */
    @get:Internal
    abstract val crateDir: DirectoryProperty

    @get:Input
    abstract val packageName: Property<String>

    @get:Input
    abstract val abis: ListProperty<String>

    /** Passed to `cargo ndk --platform`. */
    @get:Input
    abstract val platform: Property<Int>

    @get:Input
    abstract val debuggable: Property<Boolean>

    @get:Input
    @get:Optional
    abstract val cargoProfile: Property<String>

    /** NDK resolved by AGP; exported as ANDROID_NDK_HOME unless the environment already sets it. */
    @get:Input
    @get:Optional
    abstract val ndkDir: Property<String>

    @get:OutputDirectory
    abstract val outputDir: DirectoryProperty

    @get:Inject
    abstract val execOps: ExecOperations

    init {
        group = "build"
        description = "Builds the Rust library with cargo-ndk."
        // Cargo does its own change tracking.
        outputs.upToDateWhen { false }
    }

    @TaskAction
    fun build() {
        if (!crateDir.isPresent) throw GradleException("reactive.crateDir is not set. Point it at the crate or workspace root.")
        if (!packageName.isPresent) throw GradleException("reactive.packageName is not set. Set it to the cargo package to build.")
        val abiList = abis.get()
        try {
            validateAbis(abiList)
        } catch (e: IllegalArgumentException) {
            throw GradleException(e.message ?: "Invalid ABIs", e)
        }

        val env = System.getenv()
        val toolchain = CargoToolchain(env, File(System.getProperty("user.home")))
        val cargo = toolchain.findCargo() ?: throw GradleException(
            "Could not find cargo. Install Rust from https://rustup.rs, or set the CARGO environment " +
                "variable to the cargo executable. Searched \$CARGO, PATH and ${toolchain.cargoBinDir}."
        )
        val cargoNdk = toolchain.findCargoNdk(cargo) ?: throw GradleException(
            "Could not find cargo-ndk. Install it with `cargo install cargo-ndk`, and add the Android " +
                "targets with `rustup target add ${abiList.joinToString(" ") { SUPPORTED_ABIS.getValue(it) }}`. " +
                "Searched ${cargo.parentFile}, PATH and ${toolchain.cargoBinDir}."
        )

        val out = outputDir.get().asFile
        // Drop libraries for ABIs that are no longer requested.
        out.deleteRecursively()
        out.mkdirs()

        val args = cargoNdkArgs(
            platform = platform.get(),
            abis = abiList,
            outputDir = out,
            packageName = packageName.get(),
            profileArgs = profileArgs(cargoProfile.orNull, debuggable.get()),
        )
        logger.info("Running {} {} in {}", cargo, args.joinToString(" "), crateDir.get().asFile)

        execOps.exec {
            workingDir = crateDir.get().asFile
            executable = cargo.absolutePath
            args(args)
            environment("PATH", toolchain.childPath(cargo, cargoNdk))
            val ndk = ndkDir.orNull
            if (ndk != null && env["ANDROID_NDK_HOME"].isNullOrEmpty() && File(ndk).isDirectory) {
                environment("ANDROID_NDK_HOME", ndk)
            }
        }
    }
}
