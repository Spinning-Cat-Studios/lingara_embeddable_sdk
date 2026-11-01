package com.getlingara.embed.server.kotlin

import com.getlingara.kotlin.embed.MintedToken
import com.getlingara.kotlin.model.UpdateAvailable

// The handoff body (ADR 1.10.26af D6, ES6): {"token":"lgr_et_…","expires_at":"…"}, the shape every
// player kit and the widget read, and since ADR 31.10.26f a third key, update_available, when the
// mint answered one. This is the one place a kit exposes the token on purpose.

/**
 * The JSON your token route sends to the player. `expires_at` is the server's string, copied byte
 * for byte; `expires_in` and `subject` are not forwarded. The mint's `update_available` is
 * forwarded when present.
 */
public fun tokenHandoff(token: MintedToken): String = handoffBody(token.token.exposeSecret(), token.expiresAt, token.updateAvailable)

/**
 * The pure body, held to `token-handoff.json`: compact, `token` then `expires_at`, then
 * `update_available` only when given (its keys `latest`, `deprecated`, `message`, nulls kept); `/`
 * and non-ASCII unescaped.
 */
internal fun handoffBody(
    token: String,
    expiresAt: String,
    updateAvailable: UpdateAvailable? = null,
): String {
    val notice = updateAvailable?.let { ",\"update_available\":${noticeJson(it)}" }.orEmpty()
    return "{\"token\":${jsonString(token)},\"expires_at\":${jsonString(expiresAt)}$notice}"
}

/** The notice in the wire order (V1 §6), every key written, a null kept. */
private fun noticeJson(n: UpdateAvailable): String =
    "{\"latest\":${jsonOrNull(n.latest)},\"deprecated\":${n.deprecated},\"message\":${jsonOrNull(n.message)}}"

private fun jsonOrNull(value: String?): String = value?.let(::jsonString) ?: "null"

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
