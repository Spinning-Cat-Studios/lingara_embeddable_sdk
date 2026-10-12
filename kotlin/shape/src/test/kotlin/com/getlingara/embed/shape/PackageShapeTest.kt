package com.getlingara.embed.shape

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PackageShapeTest {
    // 1.10.26ae AC8: -kotlin depends at runtime only on kotlin-stdlib (no JNA,
    // no kotlinx-*, no server kit, no lingara-kotlin); -java only on -kotlin;
    // each natives artefact on its own JNA variant.
    @Test
    fun eachArtefactDependsOnlyOnWhatD1Says() {
        val kotlin = Artefact("lingara-embed-kotlin").runtimeDependencies()
        assertEquals(listOf("org.jetbrains.kotlin:kotlin-stdlib"), kotlin.map { it.coordinate })
        val forbidden = listOf("net.java.dev.jna", "kotlinx", "lingara-embed-server", "lingara-kotlin", "lingara-java")
        assertTrue(kotlin.none { dep -> forbidden.any { it in dep.coordinate } }, "$kotlin")

        val java = Artefact("lingara-embed-java").runtimeDependencies()
        assertEquals(listOf("com.getlingara:lingara-embed-kotlin"), java.map { it.coordinate })

        val desktop = Artefact("lingara-embed-natives-desktop").runtimeDependencies()
        assertEquals(listOf("net.java.dev.jna:jna"), desktop.map { it.coordinate })
        assertTrue(desktop.single().type.let { it == null || it == "jar" }, "the plain JAR")

        val android = Artefact("lingara-embed-natives-android").runtimeDependencies()
        assertEquals(listOf("net.java.dev.jna:jna"), android.map { it.coordinate })
        assertEquals("aar", android.single().type, "JNA's AAR")
    }
}
