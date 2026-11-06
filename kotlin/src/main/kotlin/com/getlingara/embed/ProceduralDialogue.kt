@file:JvmName("ProceduralDialogues")

package com.getlingara.embed

// The first Kotlin dialogue builder (ADR 3.11.26j D8): a procedural document,
// always `dialogue.v2`, `kind: procedural`. Nobody writes its checkpoints:
// the DSL draws the box the planner fills. It writes plain JSON text, and
// `canonicalJson()` hands that text to the core for the crate's canonical
// form and its checks, so Java reaches the same bytes through it.

/** A procedural dialogue: [premise] is what the whole conversation is about. */
public fun proceduralDialogue(
    id: String,
    premise: String,
    block: ProceduralDialogueBuilder.() -> Unit,
): ProceduralDialogue = ProceduralDialogue(ProceduralDialogueBuilder(id, premise).apply(block).json())

/** A built document: [json] as written, [canonicalJson] what `DialogueLoad` sends. */
public class ProceduralDialogue internal constructor(
    public val json: String,
) {
    /** The crate's canonical form; refused with the crate's path and rule when the document is not one. */
    @Throws(EmbedException::class)
    public fun canonicalJson(): String = canonicalDialogue(json)
}

/** The box a procedural dialogue is planned in. */
@ProceduralDsl
public class ProceduralDialogueBuilder internal constructor(
    private val id: String,
    private val premise: String,
) {
    public var title: String? = null
    public var pedagogy: Pedagogy? = null
    private val npcs = mutableListOf<String>()
    private val facts = mutableListOf<String>()
    private val endings = mutableListOf<String>()
    private val disclosable = mutableListOf<String>()
    private val topics = mutableListOf<String>()
    private val forbidden = mutableListOf<String>()
    private val goals = mutableListOf<String>()
    private val atoms = mutableListOf<Pair<AtomKind, Double?>>()
    private var horizon: String? = null
    private var termination: String? = null

    public fun npc(
        id: String,
        name: String,
        block: NpcSpec.() -> Unit = {},
    ) {
        npcs += NpcSpec().apply(block).json(id, name)
    }

    /** A fact, typed by its default: a `Boolean`, a `Number` or a `String`. */
    public fun fact(
        key: String,
        default: Any,
    ) {
        val (type, value) =
            when (default) {
                is Boolean -> "boolean" to default.toString()
                is Number -> "number" to num(default)
                else -> "string" to quote(default.toString())
            }
        facts += obj("key" to quote(key), "type" to quote(type), "default" to value)
    }

    /** A named ending; [label] is what a person reads. */
    public fun ending(
        id: String,
        label: String? = null,
    ) {
        endings += obj("id" to quote(id), "label" to label?.let(::quote))
    }

    /** The fact keys a Reveal beat may disclose: the knowledge pool. */
    public fun disclosable(vararg keys: String) {
        disclosable += keys.map(::quote)
    }

    public fun topic(
        id: String,
        about: String,
        block: TopicSpec.() -> Unit = {},
    ) {
        topics += TopicSpec().apply(block).json(id, about)
    }

    /** A subject never to raise: planner guidance. No [npcs] is every cast member. */
    public fun forbid(
        about: String,
        vararg npcs: String,
    ) {
        forbidden += obj("about" to quote(about), "npcs" to if (npcs.isEmpty()) null else arr(npcs.map(::quote)))
    }

    /** A named success target: [predicate] the game decides, [judgedWhen] the judge does. */
    public fun goal(
        id: String,
        describe: String,
        predicate: String? = null,
        judgedWhen: String? = null,
    ) {
        goals +=
            obj(
                "id" to quote(id),
                "describe" to quote(describe),
                "predicate" to predicate?.let(::quote),
                "when" to judgedWhen?.let(::quote),
            )
    }

    /** The atoms the planner may compose, in order; `OPEN` and `CLOSE` are required. */
    public fun atoms(vararg kinds: AtomKind) {
        atoms += kinds.map { it to null }
    }

    /** A planner hint, 0–1, on an atom already listed. */
    public fun prefer(
        kind: AtomKind,
        prefer: Double,
    ) {
        atoms.replaceAll { if (it.first == kind) kind to prefer else it }
    }

    public fun horizon(
        planDepth: Int,
        sketchDepth: Int,
        maxNodes: Int,
    ) {
        horizon = obj("plan_depth" to num(planDepth), "sketch_depth" to num(sketchDepth), "max_nodes" to num(maxNodes))
    }

    /** When a run stops: both limits are required. */
    public fun termination(
        maxTurns: Int,
        maxBeats: Int,
        block: TerminationSpec.() -> Unit = {},
    ) {
        termination = TerminationSpec(maxTurns, maxBeats).apply(block).json()
    }

    internal fun json(): String {
        val atomList = atoms.map { (kind, prefer) -> obj("atom" to quote(kind.name.lowercase()), "prefer" to prefer?.let(::num)) }
        val spec =
            obj(
                "premise" to quote(premise),
                "disclosable" to arr(disclosable),
                "topics" to arr(topics),
                "forbidden" to if (forbidden.isEmpty()) null else arr(forbidden),
                "goals" to if (goals.isEmpty()) null else arr(goals),
                "atoms" to arr(atomList),
                "horizon" to horizon,
                "termination" to termination,
            )
        return obj(
            "schema" to quote("dialogue.v2"),
            "kind" to quote("procedural"),
            "id" to quote(id),
            "title" to title?.let(::quote),
            "pedagogy" to pedagogy?.let { quote(it.name.lowercase()) },
            "npcs" to arr(npcs),
            "facts" to arr(facts),
            "endings" to arr(endings),
            "procedural" to spec,
        )
    }
}
