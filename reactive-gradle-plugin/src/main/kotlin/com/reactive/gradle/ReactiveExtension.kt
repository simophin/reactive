package com.reactive.gradle

import org.gradle.api.file.DirectoryProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property

/**
 * Configuration for the `com.reactive.android` plugin.
 *
 * ```kotlin
 * reactive {
 *     crateDir.set(file("../.."))     // where cargo runs: the crate or workspace root
 *     packageName.set("my-app")       // cargo package passed as `-p`
 *     abis.set(listOf("arm64-v8a"))   // optional
 *     cargoProfile.set("release")     // optional
 * }
 * ```
 */
abstract class ReactiveExtension {
    /** Directory to run cargo in: the crate or its workspace root. Required. */
    abstract val crateDir: DirectoryProperty

    /** Cargo package that produces the cdylib, passed as `-p`. Required. */
    abstract val packageName: Property<String>

    /**
     * Android ABIs to build. Defaults to the comma-separated Gradle property
     * `reactive.abis`, or `arm64-v8a` when that is unset.
     */
    abstract val abis: ListProperty<String>

    /**
     * Cargo profile for every variant. When unset, debuggable variants use the
     * dev profile and the others use `--release`. `dev`/`debug` and `release`
     * map to cargo's defaults; any other name is passed as `--profile <name>`.
     */
    abstract val cargoProfile: Property<String>
}
