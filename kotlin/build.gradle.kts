// lingara-embed-kotlin: the JVM player kit (ADR 1.10.26ae D1, D3).
//
// The generated UniFFI bindings and the idiomatic layer, both in package
// com.getlingara.embed. It holds no native library and no runtime JNA: JNA
// is compileOnly here, and each natives artefact brings its own variant (the
// plain JAR on a desktop, the AAR on Android), so the wrong JNA never sits in
// front of either.
//
// Publishing is the Maven Central convention plugin (1.10.26ag D6a), whose
// artifact for this project is lingara-embed-kotlin.

plugins {
    id("lingara.kotlin-conventions")
    id("lingara.publishing-conventions")
}

description = "Lingara inside a JVM game: the Kotlin player kit over the native core."

val generated = "src/main/kotlin/com/getlingara/embed/lingara_embed_uniffi.kt"

// UniFFI writes the bindings; neither ktlint nor a hand edit touches them,
// and `make check-codegen-kotlin` holds them byte-equal to a fresh regen.
spotless {
    kotlin {
        targetExclude(generated)
    }
}

kotlin {
    compilerOptions {
        // The generated file's one warning, suppressed by name as the
        // convention asks: uniffiEnsureInitialized() names two objects
        // as bare expressions to force their initialisation.
        freeCompilerArgs.add("-Xwarning-level=UNUSED_EXPRESSION:disabled")
    }
}

// BuildVersion.VERSION, from VERSION: NativeLoader compares it with the
// native library's own, and the User-Agent token carries it. Never committed.
val buildVersion =
    tasks.register("generateBuildVersion") {
        val out = layout.buildDirectory.dir("generated/buildVersion")
        val version = project.version.toString()
        inputs.property("version", version)
        outputs.dir(out)
        doLast {
            val file = out.get().file("com/getlingara/embed/BuildVersion.kt").asFile
            file.parentFile.mkdirs()
            file.writeText(
                "package com.getlingara.embed\n\ninternal object BuildVersion {\n    const val VERSION: String = \"$version\"\n}\n",
            )
        }
    }

sourceSets {
    main {
        kotlin.srcDir(buildVersion)
    }
}

tasks.jar {
    manifest {
        attributes("Automatic-Module-Name" to "com.getlingara.embed")
    }
}

dependencies {
    compileOnly(libs.jna)
    testImplementation(libs.jna)
}

// The host library and the conformance server come from `make test-kotlin`
// (kotlin-natives-host), so the suite runs against the Rust of this commit.
val hostLibrary = rootProject.layout.projectDirectory.dir("target/debug").asFile.path
val emptyLibraryDir = layout.buildDirectory.dir("no-natives")

tasks.withType<Test>().configureEach {
    systemProperty("lingara.root", rootProject.projectDir.path)
    systemProperty("lingara.version", project.version.toString())
}

tasks.test {
    systemProperty("jna.library.path", hostLibrary)
    useJUnitPlatform { excludeTags("no-natives") }
}

// AC5's first half needs a JVM whose JNA has never seen the host library:
// JNA reads jna.library.path once, so the case gets its own test task.
val noNativesTest =
    tasks.register<Test>("noNativesTest") {
        description = "NativeLoader with no native library anywhere JNA looks."
        testClassesDirs = sourceSets.test.get().output.classesDirs
        classpath = sourceSets.test.get().runtimeClasspath
        useJUnitPlatform { includeTags("no-natives") }
        doFirst { emptyLibraryDir.get().asFile.mkdirs() }
        systemProperty("jna.library.path", emptyLibraryDir.get().asFile.path)
    }

tasks.check {
    dependsOn(noNativesTest)
}
