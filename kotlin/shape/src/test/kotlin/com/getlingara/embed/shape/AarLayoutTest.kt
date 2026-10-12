package com.getlingara.embed.shape

import java.nio.ByteBuffer
import java.nio.ByteOrder
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class AarLayoutTest {
    private val library = "jni/arm64-v8a/liblingara_embed_uniffi.so"

    /** The `p_align` of every PT_LOAD segment of a 64-bit little-endian ELF. */
    private fun loadAlignments(elf: ByteBuffer): List<Long> {
        val phoff = elf.getLong(0x20)
        val phentsize = elf.getShort(0x36).toInt()
        val phnum = elf.getShort(0x38).toInt()
        return (0 until phnum)
            .map { phoff.toInt() + it * phentsize }
            .filter { elf.getInt(it) == PT_LOAD }
            .map { elf.getLong(it + 0x30) }
    }

    // 1.10.26ae AC10: the AAR's library is an arm64 ELF whose every LOAD
    // segment is aligned to 16 KB (Play's bar from Android 15), read with no
    // llvm-readelf; and the AAR carries D5's consumer keep rules.
    @Test
    fun theAndroidLibraryIsArm64AndSixteenKbAligned() {
        val aar = Artefact("lingara-embed-natives-android")
        val (bytes, rules) =
            aar.zip("aar") { zip ->
                val so = zip.getInputStream(zip.getEntry(library) ?: error("no $library")).readBytes()
                val proguard = zip.getEntry("proguard.txt")?.let { zip.getInputStream(it).readBytes().decodeToString() }.orEmpty()
                so to proguard
            }
        val elf = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN)
        assertEquals(0x7F454C46, ByteBuffer.wrap(bytes, 0, 4).int, "an ELF")
        assertEquals(ELFCLASS64, bytes[4].toInt(), "64-bit")
        assertEquals(EM_AARCH64, elf.getShort(0x12).toInt(), "arm64")
        val alignments = loadAlignments(elf)
        assertTrue(alignments.isNotEmpty(), "LOAD segments")
        assertTrue(alignments.all { it % SIXTEEN_KB == 0L }, "every LOAD aligned to 16 KB: $alignments")
        for (rule in listOf(
            "-keep class com.sun.jna.** { *; }",
            "com.sun.jna.Structure",
            "com.sun.jna.Callback",
            "-dontwarn java.awt.**",
        )) {
            assertTrue(rule in rules, "proguard.txt carries $rule")
        }
    }

    private companion object {
        const val PT_LOAD = 1
        const val ELFCLASS64 = 2
        const val EM_AARCH64 = 183
        const val SIXTEEN_KB = 16_384L
    }
}
