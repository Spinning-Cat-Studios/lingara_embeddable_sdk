@file:JvmName("LingaraEmbedCommands")

package com.getlingara.embed

// The token, stream and world commands (ADR 1.10.26ae D3), as K1 names
// them. Each throws EmbedException.Failed when the core refuses it.

/** The game's token, valid until [expiresAt] (RFC 3339). */
@Throws(EmbedException::class)
public fun LingaraEmbed.provideToken(
    token: Secret,
    expiresAt: String,
): Unit = handle.provideToken(token, expiresAt)

/** The game's token, valid for [expiresInSecs] from now. */
@Throws(EmbedException::class)
public fun LingaraEmbed.provideTokenFor(
    token: Secret,
    expiresInSecs: Long,
): Unit = handle.provideTokenFor(token, expiresInSecs)

/** The game cannot get a token: every waiting call fails. */
@Throws(EmbedException::class)
public fun LingaraEmbed.tokenUnavailable(reason: String): Unit = handle.tokenUnavailable(reason)

/** Switches to, or re-points, the Http token source: after the player signs in. */
@Throws(EmbedException::class)
public fun LingaraEmbed.setTokenEndpoint(
    url: String,
    headers: List<Header> = emptyList(),
): Unit = handle.setTokenEndpoint(url, headers)

/** Drops the stream's connection, keeping its cursor: libGDX's `pause()`. */
@Throws(EmbedException::class)
public fun LingaraEmbed.streamPause(): Unit = handle.streamPause()

/** Reopens the stream from the kept cursor: libGDX's `resume()`. */
@Throws(EmbedException::class)
public fun LingaraEmbed.streamResume(): Unit = handle.streamResume()

/** Sends a scene change; the request id its `WorldSent` echoes. */
@Throws(EmbedException::class)
public fun LingaraEmbed.sendContext(context: WorldContext): Long = handle.sendContext(context)
