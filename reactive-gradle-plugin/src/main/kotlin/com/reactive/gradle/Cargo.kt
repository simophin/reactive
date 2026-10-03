package com.reactive.gradle

import java.io.File

/** ABIs cargo-ndk understands, with their Rust target triples. */
internal val SUPPORTED_ABIS: Map<String, String> = linkedMapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "armeabi-v7a" to "armv7-linux-androideabi",
    "x86_64" to "x86_64-linux-android",
    "x86" to "i686-linux-android",
)

internal const val DEFAULT_ABI = "arm64-v8a"

/** Parses a comma-separated ABI list such as the `reactive.abis` Gradle property. */
internal fun parseAbis(value: String): List<String> =
    value.split(',').map(String::trim).filter(String::isNotEmpty)

internal fun validateAbis(abis: List<String>) {
    require(abis.isNotEmpty()) { "reactive.abis is empty; set at least one ABI (for example arm64-v8a)." }
    val unknown = abis.filterNot(SUPPORTED_ABIS::containsKey)
    require(unknown.isEmpty()) {
        "Unknown Android ABI(s) ${unknown.joinToString()}. Supported: ${SUPPORTED_ABIS.keys.joinToString()}."
    }
}

/** Cargo flags selecting the build profile. */
internal fun profileArgs(cargoProfile: String?, debuggable: Boolean): List<String> =
    when (cargoProfile?.trim()?.ifEmpty { null }) {
        null -> if (debuggable) emptyList() else listOf("--release")
        "dev", "debug" -> emptyList()
        "release" -> listOf("--release")
        else -> listOf("--profile", cargoProfile.trim())
    }

/** Arguments after the cargo executable. */
internal fun cargoNdkArgs(
    platform: Int,
    abis: List<String>,
    outputDir: File,
    packageName: String,
    profileArgs: List<String>,
): List<String> = buildList {
    add("ndk")
    add("--platform"); add(platform.toString())
    abis.forEach { add("-t"); add(it) }
    add("-o"); add(outputDir.absolutePath)
    add("build")
    add("-p"); add(packageName)
    add("--lib")
    addAll(profileArgs)
}

/** Locates cargo and cargo-ndk without relying on the IDE's PATH. */
internal class CargoToolchain(
    private val env: Map<String, String>,
    private val userHome: File,
    private val isWindows: Boolean = System.getProperty("os.name").startsWith("Windows"),
) {
    private fun exe(name: String) = if (isWindows) "$name.exe" else name

    private val pathDirs: List<File> =
        env["PATH"].orEmpty().split(File.pathSeparatorChar).filter(String::isNotEmpty).map(::File)

    /** `$CARGO_HOME/bin` (or `~/.cargo/bin`), where rustup and `cargo install` put binaries. */
    val cargoBinDir: File =
        env["CARGO_HOME"]?.takeIf(String::isNotEmpty)?.let { File(it, "bin") }
            ?: File(userHome, ".cargo/bin")

    private fun File.isExecutableFile() = isFile && (isWindows || canExecute())

    /** `CARGO`, then `cargo` on PATH, then `$CARGO_HOME/bin` / `~/.cargo/bin`. */
    fun findCargo(): File? {
        env["CARGO"]?.takeIf(String::isNotEmpty)?.let(::File)?.takeIf { it.isExecutableFile() }?.let { return it }
        return (pathDirs + cargoBinDir).map { File(it, exe("cargo")) }.firstOrNull { it.isExecutableFile() }
    }

    fun findCargoNdk(cargo: File): File? =
        (listOfNotNull(cargo.parentFile) + pathDirs + cargoBinDir)
            .map { File(it, exe("cargo-ndk")) }
            .firstOrNull { it.isExecutableFile() }

    /** PATH for the cargo child process, with cargo's and cargo-ndk's directories first. */
    fun childPath(vararg tools: File): String =
        (tools.mapNotNull { it.parentFile } + cargoBinDir + pathDirs)
            .map { it.path }
            .distinct()
            .joinToString(File.pathSeparator)
}
