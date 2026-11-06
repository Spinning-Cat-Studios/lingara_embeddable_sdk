package com.getlingara.embed

import java.io.File
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

class ProceduralDialogueTest {
    private val root = File(System.getProperty("lingara.root"))

    private fun vector(name: String): String = File(root, "conformance/vectors/dialogue/$name").readText().trimEnd()

    // 3.11.26g's ferryman, written with the Kotlin DSL.
    private fun ferryman(): ProceduralDialogue =
        proceduralDialogue("the_ferryman", "The player needs to cross the river before dark") {
            title = "The ferryman"
            pedagogy = Pedagogy.SUBTLE
            ending("crossed", "Got across")
            ending("turned_back")
            npc("old_li", "Old Li") {
                persona("a talkative ferryman who knows every family on the river")
                disposition(0.7, 0.4)
            }
            npc("ma_ren", "Ma Ren") {
                persona("a brisk ferrywoman who wants to cast off before dark")
                disposition(0.5, 0.7)
            }
            npc("boss_wu", "Boss Wu") {
                persona("the boat owner, who talks only about money")
                disposition(0.2, 0.95)
            }
            fact("fare_paid", false)
            fact("flood_rumour", "the upper weir cracked last spring")
            fact("family_name", "the Chens of Willow Bend")
            disclosable("flood_rumour", "family_name")
            topic("cargo", "what the player is carrying across")
            topic("fare", "the price of the crossing")
            topic("flood", "the flood on the upper river") { discloses("flood_rumour") }
            topic("family", "the family waiting on the far bank") {
                openWhen("fare_paid")
                discloses("family_name")
            }
            topic("smuggling", "the crates nobody asks about") { npcs("boss_wu") }
            forbid("the ferryman's debts")
            goal(
                "fare_agreed",
                "a fare both sides accept",
                predicate = "fare_paid",
                judgedWhen = "The player and the ferryman agree on a fare.",
            )
            atoms(AtomKind.OPEN, AtomKind.ELICIT, AtomKind.REVEAL, AtomKind.PRACTISE, AtomKind.CLOSE)
            prefer(AtomKind.REVEAL, 0.3)
            prefer(AtomKind.PRACTISE, 0.8)
            horizon(2, 1, 10)
            termination(40, 12) {
                success("fare_agreed", "crossed")
                stall(6, "turned_back")
                onLimit("turned_back")
            }
        }

    // 3.11.26j AC14: the DSL's text, through canonicalDialogue, is the golden
    // bytes, and the core's own validation accepts it.
    @Test
    fun `procedural dsl canonicalises to the golden bytes`() {
        val canonical = ferryman().canonicalJson()
        assertEquals(vector("ferryman.procedural.canonical.json"), canonical)
        val embed = LingaraEmbed.start(EmbedConfig(baseUrl = "http://127.0.0.1:9", streamOnStart = false))
        try {
            embed.dialogueValidate(canonical)
        } finally {
            embed.close()
        }
        assertFailsWith<EmbedException> { canonicalDialogue("{\"schema\":\"dialogue.v2\",\"kind\":\"procedural\"}") }
    }
}
