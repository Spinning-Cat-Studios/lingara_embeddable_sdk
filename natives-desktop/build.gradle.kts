// lingara-embed-natives-desktop: the Windows, macOS and Linux libraries
// where JNA looks for them (ADR 1.10.26ae D1, D5). Its resources are the
// union of the three desktop dist/jvm/<rid>/ directories make/kotlin.mk's
// natives-desktop writes, each under JNA's own resource prefix
// (win32-x86-64/, darwin-aarch64/, darwin-x86-64/, linux-x86-64/), so JNA
// extracts and loads the right one with no code of ours.
//
// JNA's plain JAR is `api`: the generated EmbedHandle has a public
// constructor taking JNA's Pointer, so a game must see JNA at compile time,
// and this artefact is where JNA's desktop variant comes from.
//
// Publishing is the Maven Central convention plugin (1.10.26ag D6a), whose
// artifact for this project is lingara-embed-natives-desktop.

plugins {
    id("lingara.java-conventions")
    id("lingara.publishing-conventions")
}

description = "Lingara inside a JVM game: the desktop native libraries for lingara-embed-kotlin."

val dist = rootProject.layout.projectDirectory.dir("dist/jvm")

sourceSets {
    main {
        resources.srcDirs(dist.dir("win-x64"), dist.dir("osx"), dist.dir("linux-x64"))
    }
}

dependencies {
    api(libs.jna)
}
