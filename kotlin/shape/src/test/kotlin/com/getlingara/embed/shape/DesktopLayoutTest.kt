package com.getlingara.embed.shape

import kotlin.test.Test
import kotlin.test.assertEquals

class DesktopLayoutTest {
    // 1.10.26ae AC11: the desktop JAR holds exactly D5's four libraries, each
    // under JNA's resource prefix for its platform, and no other library.
    @Test
    fun theJarHoldsExactlyTheFourDesktopLibraries() {
        val libraries =
            Artefact("lingara-embed-natives-desktop").zip("jar") { jar ->
                jar
                    .entries()
                    .asSequence()
                    .map { it.name }
                    .filter { it.endsWith(".dll") || it.endsWith(".dylib") || it.endsWith(".so") }
                    .toSortedSet()
            }
        val expected =
            sortedSetOf(
                "darwin-aarch64/liblingara_embed_uniffi.dylib",
                "darwin-x86-64/liblingara_embed_uniffi.dylib",
                "linux-x86-64/liblingara_embed_uniffi.so",
                "win32-x86-64/lingara_embed_uniffi.dll",
            )
        assertEquals(expected, libraries)
    }
}
