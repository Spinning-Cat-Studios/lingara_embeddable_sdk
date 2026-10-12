// The game itself: libGDX core and lingara-embed-kotlin. No natives here;
// each launcher adds its platform's.

plugins {
    kotlin("jvm")
}

kotlin {
    jvmToolchain(17)
}

dependencies {
    api(libs.gdx)
    api("com.getlingara:lingara-embed-kotlin")
}
