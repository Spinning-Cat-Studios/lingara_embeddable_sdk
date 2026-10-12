// :kotlin:shape — the four K7 artefacts as a game downloads them (ADR
// 1.10.26ae D9). Test-only and never published. It reads the repository
// `publishToMavenLocal -Dmaven.repo.local=<root>/build/m2` fills (make
// check-jvm-shape), not a publication's build directory, so it passes
// unchanged when K8 swaps core maven-publish for the Central convention.
// Included only with an Android SDK, since one of the four is an AAR.

plugins {
    id("lingara.kotlin-conventions")
}

val m2 = System.getProperty("maven.repo.local") ?: "${System.getProperty("user.home")}/.m2/repository"

tasks.test {
    systemProperty("lingara.m2", m2)
    systemProperty("lingara.version", project.version.toString())
    dependsOn(
        ":kotlin:publishToMavenLocal",
        ":java:publishToMavenLocal",
        ":natives-desktop:publishToMavenLocal",
        ":natives-android:publishToMavenLocal",
    )
    // The repository is outside Gradle's view, so the suite always re-reads it.
    outputs.upToDateWhen { false }
}
