// The convention plugins' own build (ADR 1.10.26af D13, after the app kits'
// 30.9.26am D8). Each plugin a convention applies is on this classpath at its
// catalogue pin, so no project build file names a plugin version.

plugins {
    `kotlin-dsl`
}

dependencies {
    implementation(libs.spotless.plugin)
    // The Maven Central publishing plugin.
    implementation(libs.maven.publish.plugin)
    // lingara.kotlin-conventions applies the first; the second is here for
    // K7's projects, which apply it without a version.
    implementation(libs.kotlin.gradle.plugin)
    implementation(libs.kotlin.serialization.plugin)
}
