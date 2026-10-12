// :snippets:player:kotlin — the Kotlin player kit's examples the
// documentation site shows (ADR 1.10.26ae; P1 §3's keys install,
// tokenSource, poll, practice and dialogue). `make test-kotlin` compiles
// Snippets.kt against the repo's own artefacts, so what the site shows
// compiled. install.kts sits outside every source set, so Gradle never
// compiles it.

plugins {
    id("lingara.kotlin-conventions")
}

dependencies {
    implementation(project(":kotlin"))
    implementation(project(":natives-desktop"))
}
