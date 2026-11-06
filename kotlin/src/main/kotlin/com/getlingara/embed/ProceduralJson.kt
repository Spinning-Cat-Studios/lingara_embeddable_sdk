package com.getlingara.embed

// Plain JSON text for the procedural DSL (ADR 3.11.26j D8). Its key order and
// number spelling do not matter: `canonicalDialogue` rewrites the text in the
// crate's canonical form, so the JVM keeps no canonical writer of its own.

internal fun quote(text: String): String =
    buildString {
        append('"')
        for (c in text) {
            when {
                c == '"' -> append("\\\"")
                c == '\\' -> append("\\\\")
                c < ' ' -> append("\\u%04x".format(c.code))
                else -> append(c)
            }
        }
        append('"')
    }

/** An object of the members whose value is set; each value is JSON text already. */
internal fun obj(vararg members: Pair<String, String?>): String =
    members.filter { it.second != null }.joinToString(",", "{", "}") { (key, value) -> "${quote(key)}:$value" }

internal fun arr(items: List<String>): String = items.joinToString(",", "[", "]")

/** A whole number as an integer, so `3` stays `3`. */
internal fun num(x: Number): String {
    val d = x.toDouble()
    return if (d == Math.floor(d) && Math.abs(d) < 9.0e15) d.toLong().toString() else d.toString()
}
