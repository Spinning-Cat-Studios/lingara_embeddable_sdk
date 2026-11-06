//! The Rust builder (ADR 28.10.26g D1, D3, D5): AC1 and AC4.

use serde_json::Value;

use crate::build::{AtomKind, Checkpoint, Dialogue, Edge, Fact, Forbid, Goal, Npc, Outcome, Procedural, Target, Termination, Topic};
use crate::procedural::ProceduralDialogue;
use crate::{DialogueIr, IrError, Level, Pedagogy, from_json, validate};

/// A file of `conformance/vectors/dialogue/`, read at run time so the
/// packaged crate never needs it to compile.
pub(super) fn vector(name: &str) -> String {
    let path = format!("{}/../conformance/vectors/dialogue/{name}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("{path}: {err}"))
}

/// N2's two-keepers dialogue, written with the builder.
fn two_keepers() -> Result<DialogueIr, IrError> {
    Dialogue::new("night_market_keepers", "suspicious_of_player")
        .title("The two keepers")
        .pedagogy(Pedagogy::None)
        .npc(Npc::new("old_wei", "Old Wei").persona("a retired ferryman who distrusts strangers").speaks("zh", Level::NATIVE, Level::NATIVE))
        .fact(Fact::number("trust").default(0))
        .fact(Fact::string("eastern_gate"))
        .checkpoint(
            Checkpoint::new("suspicious_of_player", "Size the stranger up; warm to them only if they explain why they are here")
                .edge(
                    Edge::new("asks_about_brother", "reveal_gate")
                        .predicate("trust >= 0.4")
                        .when("the player has asked about their missing brother"),
                )
                .edge(Edge::new("turns_hostile", Outcome::Failure).when("the player threatens or insults Old Wei")),
        )
        .checkpoint(
            Checkpoint::new("reveal_gate", "Tell the player the eastern gate is guarded, reluctantly")
                .discloses("eastern_gate")
                .edge(Edge::new("thanks", Outcome::Success).when("the player thanks Old Wei or says goodbye")),
        )
        .build()
}

/// D1's worked dialogue, extended as D4 describes: two NPCs on one graph,
/// a `0.4` and a `0` default, an edge to each outcome, a cycle through a
/// judged edge, and goals with a quote, an accent and a CJK place name.
pub(super) fn missing_brother() -> Result<DialogueIr, IrError> {
    Dialogue::new("missing_brother", "introduction")
        .title("The missing brother")
        .pedagogy(Pedagogy::None)
        .npc(
            Npc::new("elena", "Elena")
                .persona("a farmer who distrusts soldiers")
                .speaks("es", Level::NATIVE, Level::NATIVE)
                .speaks("en", Level::cefr("A2")?, Level::cefr("B1")?),
        )
        .npc(
            Npc::new("tomas", "Tomás")
                .persona("Elena's neighbour, a retired soldier")
                .speaks("es", Level::NATIVE, Level::NATIVE)
                .speaks("en", Level(6), Level(7)),
        )
        .fact(Fact::number("trust").default(0))
        .fact(Fact::number("suspicion").default(0.4))
        .fact(Fact::boolean("alarm_raised").default(false))
        .fact(Fact::string("eastern_gate"))
        .checkpoint(
            Checkpoint::new("introduction", "Elena is wary and asks who the player is")
                .edge(Edge::new("alarm", Outcome::Failure).predicate("alarm_raised"))
                .edge(Edge::new("explained", "ask_about_brother").when("the player says who they are and why they are here")),
        )
        .checkpoint(
            Checkpoint::new("ask_about_brother", "Admit she saw soldiers ride toward \"the old mill\"; reveal the gate only if she trusts the player")
                .discloses("eastern_gate")
                .edge(Edge::new("gate", Outcome::Success).predicate("trust >= 0.4").when("the player asks where soldiers might take a prisoner"))
                .edge(Edge::new("neighbour", "ask_tomas").when("the player asks who else saw the soldiers"))
                .edge(Edge::new("leaves", Outcome::Abandoned).when("the player walks away")),
        )
        .checkpoint(
            Checkpoint::new("ask_tomas", "Tomás says the soldiers spoke of the road to 長安")
                .edge(Edge::new("suspicious", Outcome::Failure).predicate("suspicion > 0.5"))
                .edge(Edge::new("back_to_elena", "ask_about_brother").when("the player returns to Elena with what Tomás said")),
        )
        .build()
}

// 28.10.26g AC1
#[test]
fn the_worked_dialogues_match_the_golden_bytes() {
    let cases = [(two_keepers(), "two-keepers.comparison.json"), (missing_brother(), "missing-brother.comparison.json")];
    for (ir, name) in cases {
        let ir = ir.unwrap();
        assert_eq!(ir.to_canonical_json(), vector(name), "{name}");
        // The vector itself is a dialogue the core accepts.
        validate(&from_json(&vector(name)).unwrap()).unwrap();
    }
}

/// The label table every kit reads (D3).
fn labels() -> Value {
    serde_json::from_str(&vector("level-labels.json")).unwrap()
}

/// The path a two-keepers built with one change is refused at.
fn refused(ir: Result<DialogueIr, IrError>) -> String {
    ir.expect_err("built").path
}

// 28.10.26g AC4
#[test]
fn levels_defaults_and_build_refusals() {
    let table = labels();
    for (scale, map) in [("cefr", Level::cefr as fn(&str) -> _), ("jlpt", Level::jlpt)] {
        for (label, level) in table[scale].as_object().unwrap() {
            assert_eq!(map(label).unwrap(), Level(level.as_u64().unwrap() as u8), "{scale} {label}");
        }
    }
    assert_eq!(Level::NATIVE, Level(table["native"].as_u64().unwrap() as u8));
    assert_eq!(Level::NATIVE, Level(9));
    assert!(Level::cefr("D1").is_err());
    assert!(Level::jlpt("N6").is_err());
    assert!(Level::cefr("a2").is_err());

    // D2's builder range: each refused at its own path.
    for x in [0.00001, 1_000_000_000.5, 0.12345, 9_007_199_254_740_992.0] {
        let ir = Dialogue::new("d", "a")
            .npc(Npc::new("n", "N"))
            .fact(Fact::number("x").default(x))
            .checkpoint(Checkpoint::new("a", "A").edge(Edge::new("e", Outcome::Success).when("w")))
            .build();
        assert_eq!(refused(ir), "facts[0].default", "{x}");
    }
    // …and its edges accepted.
    for x in [0.0001, 12345.6789, 9_007_199_254_740_991.0, -3.0] {
        let ir = Dialogue::new("d", "a")
            .npc(Npc::new("n", "N"))
            .fact(Fact::number("x").default(x))
            .checkpoint(Checkpoint::new("a", "A").edge(Edge::new("e", Outcome::Success).when("w")))
            .build();
        assert!(ir.is_ok(), "{x}");
    }

    // What `validate` refuses, `build()` refuses, with validate's path.
    let base = || Dialogue::new("d", "a").npc(Npc::new("n", "N"));
    let judged = || Edge::new("e", Outcome::Success).when("w");
    assert_eq!(refused(base().checkpoint(Checkpoint::new("a", "A").edge(Edge::new("e", "nowhere"))).build()), "checkpoints[0].edges[0].to");
    assert_eq!(refused(base().checkpoint(Checkpoint::new("a", "A").edge(judged().predicate("trst"))).build()), "checkpoints[0].edges[0].predicate");
    assert_eq!(refused(base().fact(Fact::number("x").default(true)).checkpoint(Checkpoint::new("a", "A").edge(judged())).build()), "facts[0].default");
    assert_eq!(refused(Dialogue::new("Bad-Id", "a").npc(Npc::new("n", "N")).checkpoint(Checkpoint::new("a", "A")).build()), "id");
    let speaks_ten = Npc::new("n", "N").speaks("en", Level(10), Level(1));
    assert!(Dialogue::new("d", "a").npc(speaks_ten).checkpoint(Checkpoint::new("a", "A").edge(judged())).build().is_err());
}

/// AC21's two documents, which the C# and GDScript builders also write: one
/// on the v1 shorthands, one with named endings (ADR 3.11.26g §11.5).
pub(super) fn gate_keeper(named: bool) -> Result<DialogueIr, IrError> {
    let (pass, refuse): (crate::build::Target, crate::build::Target) =
        if named { (Target::ending("passed"), Target::ending("fled")) } else { (Outcome::Success.into(), Outcome::Failure.into()) };
    let dialogue = Dialogue::new("gate_keeper", "greet").npc(Npc::new("guard", "Guard"));
    let dialogue = if named { dialogue.ending("passed", "Let through").ending("fled", "Ran off") } else { dialogue };
    dialogue
        .checkpoint(
            Checkpoint::new("greet", "Ask the player their business at the gate")
                .edge(Edge::new("pass", pass).when("the player gives a good reason"))
                .edge(Edge::new("refuse", refuse).when("the player is rude to the guard"))
                .edge(Edge::new("leave", Outcome::Abandoned).when("the player walks away")),
        )
        .build()
}

// 3.11.26g AC21
#[test]
fn endings_write_v2_and_shorthands_stay_v1() {
    let v1 = gate_keeper(false).unwrap();
    assert_eq!(v1.to_canonical_json(), vector("gate-keeper.v1.canonical.json").trim_end());
    assert!(v1.to_canonical_json().starts_with(r#"{"checkpoints""#) && v1.schema == crate::SCHEMA_V1);
    let v2 = gate_keeper(true).unwrap();
    assert_eq!(v2.to_canonical_json(), vector("gate-keeper.v2.canonical.json").trim_end());
    assert_eq!(v2.schema, crate::SCHEMA_V2);
    // An edge to an undeclared ending is refused once endings are declared.
    let undeclared = Dialogue::new("d", "a")
        .npc(Npc::new("n", "N"))
        .ending("won", "")
        .checkpoint(Checkpoint::new("a", "A").edge(Edge::new("e", Outcome::Success).when("w")))
        .build();
    assert_eq!(refused(undeclared), "checkpoints[0].edges[0].to");
}

/// 3.11.26g's ferryman, written with the procedural builder; `edit` changes
/// one part so a refusal can be shown at its own path.
pub(super) fn ferryman(edit: impl FnOnce(Procedural) -> Procedural) -> Result<ProceduralDialogue, IrError> {
    use AtomKind::{Close, Elicit, Open, Practise, Reveal};
    let built = Procedural::new("the_ferryman", "The player needs to cross the river before dark")
        .title("The ferryman")
        .pedagogy(Pedagogy::Subtle)
        .ending("crossed", "Got across")
        .ending("turned_back", "")
        .npc(Npc::new("old_li", "Old Li").persona("a talkative ferryman who knows every family on the river").disposition(0.7, 0.4))
        .npc(Npc::new("ma_ren", "Ma Ren").persona("a brisk ferrywoman who wants to cast off before dark").disposition(0.5, 0.7))
        .npc(Npc::new("boss_wu", "Boss Wu").persona("the boat owner, who talks only about money").disposition(0.2, 0.95))
        .fact(Fact::boolean("fare_paid").default(false))
        .fact(Fact::string("flood_rumour").default("the upper weir cracked last spring"))
        .fact(Fact::string("family_name").default("the Chens of Willow Bend"))
        .disclosable(["flood_rumour", "family_name"])
        .topic(Topic::new("cargo", "what the player is carrying across"))
        .topic(Topic::new("fare", "the price of the crossing"))
        .topic(Topic::new("flood", "the flood on the upper river").discloses(["flood_rumour"]))
        .topic(Topic::new("family", "the family waiting on the far bank").when("fare_paid").discloses(["family_name"]));
    edit(built)
        .topic(Topic::new("smuggling", "the crates nobody asks about").npcs(["boss_wu"]))
        .forbid(Forbid::new("the ferryman's debts"))
        .goal(Goal::new("fare_agreed", "a fare both sides accept").predicate("fare_paid").when("The player and the ferryman agree on a fare."))
        .atoms([Open, Elicit, Reveal, Practise, Close])
        .prefer(Reveal, 0.3)
        .prefer(Practise, 0.8)
        .horizon(2, 1, 10)
        .termination(Termination::new(40, 12).success("fare_agreed", "crossed").stall(6, "turned_back").on_limit("turned_back"))
        .build()
}

fn canonical(doc: ProceduralDialogue) -> String {
    crate::Document::Procedural(doc).to_canonical_json()
}

// 3.11.26j AC12
#[test]
fn procedural_builder_writes_the_golden_bytes() {
    let golden = vector("ferryman.procedural.canonical.json");
    assert_eq!(canonical(ferryman(|p| p).unwrap()), golden.trim_end());
    // The golden bytes are the vector's own canonical form.
    assert_eq!(crate::load_document(&vector("ferryman.procedural.json")).unwrap().0.to_canonical_json(), golden.trim_end());

    // A Reveal with no topic that discloses anything, and a `discloses` outside the pool.
    let silent = Procedural::new("p", "A premise")
        .npc(Npc::new("n", "N"))
        .ending("done", "")
        .topic(Topic::new("t", "a topic"))
        .atoms([AtomKind::Open, AtomKind::Reveal, AtomKind::Close])
        .termination(Termination::new(10, 4))
        .build()
        .unwrap_err();
    assert_eq!((silent.rule.as_str(), silent.path.as_str()), ("atom_set", "procedural.atoms[1]"), "{silent}");
    let escaped = ferryman(|p| p.topic(Topic::new("debts", "money owed").discloses(["fare_paid"]))).unwrap_err();
    assert_eq!((escaped.rule.as_str(), escaped.path.as_str()), ("pool_escape", "procedural.topics[4].discloses[0]"), "{escaped}");
}

/// A one-checkpoint static dialogue whose NPC is `npc`.
fn steered(npc: Npc) -> Result<DialogueIr, IrError> {
    Dialogue::new("gate", "greet")
        .npc(npc)
        .checkpoint(Checkpoint::new("greet", "Ask the player their business").edge(Edge::new("pass", Outcome::Success).when("the player gives a reason")))
        .build()
}

// 3.11.26j AC30
#[test]
fn a_disposition_makes_a_static_document_v2() {
    // The two documents are vectors, so the C# builder is held to the same bytes (AC31).
    let plain = steered(Npc::new("guard", "Guard")).unwrap();
    assert_eq!(plain.schema, crate::SCHEMA_V1, "no disposition: byte-identical v1");
    assert_eq!(plain.to_canonical_json(), vector("guard.v1.canonical.json").trim_end());
    let led = steered(Npc::new("guard", "Guard").disposition(0.2, 0.8)).unwrap();
    assert_eq!(led.schema, crate::SCHEMA_V2, "one disposition makes it v2");
    assert_eq!(led.to_canonical_json(), vector("guard.v2.canonical.json").trim_end());
    let wild = steered(Npc::new("guard", "Guard").disposition(1.5, 0.8)).unwrap_err();
    assert_eq!((wild.rule.as_str(), wild.path.as_str()), ("number_range", "npcs[0].disposition.suggestibility"), "{wild}");
}
