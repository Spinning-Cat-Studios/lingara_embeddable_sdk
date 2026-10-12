// The libGDX example (ADR 1.10.26ae D8): libGDX's standard core, lwjgl3 and
// android modules, as a separate build that is never published. It builds
// against the repo's own artefacts through the composite below, so it always
// runs this commit's kit. Run it through the root wrapper:
// `./gradlew -p kotlin/examples/libgdx build` (make jvm-example).

pluginManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
        google()
    }
}

dependencyResolutionManagement {
    repositories {
        mavenCentral()
        google()
    }
    versionCatalogs {
        create("libs") {
            from(files("../../../gradle/libs.versions.toml"))
        }
    }
}

rootProject.name = "lingara-libgdx-example"

includeBuild("../../..") {
    dependencySubstitution {
        substitute(module("com.getlingara:lingara-embed-kotlin")).using(project(":kotlin"))
        substitute(module("com.getlingara:lingara-embed-natives-desktop")).using(project(":natives-desktop"))
        substitute(module("com.getlingara:lingara-embed-natives-android")).using(project(":natives-android"))
    }
}

include("core")
include("lwjgl3")
include("android")
