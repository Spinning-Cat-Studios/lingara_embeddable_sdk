//! The Rust builder (ADR 28.10.26g D1, D3, D5): AC1 and AC4.

use serde_json::Value;

use crate::build::{Checkpoint, Dialogue, Edge, Fact, Npc, Outcome};
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
