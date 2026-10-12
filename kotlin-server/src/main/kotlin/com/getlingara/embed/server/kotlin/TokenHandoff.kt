package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.embed.MintedToken

// The handoff body (ADR 1.10.26af D6, ES6): exactly {"token":"lgr_et_…","expires_at":"…"}, the
// shape every player kit and the widget read. This is the one place a kit exposes the token on
// purpose.

/**
 * The JSON your token route sends to the player. `expires_at` is the server's string, copied byte
 * for byte; `expires_in` and `subject` are not forwarded.
 */
public fun tokenHandoff(token: MintedToken): String = handoffBody(token.token.exposeSecret(), token.expiresAt)

/**
 * The pure body over two strings, held to `token-handoff.json`: compact, these two keys in this
 * order, `/` and non-ASCII unescaped.
 */
internal fun handoffBody(
    token: String,
    expiresAt: String,
): String = "{\"token\":${jsonString(token)},\"expires_at\":${jsonString(expiresAt)}}"

/** A JSON string: `"`, `\` and the C0 controls escaped, nothing else. */
private fun jsonString(value: String): String =
    buildString {
        append('"')
        for (c in value) {
            when (c) {
                '"' -> append("\\\"")
                '\\' -> append("\\\\")
                '\n' -> append("\\n")
                '\r' -> append("\\r")
                '\t' -> append("\\t")
                '\b' -> append("\\b")
                '\u000c' -> append("\\f")
                else -> if (c < ' ') append("\\u%04x".format(c.code)) else append(c)
            }
        }
        append('"')
    }
