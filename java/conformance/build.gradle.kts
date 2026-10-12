// :java:conformance — the Java facade's EK harness (ADR 1.10.26ae D9;
// conformance/ek/CONTRACT.md). It drives every case's host script through
// LingaraEmbedJava alone, reading and writing K1's JSON only through the
// generated commandFromJson and eventToJson. Jackson parses the cases and is
// conformance-only: this project is never published. `installDist` writes
// the start script make/java.mk runs.

plugins {
    id("lingara.java-conventions")
    application
}

dependencies {
    implementation(project(":java"))
    implementation(libs.jackson.databind)
    runtimeOnly(libs.jna)
}

application {
    mainClass = "com.getlingara.embed.java.conformance.Harness"
    // The host library make/kotlin.mk's kotlin-natives-host builds.
    applicationDefaultJvmArgs = listOf("-Djna.library.path=${rootProject.projectDir}/target/debug")
}
