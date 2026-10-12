package com.getlingara.embed

import java.util.concurrent.atomic.AtomicBoolean

/**
 * Thrown by [LingaraEmbed.start] when the native library cannot be used: no
 * natives artefact (and so no JNA), no library for this platform, or a
 * library of another version (ADR 1.10.26ae D5). The message names the
 * artefact to add.
 */
public class LingaraNativesMissingException(
    message: String,
    cause: Throwable? = null,
) : RuntimeException(message, cause)

/**
 * Loads the native library before anything touches the generated code
 * (ADR 1.10.26ae D5), whose first load checks every checksum and fails with
 * a message naming neither the cause nor the fix. It probes for JNA, calls
 * the library's plain-C version function through a JNA interface of its own,
 * and compares the answer with the version this artefact was built at.
 */
internal object NativeLoader {
    private const val LIBRARY = "lingara_embed_uniffi"
    private val loaded = AtomicBoolean(false)

    /** Once per process; a failure is retried on the next start. */
    fun ensureLoaded() {
        if (!loaded.get()) {
            check(BuildVersion.VERSION, NativeLoader::class.java.classLoader)
            loaded.set(true)
        }
    }

    /** The whole check, with the expected version and the probing class loader as inputs. */
    internal fun check(
        expected: String,
        loader: ClassLoader?,
    ) {
        try {
            Class.forName("com.sun.jna.Native", false, loader)
        } catch (e: ClassNotFoundException) {
            throw missing("JNA is not on the classpath", e)
        }
        val reported =
            try {
                Jna.version(libraryName())
            } catch (e: UnsatisfiedLinkError) {
                throw missing("no $LIBRARY library for this platform", e)
            }
        if (reported != expected) {
            throw LingaraNativesMissingException(
                "the native library is version $reported but lingara-embed-kotlin is $expected: " +
                    "add ${artefact()}:$expected",
            )
        }
    }

    /** UniFFI's own override, so a game that redirects the generated code redirects this too. */
    private fun libraryName(): String = System.getProperty("uniffi.component.$LIBRARY.libraryOverride") ?: LIBRARY

    private fun missing(
        why: String,
        cause: Throwable,
    ) = LingaraNativesMissingException(
        "Lingara cannot start on ${platform()}: $why. Add com.getlingara:${artefact()} " +
            "at the same version as lingara-embed-kotlin",
        cause,
    )

    private fun isAndroid(): Boolean = System.getProperty("java.vm.name").orEmpty().contains("Dalvik", ignoreCase = true)

    private fun artefact(): String = if (isAndroid()) "lingara-embed-natives-android" else "lingara-embed-natives-desktop"

    private fun platform(): String =
        if (isAndroid()) {
            "Android ${System.getProperty("os.arch")} (v1 ships arm64-v8a only)"
        } else {
            "${System.getProperty("os.name")} ${System.getProperty("os.arch")}"
        }

    /** Every JNA type lives here, loaded only once the probe has passed. */
    private object Jna {
        interface VersionLibrary : com.sun.jna.Library {
            @Suppress("FunctionName", "ktlint:standard:function-naming")
            fun lingara_embed_uniffi_version(): String
        }

        fun version(library: String): String =
            com.sun.jna.Native
                .load(library, VersionLibrary::class.java)
                .lingara_embed_uniffi_version()
    }
}
