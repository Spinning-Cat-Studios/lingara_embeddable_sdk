// :snippets:server:kotlin — the Kotlin server kit's examples the
// documentation site shows (ADR 1.10.26af D13; P1 §3's keys install, mint,
// deletePlayer, webhook and widgetSnippet). Every marked region is vendored
// at a released tag, and `make test-kotlin-server` compiles every file, so
// what the site shows compiled. install.kts sits outside every source set, so
// Gradle never compiles it. Ktor is the snippets' own dependency, as it would
// be the caller's.

plugins {
    id("lingara.kotlin-conventions")
}

dependencies {
    implementation(project(":kotlin-server"))
    implementation(libs.ktor.server.core)
    implementation(libs.ktor.server.cio)
}
