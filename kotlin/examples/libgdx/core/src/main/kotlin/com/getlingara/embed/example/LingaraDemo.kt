package com.getlingara.embed.example

import com.badlogic.gdx.ApplicationAdapter
import com.badlogic.gdx.Gdx
import com.badlogic.gdx.Input
import com.badlogic.gdx.InputAdapter
import com.badlogic.gdx.graphics.g2d.BitmapFont
import com.badlogic.gdx.graphics.g2d.SpriteBatch
import com.badlogic.gdx.utils.ScreenUtils
import com.getlingara.embed.DialogueConfig
import com.getlingara.embed.EmbedConfig
import com.getlingara.embed.EmbedEvent
import com.getlingara.embed.LingaraEmbed
import com.getlingara.embed.Npc
import com.getlingara.embed.NpcTurnUpdate
import com.getlingara.embed.PracticePhase
import com.getlingara.embed.PracticeSnapshot
import com.getlingara.embed.PracticeSource
import com.getlingara.embed.TokenSourceConfig
import com.getlingara.embed.WorldContext
import com.getlingara.embed.dialogueOpen
import com.getlingara.embed.dialogueSay
import com.getlingara.embed.dialogueSetLevel
import com.getlingara.embed.practiceAnswer
import com.getlingara.embed.practiceChoose
import com.getlingara.embed.practiceNext
import com.getlingara.embed.practiceReveal
import com.getlingara.embed.practiceStart
import com.getlingara.embed.sendContext
import com.getlingara.embed.streamPause
import com.getlingara.embed.streamResume

/**
 * One screen of the JVM kit (ADR 1.10.26ae D8): a token from K2's demo token
 * server through the core's Http source, a scene change, a practice session,
 * and an NPC whose reply streams into a bubble fragment by fragment.
 *
 * Keys: S walks to the other stall (a scene change), P starts a practice,
 * 1–4 choose an option, type and Enter to answer, N reveals or goes on,
 * T says a line to Mei, L switches her level between 2 and 4.
 */
class LingaraDemo : ApplicationAdapter() {
    private lateinit var lingara: LingaraEmbed
    private lateinit var batch: SpriteBatch
    private lateinit var font: BitmapFont
    private val scenes = listOf("the tea house", "the night market")
    private val lines = listOf("Hello!", "What do you recommend?", "How much is this?")
    private var scene = 0
    private var level = 2
    private var said = 0
    private var dialogue: Long? = null
    private var session: Long? = null
    private var snapshot: PracticeSnapshot? = null
    private var bubble = ""
    private var typed = ""
    private var status = "starting"
    private var tokenTaken = false
    private val started = System.nanoTime()

    override fun create() {
        batch = SpriteBatch()
        font = BitmapFont()
        val tokenUrl = System.getenv("LINGARA_DEMO_TOKEN_URL") ?: "http://127.0.0.1:8787/token"
        lingara =
            LingaraEmbed.start(
                EmbedConfig(
                    baseUrl = System.getenv("LINGARA_EMBED_BASE_URL"),
                    tokenSource = TokenSourceConfig.Http(tokenUrl, emptyList()),
                    userAgentSuffix = "lingara-libgdx-example/1.0",
                ),
            )
        Gdx.input.inputProcessor = Keys()
        walk(0)
    }

    override fun render() {
        lingara.drain(LingaraEmbed.DEFAULT_DRAIN) { event -> onEvent(event) }
        ScreenUtils.clear(0.1f, 0.12f, 0.16f, 1f)
        batch.begin()
        screen().forEachIndexed { n, line -> font.draw(batch, line, 20f, Gdx.graphics.height - 20f - n * 22f) }
        batch.end()
        exitWhenAsked()
    }

    override fun pause() {
        lingara.streamPause()
    }

    override fun resume() {
        lingara.streamResume()
    }

    override fun dispose() {
        lingara.close()
        batch.dispose()
        font.dispose()
    }

