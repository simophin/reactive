package com.reactive.gradle

import org.gradle.testfixtures.ProjectBuilder
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class ReactivePluginTest {

    @Test
    fun `creates extension with default ABI`() {
        val project = ProjectBuilder.builder().build()
        project.pluginManager.apply("com.reactive.android")
        val ext = project.extensions.getByType(ReactiveExtension::class.java)
        assertEquals(listOf(DEFAULT_ABI), ext.abis.get())
    }

}
