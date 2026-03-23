package com.reactive.gradle

import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import kotlin.test.assertEquals

class AbiMappingTest {

    @Test
    fun `arm64-v8a maps to aarch64-linux-android`() {
        assertEquals("aarch64-linux-android", tripleForAbi("arm64-v8a"))
    }

    @Test
    fun `armeabi-v7a maps to armv7-linux-androideabi`() {
        assertEquals("armv7-linux-androideabi", tripleForAbi("armeabi-v7a"))
    }

    @Test
    fun `x86_64 maps to x86_64-linux-android`() {
        assertEquals("x86_64-linux-android", tripleForAbi("x86_64"))
    }

    @Test
    fun `x86 maps to i686-linux-android`() {
        assertEquals("i686-linux-android", tripleForAbi("x86"))
    }

    @Test
    fun `unknown ABI throws with helpful message`() {
        val ex = assertThrows<IllegalStateException> { tripleForAbi("mips") }
        assert(ex.message!!.contains("mips"))
        assert(ex.message!!.contains("Supported:"))
    }

    @Test
    fun `all expected ABIs are present`() {
        val expected = setOf("arm64-v8a", "armeabi-v7a", "x86_64", "x86")
        assertEquals(expected, ABI_TO_TRIPLE.keys)
    }

    @Test
    fun `linkerEnvVar produces correct cargo env var name`() {
        assertEquals("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",   linkerEnvVar("aarch64-linux-android"))
        assertEquals("CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",  linkerEnvVar("armv7-linux-androideabi"))
        assertEquals("CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",     linkerEnvVar("x86_64-linux-android"))
        assertEquals("CARGO_TARGET_I686_LINUX_ANDROID_LINKER",       linkerEnvVar("i686-linux-android"))
    }

    @Test
    fun `linkerEnvVar round-trips through tripleForAbi for all ABIs`() {
        val expected = mapOf(
            "arm64-v8a"   to "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
            "armeabi-v7a" to "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
            "x86_64"      to "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
            "x86"         to "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
        )
        for ((abi, envVar) in expected) {
            assertEquals(envVar, linkerEnvVar(tripleForAbi(abi)))
        }
    }
}
