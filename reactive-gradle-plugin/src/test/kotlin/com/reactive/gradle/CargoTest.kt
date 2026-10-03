package com.reactive.gradle

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNull
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import org.junit.jupiter.api.io.TempDir
import java.io.File

class CargoTest {

    @Test
    fun `parses comma separated ABIs`() {
        assertEquals(listOf("arm64-v8a", "x86_64"), parseAbis(" arm64-v8a, x86_64 ,"))
    }

    @Test
    fun `rejects unknown and empty ABIs`() {
        validateAbis(SUPPORTED_ABIS.keys.toList())
        assertThrows<IllegalArgumentException> { validateAbis(listOf("arm64")) }
        assertThrows<IllegalArgumentException> { validateAbis(emptyList()) }
    }

    @Test
    fun `profile follows debuggable unless overridden`() {
        assertEquals(emptyList<String>(), profileArgs(null, debuggable = true))
        assertEquals(listOf("--release"), profileArgs(null, debuggable = false))
        assertEquals(listOf("--release"), profileArgs("release", debuggable = true))
        assertEquals(emptyList<String>(), profileArgs("dev", debuggable = false))
        assertEquals(listOf("--profile", "small"), profileArgs("small", debuggable = true))
    }

    @Test
    fun `builds cargo ndk command line`() {
        val out = File("/tmp/out")
        assertEquals(
            listOf(
                "ndk", "--platform", "26", "-t", "arm64-v8a", "-t", "x86_64", "-o", out.absolutePath,
                "build", "-p", "my-app", "--lib", "--release",
            ),
            cargoNdkArgs(26, listOf("arm64-v8a", "x86_64"), out, "my-app", listOf("--release")),
        )
    }

    private fun executable(dir: File, name: String): File =
        File(dir, name).apply {
            parentFile.mkdirs()
            writeText("#!/bin/sh\n")
            setExecutable(true)
        }

    @Test
    fun `finds cargo in CARGO, then PATH, then cargo home`(@TempDir tmp: File) {
        val home = File(tmp, "home")
        val homeCargo = executable(File(home, ".cargo/bin"), "cargo")
        val pathDir = File(tmp, "path")

        assertNull(CargoToolchain(mapOf("PATH" to pathDir.path), File(tmp, "nobody"), false).findCargo())
        assertEquals(homeCargo, CargoToolchain(mapOf("PATH" to pathDir.path), home, false).findCargo())

        val pathCargo = executable(pathDir, "cargo")
        assertEquals(pathCargo, CargoToolchain(mapOf("PATH" to pathDir.path), home, false).findCargo())

        val explicit = executable(File(tmp, "explicit"), "cargo")
        assertEquals(
            explicit,
            CargoToolchain(mapOf("PATH" to pathDir.path, "CARGO" to explicit.path), home, false).findCargo(),
        )
    }

    @Test
    fun `finds cargo-ndk in cargo home when cargo comes from elsewhere`(@TempDir tmp: File) {
        val home = File(tmp, "home")
        val cargoNdk = executable(File(home, ".cargo/bin"), "cargo-ndk")
        val cargo = executable(File(tmp, "toolchain/bin"), "cargo")
        val toolchain = CargoToolchain(mapOf("PATH" to "/usr/bin"), home, false)

        assertEquals(cargoNdk, toolchain.findCargoNdk(cargo))
        assertEquals(
            listOf(cargo.parentFile.path, cargoNdk.parentFile.path, "/usr/bin").joinToString(File.pathSeparator),
            toolchain.childPath(cargo, cargoNdk),
        )
        assertNull(CargoToolchain(mapOf("PATH" to "/nonexistent"), File(tmp, "nobody"), false).findCargoNdk(cargo))
    }

    @Test
    fun `CARGO_HOME overrides the default cargo bin dir`() {
        val toolchain = CargoToolchain(mapOf("CARGO_HOME" to "/opt/cargo"), File("/home/u"), false)
        assertEquals(File("/opt/cargo/bin"), toolchain.cargoBinDir)
    }
}
