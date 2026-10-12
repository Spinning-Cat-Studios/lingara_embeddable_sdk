// :kotlin-server:conformance-es — the Kotlin server kit's ES harness (ADR
// 1.10.26af D13; conformance/es/CONTRACT.md, What a harness reports), on the
// kit's public API only, its webhook receiver served by Ktor 3 on the CIO
// engine. Ktor is a conformance-only dependency: this project is never
// published, and code-health leaves it out on purpose (D13). `installDist`
// writes the start script make/kotlin-server.mk runs:
// kotlin-server/conformance-es/build/install/conformance-es/bin/conformance-es.

plugins {
    id("lingara.kotlin-conventions")
    application
}

dependencies {
    implementation(project(":kotlin-server"))
    implementation(libs.ktor.server.core)
    implementation(libs.ktor.server.cio)
}

application {
    mainClass = "com.getlingara.embed.server.kotlin.conformance.HarnessKt"
}
