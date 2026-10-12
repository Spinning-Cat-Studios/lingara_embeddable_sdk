// The desktop launcher: the LWJGL3 backend and lingara-embed-natives-desktop
// (ADR 1.10.26ae D8). `run` starts it; CI runs it for 10 s under xvfb-run.

plugins {
    kotlin("jvm")
    application
}

kotlin {
    jvmToolchain(17)
}

dependencies {
    implementation(project(":core"))
    implementation(libs.gdx.backend.lwjgl3)
    implementation(variantOf(libs.gdx.platform) { classifier("natives-desktop") })
    implementation("com.getlingara:lingara-embed-natives-desktop")
}

application {
    mainClass = "com.getlingara.embed.example.Lwjgl3LauncherKt"
    // GLFW must own the main thread on macOS.
    if (System.getProperty("os.name").startsWith("Mac")) {
        applicationDefaultJvmArgs = listOf("-XstartOnFirstThread")
    }
}
