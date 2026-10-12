package com.getlingara.embed.server.kotlin

// ES4's option rules (ADR 1.10.26af D7), one function per row of the contract's table. Every
// pattern is ASCII-only and matched against the whole value.

private val PLAN_ID = Regex("[0-9A-Fa-f]{8}(-[0-9A-Fa-f]{4}){3}-[0-9A-Fa-f]{12}")
private val SURFACE = Regex("[a-z][a-z0-9-]{0,31}")
private val THEME = Regex("light|dark|auto")
private val LOCALE = Regex("[A-Za-z]{2,3}(-[A-Za-z0-9]{2,8}){0,3}")
private val ORIGIN =
    Regex("(https://[A-Za-z0-9-]+(\\.[A-Za-z0-9-]+)*(:[0-9]{1,5})?|http://(localhost|127\\.0\\.0\\.1)(:[0-9]{1,5})?)/?")
private val ID = Regex("[A-Za-z][A-Za-z0-9_-]{0,63}")
private val NONCE = Regex("[A-Za-z0-9+/=_-]{8,128}")
private const val MAX_TITLE = 80
private const val MAX_URL_BYTES = 2048

/** The contract's whitespace set: U+2000 to U+200A, and these. */
private const val SPACES = " \u0085      　"

/** The first refused option's name, in the contract's order, or null. */
internal fun firstRefused(
    o: SnippetOptions,
    surface: String,
    id: String,
): String? {
    val rules =
        listOf(
            "planId" to PLAN_ID.matches(o.planId),
            "surface" to SURFACE.matches(surface),
            "theme" to optional(o.theme) { THEME.matches(it) },
            "locale" to optional(o.locale) { LOCALE.matches(it) },
            "lingaraOrigin" to optional(o.lingaraOrigin) { ORIGIN.matches(it) },
            "title" to optional(o.title, ::title),
            "id" to ID.matches(id),
            "tokenEndpoint" to tokenEndpoint(o.tokenEndpoint),
            "nonce" to optional(o.nonce) { NONCE.matches(it) },
            "elementSrc" to optional(o.elementSrc, ::elementSrc),
        )
    return rules.firstOrNull { !it.second }?.first
}

private fun optional(
    value: String?,
    rule: (String) -> Boolean,
): Boolean = value == null || rule(value)

/** 1 to 80 scalar values, none of them a control character. */
private fun title(s: String): Boolean {
    val count = s.codePointCount(0, s.length)
    return count in 1..MAX_TITLE && s.codePoints().noneMatch(::isControl)
}

/** A path that starts `/` but not `//`, or an absolute `https://` URL. */
private fun tokenEndpoint(s: String): Boolean {
    val shape = (s.startsWith("/") && !s.startsWith("//")) || s.startsWith("https://")
    return shape && urlText(s)
}

private fun elementSrc(s: String): Boolean = s.startsWith("https://") && urlText(s)

/** At most 2048 UTF-8 bytes, with no control character or whitespace. */
private fun urlText(s: String): Boolean =
    s.toByteArray(Charsets.UTF_8).size <= MAX_URL_BYTES && s.codePoints().noneMatch { isControl(it) || isSpace(it) }

/** Unicode category Cc. */
private fun isControl(c: Int): Boolean = c <= 0x1f || c in 0x7f..0x9f

private fun isSpace(c: Int): Boolean = c in 0x2000..0x200a || (c <= 0xffff && SPACES.indexOf(c.toChar()) >= 0)
