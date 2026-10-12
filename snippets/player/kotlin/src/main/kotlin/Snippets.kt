// The Kotlin player kit as the documentation site shows it. Each marked
// region is one key; the docs join a key's regions in file order. No region
// holds a client secret: a game holds only the embed token its server mints.

import com.getlingara.embed.DialogueConfig
import com.getlingara.embed.EmbedConfig
import com.getlingara.embed.EmbedEvent
import com.getlingara.embed.Header
import com.getlingara.embed.LingaraEmbed
import com.getlingara.embed.Npc
import com.getlingara.embed.NpcTurnUpdate
import com.getlingara.embed.PracticeSource
import com.getlingara.embed.Secret
import com.getlingara.embed.TokenSourceConfig
import com.getlingara.embed.dialogueOpen
import com.getlingara.embed.dialogueSay
import com.getlingara.embed.practiceAnswer
import com.getlingara.embed.practiceStart

class Snippets(
    session: String,
) {
    // lingara:begin tokenSource
    // Your server mints the embed token; the kit POSTs {} here whenever it needs one.
    private val lingara =
        LingaraEmbed.start(
            EmbedConfig(
                tokenSource =
                    TokenSourceConfig.Http(
                        url = "https://my-game.example/lingara/token",
                        headers = listOf(Header("Authorization", Secret("Bearer $session"))),
                    ),
                userAgentSuffix = "my-game/1.4",
            ),
        )
    // lingara:end

    private var practice: Long? = null
    private var npc: Long? = null
    var bubble: String = ""

    // lingara:begin poll
    // In render(), on the game thread: one crossing, at most 256 events.
    fun render() {
        lingara.drain(256) { event ->
            when (event) {
                is EmbedEvent.Practice -> showPractice(event.snapshot.question?.prompt)
                is EmbedEvent.NpcTurn -> onNpcTurn(event.update)
                is EmbedEvent.Error -> println("Lingara error: ${event.error.message}")
                else -> Unit
            }
        }
    }

    // In dispose(): stops the core and frees it.
    fun dispose() {
        lingara.close()
    }
    // lingara:end

    // lingara:begin practice
    fun startPractice() {
        practice = lingara.practiceStart(PracticeSource.Request("ordering tea", "en", "zh", 2, listOf("tea")))
    }

    fun answer(text: String) {
        val session = practice ?: return
        val graded = lingara.practiceAnswer(session, text) // graded in the same call
        println("${graded.lastResult?.outcome}: ${graded.lastResult?.expected}")
    }
    // lingara:end

    // lingara:begin dialogue
    fun talkToMei(line: String) {
        val dialogue = npc ?: lingara.dialogueOpen(DialogueConfig(Npc("Mei", "a tea seller"), "the tea house", "en", "zh", 2, emptyList()))
        npc = dialogue
        lingara.dialogueSay(dialogue, line)
    }

    private fun onNpcTurn(update: NpcTurnUpdate) {
        when (update) {
            is NpcTurnUpdate.Opened -> bubble = ""
            is NpcTurnUpdate.Fragment -> bubble += update.text
            is NpcTurnUpdate.Finished -> bubble = update.text
            else -> Unit
        }
    }
    // lingara:end

    private fun showPractice(prompt: String?) {
        println(prompt)
    }
}
