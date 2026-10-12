// lingara-embed-server-java: the Java server kit (ADR 1.10.26af D2).
//
// Its own coordinates, apart from K7's player artifact lingara-embed-java, so
// a game never ships a server's JSON stack and a server never ships natives.
// The one runtime dependency is the Lingara library, written literally on the
// line below so tools/library-floor/check.sh can read the floor
// (LIBRARY_FLOOR): Jackson and the webhook verifier come through it. It is
// `api` because the kit's signatures carry the library's LingaraClient,
// MintedToken and Event. Publishing is the Maven Central convention plugin,
// whose artifact for a `-server` project is lingara-embed-server-<language>.

plugins {
    id("lingara.java-conventions")
    id("lingara.publishing-conventions")
}

description = "The official Java server kit for embedding Lingara: mint, route, delete, snippet."

dependencies {
    api("com.getlingara:lingara-java:0.2.0-alpha.1")
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
            val file = out.get().file("com/getlingara/embed/server/version.txt").asFile
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
    systemProperty("lingara.moduleInfo", layout.buildDirectory.file("classes/java/main/module-info.class").get().asFile.path)
}
