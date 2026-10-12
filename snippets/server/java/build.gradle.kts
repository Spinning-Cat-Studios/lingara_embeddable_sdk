// :snippets:server:java — the Java server kit's examples the documentation
// site shows (ADR 1.10.26af D13; P1 §3's keys install, mint, deletePlayer,
// webhook and widgetSnippet). Every marked region is vendored at a released
// tag, and `make test-java-server` compiles every file, so what the site
// shows compiled. install.xml sits outside every source set, so Gradle never
// reads it. The server is the JDK's HttpServer and the store is JDBC, so the
// snippets need nothing beyond the kit.

plugins {
    id("lingara.java-conventions")
}

dependencies {
    implementation(project(":java-server"))
}
