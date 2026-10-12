// :snippets:player:java — the Java facade's examples the documentation site
// shows (ADR 1.10.26ae; P1 §3's keys install, tokenSource, poll, practice
// and dialogue). `make test-java` compiles Snippets.java against the repo's
// own artefacts, so what the site shows compiled. install.xml sits outside
// every source set, so Gradle never reads it.

plugins {
    id("lingara.java-conventions")
}

dependencies {
    implementation(project(":java"))
    implementation(project(":natives-desktop"))
}
