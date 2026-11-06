package com.getlingara.embed

/** Marks the procedural DSL's receivers, so an inner block cannot reach an outer one's members. */
@DslMarker
public annotation class ProceduralDsl

/** A cast member's persona, languages and disposition. */
@ProceduralDsl
public class NpcSpec internal constructor() {
    private var persona: String? = null
    private val languages = mutableListOf<String>()
    private var disposition: String? = null

    public fun persona(text: String) {
        persona = text
    }

    /** One language, at speaking and comprehension levels 1–9. */
    public fun speaks(
        code: String,
        speaking: Int,
        comprehension: Int,
    ) {
        languages += obj("code" to quote(code), "speaking" to num(speaking), "comprehension" to num(comprehension))
    }

    /** How the NPC steers, each 0–1: how readily it follows the player off its goal, and how hard it drives toward it. */
    public fun disposition(
        suggestibility: Double,
        initiative: Double,
    ) {
        disposition = obj("suggestibility" to num(suggestibility), "initiative" to num(initiative))
    }

    internal fun json(
        id: String,
        name: String,
    ): String {
        val spoken = if (languages.isEmpty()) null else arr(languages)
        return obj(
            "id" to quote(id),
            "name" to quote(name),
            "persona" to persona?.let(::quote),
            "languages" to spoken,
            "disposition" to disposition,
        )
    }
}

/** A topic's audience, condition and disclosures. */
@ProceduralDsl
public class TopicSpec internal constructor() {
    private var condition: String? = null
    private var npcs: List<String>? = null
    private val disclosed = mutableListOf<String>()

    /** A K1 predicate over facts; the topic is open only while it holds. (`when` is a Kotlin keyword; the member stays `when`.) */
    public fun openWhen(predicate: String) {
        condition = predicate
    }

    /** The cast ids that may raise it; never called, every cast member. */
    public fun npcs(vararg ids: String) {
        npcs = ids.toList()
    }

    /** The facts a Reveal beat on this topic discloses: within the pool. */
    public fun discloses(vararg keys: String) {
        disclosed += keys
    }

    internal fun json(
        id: String,
        about: String,
    ): String {
        val discloses = if (disclosed.isEmpty()) null else arr(disclosed.map(::quote))
        return obj(
            "id" to quote(id),
            "about" to quote(about),
            "npcs" to npcs?.let { arr(it.map(::quote)) },
            "when" to condition?.let(::quote),
            "discloses" to discloses,
        )
    }
}

/** The rules that end a run, past its two limits. */
@ProceduralDsl
public class TerminationSpec internal constructor(
    private val maxTurns: Int,
    private val maxBeats: Int,
) {
    private var success: String? = null
    private var stall: String? = null
    private var onLimit: String? = null

    /** When [goal]'s predicate holds, the run ends on [ending]. */
    public fun success(
        goal: String,
        ending: String,
    ) {
        success = obj("goal" to quote(goal), "ending" to quote(ending))
    }

    /** [turns] turns with no crossing end the run on [ending]. */
    public fun stall(
        turns: Int,
        ending: String,
    ) {
        stall = obj("turns" to num(turns), "ending" to quote(ending))
    }

    /** What either limit ends on; never called, `abandoned`. */
    public fun onLimit(ending: String) {
        onLimit = quote(ending)
    }

    internal fun json(): String =
        obj("max_turns" to num(maxTurns), "max_beats" to num(maxBeats), "success" to success, "stall" to stall, "on_limit" to onLimit)
}
