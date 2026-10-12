// lingara-embed-java: the Java 17 facade over lingara-embed-kotlin (ADR
// 1.10.26ae D4). It adds only what Java cannot spell (a builder for
// Kotlin's default arguments, Optional, Consumer, one unchecked exception and
// a visitor for exhaustive dispatch) and passes the generated types through.
//
// Publishing is the Maven Central convention plugin (1.10.26ag D6a), whose
// artifact for this project is lingara-embed-java.

plugins {
    id("lingara.java-conventions")
    id("lingara.publishing-conventions")
}

description = "Lingara inside a JVM game: the Java facade over the Kotlin player kit."

dependencies {
    api(project(":kotlin"))
    testImplementation(libs.jna)
}

tasks.jar {
    manifest {
        attributes("Automatic-Module-Name" to "com.getlingara.embed.java")
    }
}

// The host library and the conformance server come from `make test-java`.
tasks.test {
    systemProperty("jna.library.path", rootProject.layout.projectDirectory.dir("target/debug").asFile.path)
    systemProperty("lingara.root", rootProject.projectDir.path)
}
