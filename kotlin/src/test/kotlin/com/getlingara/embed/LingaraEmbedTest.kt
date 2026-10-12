package com.getlingara.embed

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue
import kotlin.test.fail

class LingaraEmbedTest {
    private val case = "ek5.demo-plan-sends-nothing"
    private val plan = "8f0e2c1a-0000-4000-8000-0000000000de"

    private fun startAgainst(serve: ConformanceServe): LingaraEmbed {
        val embed = LingaraEmbed.start(EmbedConfig(baseUrl = serve.url, streamOnStart = false))
        embed.provideTokenFor(Secret("lgr_et_conformance0001"), 900)
        return embed
    }

    private fun LingaraEmbed.firstPractice(withinMs: Long): EmbedEvent.Practice {
        val deadline = System.currentTimeMillis() + withinMs
        while (System.currentTimeMillis() < deadline) {
            val events = drain(256)
            assertTrue(events.size <= 256)
            events.filterIsInstance<EmbedEvent.Practice>().firstOrNull { it.snapshot.phase == PracticePhase.UNANSWERED }?.let { return it }
            Thread.sleep(5)
        }
        fail("no unanswered Practice event within $withinMs ms")
    }

    // 1.10.26ae AC4: against conformance-server serve armed with one EK case,
    // the kit starts, drains in bounded order, answers a practice question in
    // the same call, and closes within 1 s.
    @Test
    fun startDrainAnswerAndCloseOnTheJvm() {
        ConformanceServe().use { serve ->
            serve.arm(case)
            val embed = startAgainst(serve)
            val session = embed.practiceStart(PracticeSource.Plan(plan, false))
            val practice = embed.firstPractice(2_000)
            assertEquals(session, practice.session)
            val graded = embed.practiceAnswer(session, "ni hao")
            assertEquals(PracticeOutcome.CORRECT, graded.lastResult?.outcome)
            embed.practiceFinish(session)
            val begun = System.nanoTime()
            assertTrue(embed.closeAndWait(1_000), "the core's thread joined")
            assertTrue((System.nanoTime() - begun) / 1_000_000 < 1_500)
            embed.close()
            val verdict = serve.finish(case)
            assertTrue("\"pass\"" in verdict, verdict)
        }
    }

    // 1.10.26ae D3: drain(max) returns at most max events, and drain(0)
    // takes none.
    @Test
    fun drainIsBounded() {
        LingaraEmbed.start(EmbedConfig(baseUrl = "http://127.0.0.1:9", streamOnStart = false)).use { embed ->
            embed.provideTokenFor(Secret("lgr_et_conformance0001"), 900)
            val first = embed.practiceStart(PracticeSource.Plan(plan, false))
            val second = embed.practiceStart(PracticeSource.Plan(plan, false))
            assertTrue(second > first, "ids rise")
            assertTrue(embed.drain(0).isEmpty(), "drain(0) takes nothing")
            val deadline = System.currentTimeMillis() + 2_000
            val seen = mutableListOf<EmbedEvent>()
            while (seen.size < 2 && System.currentTimeMillis() < deadline) {
                val one = embed.drain(1)
                assertTrue(one.size <= 1)
                seen += one
            }
            assertTrue(seen.isNotEmpty(), "events arrived")
        }
    }

    // 1.10.26ae AC18: the kit's token, then the game's suffix when one is set.
    @Test
    fun theKitTokenPrecedesTheGameSuffix() {
        val v = LingaraEmbed.VERSION
        assertEquals("lingara-embed-kotlin/$v my-game/1.4", userAgentSuffix(KitSurface.KOTLIN, "my-game/1.4"))
        assertEquals("lingara-embed-java/$v my-game/1.4", userAgentSuffix(KitSurface.JAVA, "my-game/1.4"))
        assertEquals("lingara-embed-kotlin/$v", userAgentSuffix(KitSurface.KOTLIN, null))
        assertEquals("lingara-embed-java/$v", userAgentSuffix(KitSurface.JAVA, " "))
        assertEquals(System.getProperty("lingara.version"), v)
    }

    // 1.10.26ae D7: a TokenFetcher beside an Http token source is refused.
    @Test
    fun aFetcherCannotSitBesideAnHttpSource() {
        val http = TokenSourceConfig.Http("https://game.example/token", emptyList())
        val refused = runCatching { LingaraEmbed.start(EmbedConfig(tokenSource = http), TokenFetcher { error("never") }) }
        assertTrue(refused.exceptionOrNull() is IllegalArgumentException)
    }
}
