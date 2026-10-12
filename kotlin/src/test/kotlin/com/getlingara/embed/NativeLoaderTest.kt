package com.getlingara.embed

import org.junit.jupiter.api.Tag
import kotlin.test.Test
import kotlin.test.assertFailsWith
import kotlin.test.assertTrue

class NativeLoaderTest {
    /** Every class but JNA's: a game with no natives artefact has no JNA either. */
    private class HidesJna(
        parent: ClassLoader,
    ) : ClassLoader(parent) {
        override fun loadClass(
            name: String,
            resolve: Boolean,
        ): Class<*> {
            if (name.startsWith("com.sun.jna.")) throw ClassNotFoundException(name)
            return super.loadClass(name, resolve)
        }
    }

    // 1.10.26ae AC5: with jna.library.path at an empty directory and no
    // natives artefact (this test's own JVM, the noNativesTest task), and
    // again with JNA hidden from the class loader, start throws
    // LingaraNativesMissingException naming the desktop natives artefact,
    // before any generated class loads.
    @Test
    @Tag("no-natives")
    fun missingNativesOrJnaNameTheArtefactToAdd() {
        val noLibrary =
            assertFailsWith<LingaraNativesMissingException> {
                LingaraEmbed.start(EmbedConfig(streamOnStart = false))
            }
        assertTrue("lingara-embed-natives-desktop" in noLibrary.message.orEmpty(), noLibrary.message)
        assertTrue(noLibrary.cause is UnsatisfiedLinkError, "the load failed, not the generated code")
        val noJna =
            assertFailsWith<LingaraNativesMissingException> {
                NativeLoader.check(LingaraEmbed.VERSION, HidesJna(javaClass.classLoader))
            }
        assertTrue("JNA" in noJna.message.orEmpty(), noJna.message)
        assertTrue("lingara-embed-natives-desktop" in noJna.message.orEmpty(), noJna.message)
    }

    // 1.10.26ae AC6: a native library reporting another version is refused,
    // with both versions named.
    @Test
    fun aStaleNativeLibraryIsRefusedAtStart() {
        val stale =
            assertFailsWith<LingaraNativesMissingException> {
                NativeLoader.check("9.9.9-expected", javaClass.classLoader)
            }
        val message = stale.message.orEmpty()
        assertTrue("9.9.9-expected" in message && LingaraEmbed.VERSION in message, message)
        NativeLoader.check(LingaraEmbed.VERSION, javaClass.classLoader)
    }
}
