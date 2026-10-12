// :kotlin:conformance — the Kotlin surface's EK harness (ADR 1.10.26ae D9;
// conformance/ek/CONTRACT.md). It drives every case's host script through
// LingaraEmbed alone, reading and writing K1's JSON only through the
// generated commandFromJson and eventToJson. kotlinx-serialization parses the
// cases and is conformance-only: this project is never published.
// `installDist` writes the start script make/kotlin.mk runs.

plugins {
    id("lingara.kotlin-conventions")
    application
}

dependencies {
    implementation(project(":kotlin"))
    implementation(libs.kotlinx.serialization.json)
    runtimeOnly(libs.jna)
}

application {
    mainClass = "com.getlingara.embed.conformance.HarnessKt"
    // The host library make/kotlin.mk's kotlin-natives-host builds.
    applicationDefaultJvmArgs = listOf("-Djna.library.path=${rootProject.projectDir}/target/debug")
}
