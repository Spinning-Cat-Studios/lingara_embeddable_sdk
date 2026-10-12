// lingara-embed-natives-android: the arm64 library as an AAR (ADR 1.10.26ae
// D1, D5), with the consumer R8 rules every minified game needs and the
// INTERNET permission. Its jniLibs are dist/jvm/android-arm64/, which
// make/kotlin.mk's natives-android writes: API 23, 16 KB pages.
//
// JNA's AAR (its own .so files) is `api`, for the same reason the desktop
// artefact's JAR is: the generated EmbedHandle names JNA's Pointer. AGP 9
// compiles Kotlin itself, and this module has none, so no Kotlin plugin.
//
// Publishing is the Maven Central convention plugin (1.10.26ag D6a), which
// publishes the release variant as lingara-embed-natives-android.

plugins {
    alias(libs.plugins.android.library)
    id("lingara.publishing-conventions")
}

description = "Lingara inside an Android game: the arm64 native library for lingara-embed-kotlin."

android {
    namespace = "com.getlingara.embed.natives"
    compileSdk = 36
    defaultConfig {
        minSdk = 23
        consumerProguardFiles("consumer-rules.pro")
    }
    sourceSets {
        getByName("main") {
            jniLibs.directories.add(rootProject.layout.projectDirectory.dir("dist/jvm/android-arm64").asFile.path)
        }
    }
    // The library is stripped already (the release-jvm profile), and the AAR
    // must carry the bytes the release's natives leg checked (1.10.26ag D3):
    // AGP's own strip pass would rewrite them.
    packaging {
        jniLibs {
            keepDebugSymbols += "**/*.so"
        }
    }
}

dependencies {
    api(libs.jna) {
        artifact { type = "aar" }
    }
}

// The AAR is a zip, not a jar, so the convention's META-INF/LICENSE does not
// reach it: the root LICENSE goes at its top level, where
// tools/release/check-package.sh looks.
tasks.withType<Zip>().matching { it.name == "bundleReleaseAar" }.configureEach {
    from(rootProject.file("LICENSE"))
}
