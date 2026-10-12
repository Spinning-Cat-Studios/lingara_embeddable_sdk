// lingara-embed-server-kotlin: the Kotlin server kit (ADR 1.10.26af D2).
//
// Its own coordinates, apart from K7's player artifact lingara-embed-kotlin,
// so a game never pays for coroutines, kotlinx-serialization and a client
// built for a server that holds a secret (K7 D2's rule). The one runtime
// dependency is the Lingara library, written literally on the line below so
// tools/library-floor/check.sh can read the floor (LIBRARY_FLOOR):
// coroutines, kotlinx-serialization and the webhook verifier come through it.
// It is `api` because the kit's signatures carry the library's
// LingaraClient, MintedToken and Event.

plugins {
    id("lingara.kotlin-conventions")
    id("lingara.publishing-conventions")
}

description = "The official Kotlin server kit for embedding Lingara: mint, route, delete, snippet."

kotlin {
    explicitApi()
}

dependencies {
    api("com.getlingara:lingara-kotlin:0.2.0-alpha.1")
}

// The kit's version as a resource, from VERSION (D1: no kit writes its
// version by hand): the default elementSrc names the element of this release.
val versionResource =
    tasks.register("generateVersionResource") {
        val out = layout.buildDirectory.dir("generated/version")
        val version = project.version.toString()
        inputs.property("version", version)
        outputs.dir(out)
        doLast {
            val file = out.get().file("com/getlingara/embed/server/kotlin/version.txt").asFile
            file.parentFile.mkdirs()
            file.writeText(version)
        }
    }

sourceSets {
    main {
        resources.srcDir(versionResource)
    }
}

val pom = tasks.named("generatePomFileForMavenPublication")

tasks.test {
    dependsOn(pom)
    systemProperty("lingara.version", project.version.toString())
    systemProperty("lingara.snippetVectors", rootProject.file("conformance/vectors/snippet.json").path)
    systemProperty("lingara.handoffVectors", rootProject.file("conformance/vectors/token-handoff.json").path)
    systemProperty("lingara.webhookVectors", rootProject.file("conformance/vectors/webhook-signatures.json").path)
    systemProperty("lingara.pom", layout.buildDirectory.file("publications/maven/pom-default.xml").get().asFile.path)
}
