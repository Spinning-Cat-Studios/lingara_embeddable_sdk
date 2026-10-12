// The example's plugin versions, from the repo-root catalogue: the Kotlin
// Gradle plugin for core and lwjgl3, and the Android Gradle plugin for
// android. Nothing here is published.

plugins {
    alias(libs.plugins.kotlin.jvm) apply false
    alias(libs.plugins.android.application) apply false
}
