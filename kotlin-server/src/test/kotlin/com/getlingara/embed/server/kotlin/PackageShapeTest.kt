package com.getlingara.embed.server.kotlin

import org.w3c.dom.Element
import java.io.File
import javax.xml.parsers.DocumentBuilderFactory
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse

class PackageShapeTest {
    /**
     * 1.10.26af AC26: the generated POM's only dependency is lingara-kotlin (kotlin-stdlib,
     * coroutines and kotlinx-serialization come through it), and no player artifact
     * (lingara-embed-kotlin, the natives) appears.
     */
    @Test
    fun onlyTheLibraryIsRequired() {
        val factory = DocumentBuilderFactory.newInstance()
        factory.setFeature("http://apache.org/xml/features/disallow-doctype-decl", true)
        val pom = File(System.getProperty("lingara.pom"))
        val document = factory.newDocumentBuilder().parse(pom)
        val deps = document.getElementsByTagName("dependency")
        val coordinates =
            (0 until deps.length).map { i ->
                val d = deps.item(i) as Element
                "${text(d, "groupId")}:${text(d, "artifactId")}:${text(d, "scope")}"
            }
        assertEquals(listOf("com.getlingara:lingara-kotlin:compile"), coordinates)
        assertEquals("lingara-embed-server-kotlin", text(document.documentElement, "artifactId"))
        val text = pom.readText()
        assertFalse("lingara-embed-kotlin" in text || "lingara-embed-natives" in text)
    }

    private fun text(
        parent: Element,
        tag: String,
    ): String =
        parent
            .getElementsByTagName(tag)
            .item(0)
            ?.textContent
            ?.trim() ?: ""
}
