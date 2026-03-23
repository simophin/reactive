package com.reactive.gradle

import org.gradle.testfixtures.ProjectBuilder
import org.junit.jupiter.api.Test
import kotlin.test.assertEquals
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class ReactivePluginTest {

    @Test
    fun `plugin registers reactive extension`() {
        val project = ProjectBuilder.builder().build()
        project.plugins.apply(ReactivePlugin::class.java)

        assertNotNull(project.extensions.findByType(ReactiveExtension::class.java))
    }

    @Test
    fun `targets defaults to arm64-v8a`() {
        val project = ProjectBuilder.builder().build()
        project.plugins.apply(ReactivePlugin::class.java)

        val ext = project.extensions.getByType(ReactiveExtension::class.java)
        assertEquals(listOf("arm64-v8a"), ext.targets.get())
    }

    @Test
    fun `libName defaults to reactive_android`() {
        val project = ProjectBuilder.builder().build()
        project.plugins.apply(ReactivePlugin::class.java)

        val ext = project.extensions.getByType(ReactiveExtension::class.java)
        assertEquals("reactive_android", ext.libName.get())
    }

    @Test
    fun `no cargo tasks registered without android plugin`() {
        val project = ProjectBuilder.builder().build()
        project.plugins.apply(ReactivePlugin::class.java)

        val cargoTasks = project.tasks.names.filter { it.startsWith("cargoAndroidBuild") }
        assertTrue(cargoTasks.isEmpty())
    }
}
