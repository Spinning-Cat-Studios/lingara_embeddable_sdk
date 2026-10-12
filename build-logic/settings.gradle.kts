// The included build that holds the JVM convention plugins (ADR 1.10.26af D13, after the app kits' 30.9.26am D8).

dependencyResolutionManagement {
    repositories {
        gradlePluginPortal()
        mavenCentral()
    }
    versionCatalogs {
        create("libs") {
            from(files("../gradle/libs.versions.toml"))
        }
    }
}

rootProject.name = "build-logic"
