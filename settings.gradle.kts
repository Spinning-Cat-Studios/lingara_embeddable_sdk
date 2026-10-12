// The repo-root Gradle build the JVM kits share (ADR 1.10.26af D13; the
// first of K7 1.10.26ae and V1 to land creates it, the second adds its
// includes). V1's four projects are below; K7 adds `:kotlin`, `:java`, the
// natives and their conformance and snippet projects as further include(...)
// lines, and nothing else here changes.
//
// scs-snapshot reads this file statically to learn which directories the
// build needs, so it holds only literal include(...) calls and one
// includeBuild: no loop, and no project directory reassigned. Every
// project's directory is its path, `:java-server:conformance-es` being
// java-server/conformance-es/.

pluginManagement {
    includeBuild("build-logic")
    repositories {
        gradlePluginPortal()
        mavenCentral()
        // The Android Gradle plugin, for natives-android (K7).
        google()
    }
}

dependencyResolutionManagement {
    repositories {
        mavenCentral()
        google()
    }
}

rootProject.name = "lingara-embeddable-sdk"

// V1 (ADR 1.10.26af): the JVM server kits.
include(":java-server")
include(":java-server:conformance-es")
include(":snippets:server:java")
include(":kotlin-server")
include(":kotlin-server:conformance-es")
include(":snippets:server:kotlin")
// K7 (ADR 1.10.26ae): the JVM player kit, its EK harnesses and its
// snippets. natives-android and the package-shape project need an Android
// SDK, so a contributor without one still builds kotlin/ and java/.
include(":kotlin")
include(":kotlin:conformance")
include(":java")
include(":java:conformance")
include(":natives-desktop")
include(":snippets:player:kotlin")
include(":snippets:player:java")
val androidSdk = providers.environmentVariable("ANDROID_HOME").orNull ?: providers.environmentVariable("ANDROID_SDK_ROOT").orNull
if (androidSdk != null && file(androidSdk).isDirectory) {
    include(":natives-android")
    include(":kotlin:shape")
} else {
    logger.lifecycle("natives-android and kotlin/shape skipped: set ANDROID_HOME to build them (ADR 1.10.26ae D10)")
}
