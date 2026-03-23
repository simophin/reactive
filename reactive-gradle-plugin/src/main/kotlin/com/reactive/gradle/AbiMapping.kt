package com.reactive.gradle

internal val ABI_TO_TRIPLE: Map<String, String> = mapOf(
    "arm64-v8a"   to "aarch64-linux-android",
    "armeabi-v7a" to "armv7-linux-androideabi",
    "x86_64"      to "x86_64-linux-android",
    "x86"         to "i686-linux-android",
)

internal fun tripleForAbi(abi: String): String =
    ABI_TO_TRIPLE[abi] ?: error("Unknown ABI '$abi'. Supported: ${ABI_TO_TRIPLE.keys.joinToString()}")

/** Cargo environment variable that overrides the linker for a given target triple. */
internal fun linkerEnvVar(triple: String): String =
    "CARGO_TARGET_${triple.uppercase().replace('-', '_').replace('.', '_')}_LINKER"
