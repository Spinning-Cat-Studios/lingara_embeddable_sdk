// :java-server:conformance-es — the Java server kit's ES harness (ADR
// 1.10.26af D13; conformance/es/CONTRACT.md, What a harness reports), on the
// kit's public API only. A separate project, so it is never in the published
// jar; the JDK's HttpServer is its webhook receiver and Jackson (through the
// library) its JSON, so it declares nothing more. `installDist` writes the
// start script make/java-server.mk runs:
// java-server/conformance-es/build/install/conformance-es/bin/conformance-es.

plugins {
    id("lingara.java-conventions")
    application
}

dependencies {
    implementation(project(":java-server"))
}

application {
    mainClass = "com.getlingara.embed.server.conformance.Harness"
}
