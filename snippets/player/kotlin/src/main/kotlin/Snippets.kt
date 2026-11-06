// The Kotlin player kit as the documentation site shows it. Each marked
// region is one key; the docs join a key's regions in file order. No region
// holds a client secret: a game holds only the embed token its server mints.

import com.getlingara.embed.AtomKind
import com.getlingara.embed.DialogueConfig
import com.getlingara.embed.DialogueSource
import com.getlingara.embed.EmbedConfig
import com.getlingara.embed.EmbedEvent
import com.getlingara.embed.GraphSession
import com.getlingara.embed.Header
import com.getlingara.embed.LingaraEmbed
import com.getlingara.embed.Npc
import com.getlingara.embed.NpcTurnUpdate
import com.getlingara.embed.Pedagogy
import com.getlingara.embed.PracticeSource
import com.getlingara.embed.ProceduralDialogue
import com.getlingara.embed.Secret
import com.getlingara.embed.TokenSourceConfig
import com.getlingara.embed.dialogueLoad
import com.getlingara.embed.dialogueOpen
import com.getlingara.embed.dialogueSay
import com.getlingara.embed.practiceAnswer
import com.getlingara.embed.practiceStart
import com.getlingara.embed.proceduralDialogue

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
                // The embed version this build was registered against: your server forwards it.
                embedVersion = "1.1.0",
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
                // Once per session, for you: log it, never draw it for the player.
                is EmbedEvent.UpdateAvailable -> println("Lingara update: latest ${event.latest}, deprecated ${event.deprecated}")
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

    // lingara:begin proceduralDialogue
    // A procedural dialogue: nobody writes the checkpoints. You draw the box (the
    // premise, what may come up, what the NPC may disclose, and when the run
    // stops) and Lingara's planner proposes the next few beats inside it. The core
    // checks every plan against this box, and ends the run on termination whatever
    // the planner does.
    fun marketGossip(): ProceduralDialogue =
        proceduralDialogue("market_gossip", "The player is buying tea and hears the market's news") {
            title = "Market gossip"
            pedagogy = Pedagogy.SUBTLE
            npc("mei", "Mei") {
                persona("a tea seller who hears every rumour")
                disposition(0.6, 0.5)
            }
            fact("tea_bought", false)
            fact("rumour", "the night market moves to the river next week")
            ending("bought", "Bought the tea")
            ending("left")
            disclosable("rumour")
            topic("tea", "the teas on the stall")
            topic("price", "what a cup costs")
            topic("news", "what people are saying in the market") { discloses("rumour") }
            forbid("the seller's family")
            goal("tea_paid", "a cup of tea, paid for", predicate = "tea_bought", judgedWhen = "The player agrees a price and pays.")
            atoms(AtomKind.OPEN, AtomKind.ELICIT, AtomKind.REVEAL, AtomKind.PRACTISE, AtomKind.CLOSE)
            termination(30, 10) {
                success("tea_paid", "bought")
                stall(6, "left")
                onLimit("left")
            }
        }

    // The core plans Mei's first beats before her first CheckpointReached; each
    // plan arrives as DialoguePlanned, and DialogueEnded carries the path taken.
    fun loadMei(): Long {
        val session =
            GraphSession(
                npc = "mei",
                sourceLang = "en",
                targetLang = "zh",
                level = 2,
                scene = null,
                facts = emptyMap(),
                checkpoint = null,
                window = emptyList(),
            )
        return lingara.dialogueLoad(DialogueSource.Ir(marketGossip().canonicalJson()), session)
    }
    // lingara:end

    private fun showPractice(prompt: String?) {
        println(prompt)
    }
}
