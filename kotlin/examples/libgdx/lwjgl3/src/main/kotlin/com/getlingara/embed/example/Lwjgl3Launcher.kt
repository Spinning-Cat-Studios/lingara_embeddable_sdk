package com.getlingara.embed.example

import com.badlogic.gdx.backends.lwjgl3.Lwjgl3Application
import com.badlogic.gdx.backends.lwjgl3.Lwjgl3ApplicationConfiguration

/** The desktop launcher (ADR 1.10.26ae D8): lingara-embed-natives-desktop supplies the library. */
fun main() {
    val config =
        Lwjgl3ApplicationConfiguration().apply {
            setTitle("Lingara — libGDX example")
            setWindowedMode(960, 540)
        }
    Lwjgl3Application(LingaraDemo(), config)
}
