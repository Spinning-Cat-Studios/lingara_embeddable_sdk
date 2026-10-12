package com.getlingara.embed.shape

import org.w3c.dom.Element
import java.io.File
import java.util.zip.ZipFile
import javax.xml.parsers.DocumentBuilderFactory

/** One published artefact in the local repository make check-jvm-shape fills. */
internal class Artefact(
    val name: String,
) {
    private val version: String = System.getProperty("lingara.version")
    private val dir = File(System.getProperty("lingara.m2"), "com/getlingara/$name/$version")

    fun file(extension: String): File =
        File(dir, "$name-$version.$extension").also { check(it.isFile) { "no $it: run make check-jvm-shape" } }

    /** The POM's dependencies a consumer resolves at runtime: compile and runtime scope. */
    fun runtimeDependencies(): List<Dependency> {
        val pom = DocumentBuilderFactory.newInstance().newDocumentBuilder().parse(file("pom"))
        val nodes = pom.getElementsByTagName("dependency")
        return (0 until nodes.length)
            .map { nodes.item(it) as Element }
            .map { Dependency(it.text("groupId"), it.text("artifactId"), it.text("type"), it.text("scope") ?: "compile") }
            .filter { it.scope == "compile" || it.scope == "runtime" }
    }

    fun <T> zip(
        extension: String,
        read: (ZipFile) -> T,
    ): T = ZipFile(file(extension)).use(read)
}

internal data class Dependency(
    val group: String?,
    val artifact: String?,
    val type: String?,
    val scope: String,
) {
    val coordinate: String get() = "$group:$artifact"
}

private fun Element.text(tag: String): String? =
    getElementsByTagName(tag)
        .takeIf { it.length > 0 }
        ?.item(0)
        ?.textContent
        ?.trim()
