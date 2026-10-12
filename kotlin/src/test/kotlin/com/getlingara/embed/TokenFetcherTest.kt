package com.getlingara.embed

import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNotEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class TokenFetcherTest {
    private val token = "lgr_et_fetchedTokenValue0001"
    private val offline = EmbedConfig(baseUrl = "http://127.0.0.1:9", streamOnStart = false, tokenWaitTimeoutMs = 5_000)

    private fun LingaraEmbed.drainUntil(
        withinMs: Long,
        done: (List<EmbedEvent>) -> Boolean,
    ): List<EmbedEvent> {
        val seen = mutableListOf<EmbedEvent>()
        val deadline = System.currentTimeMillis() + withinMs
        while (System.currentTimeMillis() < deadline) {
            seen += drain()
            if (done(seen)) return seen
            Thread.sleep(5)
        }
        fail("not done within $withinMs ms: $seen")
    }

    // 1.10.26ae AC7: with a fetcher set, TokenNeeded never reaches the game,
    // the fetch runs on its own thread, its token reaches provideToken, and a
    // thrown exception becomes tokenUnavailable; no toString() renders a
    // token or a header value.
    @Test
    fun theFetcherRunsOffThreadAndNeverRendersTheToken() {
        val fetchedOn = CountDownLatch(1)
        var fetchThread: Thread? = null
        val fetcher =
            TokenFetcher { reason ->
                assertEquals(TokenReason.INITIAL, reason)
                fetchThread = Thread.currentThread()
                fetchedOn.countDown()
                IssuedToken.expiringIn(Secret(token), 900)
            }
        LingaraEmbed.start(offline, fetcher).use { embed ->
            val events = embed.drainUntil(2_000) { fetchedOn.count == 0L && embed.status().needsToken.not() }
            assertTrue(events.none { it is EmbedEvent.TokenNeeded }, "TokenNeeded was intercepted")
            assertNotEquals(Thread.currentThread(), fetchThread)
            assertEquals("lingara-embed-token", fetchThread?.name)
        }
        theThrownFetchBecomesTokenUnavailable()
        nothingRendersTheToken()
    }

    private fun theThrownFetchBecomesTokenUnavailable() {
        val release = CountDownLatch(1)
        val fetcher =
            TokenFetcher {
                release.await(2, TimeUnit.SECONDS)
                throw IllegalStateException("the game is offline")
            }
        LingaraEmbed.start(offline, fetcher).use { embed ->
            val request = embed.sendContext(WorldContext("the market", "en", "zh", 2, null, emptyList(), false))
            embed.drain()
            Thread.sleep(200)
            release.countDown()
            val events = embed.drainUntil(6_000) { seen -> seen.any { it is EmbedEvent.Error } }
            val error = events.filterIsInstance<EmbedEvent.Error>().first().error
            assertEquals(ErrorKind.TOKEN_UNAVAILABLE, error.kind)
            assertEquals(request, error.request)
        }
    }

    private fun nothingRendersTheToken() {
        val secret = Secret(token)
        val header = Header("cookie", Secret("session=abc"))
        val rendered =
            listOf(
                secret.toString(),
                IssuedToken.expiringAt(secret, "2026-10-01T12:15:00Z").toString(),
                header.toString(),
                Command.ProvideToken(secret, "2026-10-01T12:15:00Z").toString(),
                EmbedConfig(tokenSource = TokenSourceConfig.Http("https://game.example/t", listOf(header))).toString(),
            )
        for (text in rendered) {
            assertFalse(token in text || "session=abc" in text, text)
        }
        assertTrue(Secret.REDACTED in header.toString())
        assertEquals(token, secret.exposeSecret())
    }
}
