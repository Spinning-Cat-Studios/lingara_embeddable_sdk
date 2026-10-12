package com.getlingara.embed.server.kotlin

// The widget snippet (ADR 1.10.26af D7, ES4): the `<lingara-embed>` element and its two module
// scripts, rendered identically by every server kit. The markup is the widget's own; the token
// never appears in it, only the endpoint the element asks at run time.

private object Version {
    val value: String =
        checkNotNull(Version::class.java.getResourceAsStream("version.txt")) { "the kit's version.txt resource is missing" }
            .use { it.readBytes().toString(Charsets.UTF_8).trim() }
}

/** The script-tag build of this release's element. */
public val DEFAULT_ELEMENT_SRC: String = "https://cdn.jsdelivr.net/npm/@lingara/embed@${Version.value}/dist/lingara-embed.min.js"

/**
 * The three lines to paste into your page, with no trailing newline.
 *
 * @throws SnippetException naming the first option that breaks its rule
 */
public fun widgetSnippet(options: SnippetOptions): String {
    val surface = options.surface ?: "practice"
    val id = options.id ?: "lingara-embed"
    firstRefused(options, surface, id)?.let { throw SnippetException(it) }
    val attributes =
        listOf(
            "id" to id,
            "surface" to surface,
            "plan-id" to options.planId,
            "theme" to options.theme,
            "locale" to options.locale,
            "lingara-origin" to options.lingaraOrigin,
            "title" to options.title,
        )
    val element = attributes.joinToString("") { (name, value) -> value?.let { " $name=\"${attr(it)}\"" } ?: "" }
    val nonce = options.nonce?.let { " nonce=\"${attr(it)}\"" } ?: ""
    val src = attr(options.elementSrc ?: DEFAULT_ELEMENT_SRC)
    return "<lingara-embed$element></lingara-embed>\n" +
        "<script type=\"module\"$nonce src=\"$src\"></script>\n" +
        "<script type=\"module\"$nonce>document.getElementById(${scriptString(id)}).tokenProvider = " +
        "async () => (await fetch(${scriptString(options.tokenEndpoint)}, " +
        "{ method: \"POST\", credentials: \"same-origin\" })).json();</script>"
}

/** An attribute value: the five characters, and nothing else. */
internal fun attr(value: String): String =
    buildString {
        for (c in value) {
            when (c) {
                '&' -> append("&amp;")
                '<' -> append("&lt;")
                '>' -> append("&gt;")
                '"' -> append("&quot;")
                '\'' -> append("&#39;")
                else -> append(c)
            }
        }
    }

/** A JSON string literal that can never close the script or open a comment. */
internal fun scriptString(value: String): String =
    buildString {
        append('"')
        for (c in value) {
            when (c) {
                '"' -> append("\\\"")
                '\\' -> append("\\\\")
                '<' -> append("\\u003c")
                '>' -> append("\\u003e")
                '&' -> append("\\u0026")
                ' ' -> append("\\u2028")
                ' ' -> append("\\u2029")
                else -> append(c)
            }
        }
        append('"')
    }
