package com.getlingara.embed

import java.util.concurrent.ExecutorService
import java.util.concurrent.Executors

/**
 * Answers the core's `TokenNeeded` from JVM code (ADR 1.10.26ae D7): for a
 * game whose session lives in an OkHttp cookie jar or an auth SDK. It runs
 * on a daemon thread [LingaraEmbed] owns, never on the game thread. A thrown
 * exception becomes `tokenUnavailable` with its message.
 */
public fun interface TokenFetcher {
    public fun fetch(reason: TokenReason): IssuedToken
}

/**
 * A token and exactly one of its expiry forms: [expiresAt] (RFC 3339, the
 * server kits' handoff) goes to `provideToken`, [expiresInSecs] to
 * `provideTokenFor`. [toString] redacts the token.
 */
public class IssuedToken private constructor(
    public val token: Secret,
    public val expiresAt: String?,
    public val expiresInSecs: Long?,
) {
    override fun toString(): String = "IssuedToken(token=${Secret.REDACTED}, expiresAt=$expiresAt, expiresInSecs=$expiresInSecs)"

    public companion object {
        /** Valid until [expiresAt], an RFC 3339 instant on the game's clock. */
        @JvmStatic
        public fun expiringAt(
            token: Secret,
            expiresAt: String,
        ): IssuedToken = IssuedToken(token, expiresAt, null)

        /** Valid for [expiresInSecs] from now: the mint's `expires_in`. */
        @JvmStatic
        public fun expiringIn(
            token: Secret,
            expiresInSecs: Long,
        ): IssuedToken = IssuedToken(token, null, expiresInSecs)
    }
}

/** The one daemon thread a fetcher runs on, and what it does with a token. */
internal class TokenFetchThread(
    private val fetcher: TokenFetcher,
    private val handle: EmbedHandle,
) {
    private val executor: ExecutorService =
        Executors.newSingleThreadExecutor { task -> Thread(task, "lingara-embed-token").apply { isDaemon = true } }

    /** The core raises one `TokenNeeded` per outstanding need, so a need is fetched once. */
    fun submit(reason: TokenReason) {
        executor.execute { fetchAndProvide(reason) }
    }

    @Suppress("TooGenericExceptionCaught")
    private fun fetchAndProvide(reason: TokenReason) {
        try {
            val issued = fetcher.fetch(reason)
            val at = issued.expiresAt
            if (at != null) {
                handle.provideToken(issued.token, at)
            } else {
                handle.provideTokenFor(issued.token, issued.expiresInSecs ?: 0L)
            }
        } catch (e: Exception) {
            // A closed handle refuses this too, and then nothing is waiting.
            runCatching { handle.tokenUnavailable(e.message ?: e.javaClass.name) }
        }
    }

    fun shutdown() {
        executor.shutdownNow()
    }
}
