// The Android launcher: libGDX's Android backend and
// lingara-embed-natives-android (ADR 1.10.26ae D8), minSdk 23, with a
// minified release build so a device smoke (AC15) runs R8 over the
// natives artefact's consumer rules. Only arm64-v8a, as the kit ships.

plugins {
    alias(libs.plugins.android.application)
}

val gdxNatives: Configuration by configurations.creating

android {
    namespace = "com.getlingara.embed.example"
    compileSdk = 36
    defaultConfig {
        applicationId = "com.getlingara.embed.example"
        minSdk = 23
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
        ndk { abiFilters += "arm64-v8a" }
    }
    buildTypes {
        release {
            isMinifyEnabled = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"))
            // A debug-signed release, so the smoke installs without a keystore.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    sourceSets {
        getByName("main") {
            jniLibs.directories.add(layout.buildDirectory.dir("gdx-natives").get().asFile.path)
        }
    }
}

dependencies {
    implementation(project(":core"))
    implementation(libs.gdx.backend.android)
    implementation("com.getlingara:lingara-embed-natives-android")
    gdxNatives(variantOf(libs.gdx.platform) { classifier("natives-arm64-v8a") })
}

// libGDX's own .so, out of its natives JAR, beside the kit's in the APK.
val copyGdxNatives =
    tasks.register<Copy>("copyGdxNatives") {
        from(gdxNatives.map { zipTree(it) }) { include("*.so") }
        into(layout.buildDirectory.dir("gdx-natives/arm64-v8a"))
    }

tasks.named("preBuild") { dependsOn(copyGdxNatives) }