    private fun onEvent(event: EmbedEvent) {
        when (event) {
            is EmbedEvent.Practice -> if (event.session == session) snapshot = event.snapshot
            is EmbedEvent.NpcTurn -> onTurn(event.update)
            is EmbedEvent.WorldSent -> noteToken()
            is EmbedEvent.Error -> {
                status = "error: ${event.error.kind} ${event.error.message}"
                // An HTTP status means the request reached the server, bearing the token.
                if (event.error.status != null) noteToken()
            }
            is EmbedEvent.Stream -> status = "stream: ${event.state.javaClass.simpleName}"
            else -> Unit
        }
    }

    private fun onTurn(update: NpcTurnUpdate) {
        when (update) {
            is NpcTurnUpdate.Opened -> bubble = ""
            is NpcTurnUpdate.Fragment -> bubble += update.text
            is NpcTurnUpdate.Finished -> bubble = update.text
            else -> Unit
        }
    }

    private fun screen(): List<String> {
        val s = snapshot
        val question = s?.question
        return listOfNotNull(
            "Lingara ${LingaraEmbed.VERSION} — $status",
            "Scene: ${scenes[scene]} (S)   Mei's level: $level (L)",
            "Mei: $bubble",
            "Practice (P): ${s?.phase ?: "none"}",
            question?.prompt,
            question?.options?.mapIndexed { i, o -> "${i + 1}. $o" }?.joinToString("   "),
            s?.lastResult?.let { "${it.outcome}: ${it.expected}" },
            "> $typed",
        )
    }

    private fun walk(to: Int) {
        scene = to
        lingara.sendContext(WorldContext(scenes[scene], "en", "zh", level, Npc("Mei", "a tea seller"), emptyList(), false))
    }

    private fun talk() {
        val id = dialogue ?: lingara.dialogueOpen(DialogueConfig(Npc("Mei", "a tea seller"), scenes[scene], "en", "zh", level, emptyList()))
        dialogue = id
        lingara.dialogueSay(id, lines[said++ % lines.size])
    }

    private fun toggleLevel() {
        level = if (level == 2) 4 else 2
        dialogue?.let { lingara.dialogueSetLevel(it, level) }
    }

    private fun practise() {
        session = lingara.practiceStart(PracticeSource.Request("ordering tea", "en", "zh", level, emptyList()))
    }

    /** N: reveal an unanswered question, or go on from a reviewed one. */
    private fun next() {
        val id = session ?: return
        snapshot = if (snapshot?.phase == PracticePhase.UNANSWERED) lingara.practiceReveal(id) else lingara.practiceNext(id)
    }

    private fun answer() {
        val id = session ?: return
        snapshot = lingara.practiceAnswer(id, typed)
        typed = ""
    }

    private fun choose(option: Int) {
        val id = session ?: return
        snapshot = lingara.practiceChoose(id, option.toLong())
    }

    /**
     * Once, when the first request reaches the server, which it does only
     * with a token the Http source fetched: CI's smoke reads this line.
     */
    private fun noteToken() {
        if (tokenTaken) return
        tokenTaken = true
        println("lingara-libgdx-example: token taken")
    }

    /** CI's smoke: LINGARA_DEMO_EXIT_AFTER_SECS runs the screen that long, then exits cleanly. */
    private fun exitWhenAsked() {
        val after = System.getenv("LINGARA_DEMO_EXIT_AFTER_SECS")?.toLongOrNull() ?: return
        if ((System.nanoTime() - started) / 1_000_000_000 >= after) {
            println("lingara-libgdx-example: $status")
            Gdx.app.exit()
        }
    }

    private inner class Keys : InputAdapter() {
        override fun keyDown(keycode: Int): Boolean {
            when (keycode) {
                Input.Keys.S -> walk(1 - scene)
                Input.Keys.T -> talk()
                Input.Keys.L -> toggleLevel()
                Input.Keys.P -> practise()
                Input.Keys.N -> next()
                Input.Keys.ENTER -> answer()
                in Input.Keys.NUM_1..Input.Keys.NUM_4 -> choose(keycode - Input.Keys.NUM_1)
                else -> return false
            }
            return true
        }

        override fun keyTyped(character: Char): Boolean {
            if (character == '\b') typed = typed.dropLast(1) else if (!character.isISOControl()) typed += character
            return true
        }
    }
}
