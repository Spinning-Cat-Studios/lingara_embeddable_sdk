package com.getlingara.embed

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals

class GeneratedSurfaceTest {
    // A declaration typed unsigned: a parameter, property or return type.
    private val unsigned = Regex(""":\s*U(Byte|Short|Int|Long)\b""")

    // UniFFI's own helper machinery sizes its buffers in ULong: the public
    // FfiConverter.allocationSize and RustBuffer's internal functions. Those
    // are UniFFI's, not the kit's surface, and no game calls them.
    private fun uniffiHelper(line: String): Boolean {
        val code = line.trim()
        return code.startsWith("//") || code.startsWith("internal ") || "allocationSize" in code
    }

    // 1.10.26ae AC3: no record field, method, parameter or return the kit
    // exposes is typed with an unsigned Kotlin type, so Java can call every
    // generated getter by its plain JVM name.
    @Test
    fun noUnsignedTypeCrossesTheBoundary() {
        val root = File(System.getProperty("lingara.root"))
        val file = File(root, "kotlin/src/main/kotlin/com/getlingara/embed/lingara_embed_uniffi.kt")
        val offenders =
            file
                .readLines()
                .withIndex()
                .filter { (_, line) -> unsigned.containsMatchIn(line) && !uniffiHelper(line) }
                .map { (n, line) -> "${n + 1}: ${line.trim()}" }
        assertEquals(emptyList(), offenders, "unsigned types on the kit's surface")
    }
}
