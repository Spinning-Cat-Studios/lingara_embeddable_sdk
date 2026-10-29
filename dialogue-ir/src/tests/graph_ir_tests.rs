//! The IR and its load checks (ADR 28.10.26f D1, D2): AC1 and AC2.

use serde_json::{Value, json};

use crate::limits::MAX_IR_BYTES;
use crate::{FactValue, IrError, Warning, from_value, load, validate};

/// D1's worked example, as the ADR writes it.
const MISSING_BROTHER: &str = r#"{
  "schema": "dialogue.v1",
  "id": "missing_brother",
  "title": "The missing brother",
  "pedagogy": "none",
  "npcs": [
    { "id": "elena", "name": "Elena", "persona": "a farmer who distrusts soldiers",
      "languages": [ { "code": "es", "speaking": 9, "comprehension": 9 },
                     { "code": "en", "speaking": 2, "comprehension": 3 } ] }
  ],
  "facts": [ { "key": "trust", "type": "number", "default": 0 },
             { "key": "alarm_raised", "type": "boolean", "default": false },
             { "key": "eastern_gate", "type": "string" } ],
  "start": "introduction",
  "checkpoints": [
    { "id": "introduction", "goal": "Elena is wary and asks who the player is", "may_disclose": [],
      "edges": [ { "id": "alarm", "to": "failure", "predicate": "alarm_raised" },
                 { "id": "explained", "to": "ask_about_brother",
                   "when": "the player says who they are and why they are here" } ] },
    { "id": "ask_about_brother", "goal": "Admit she saw soldiers; reveal the gate only if she trusts the player",
      "may_disclose": ["eastern_gate"],
      "edges": [ { "id": "gate", "to": "success", "predicate": "trust >= 0.4",
                   "when": "the player asks where soldiers might take a prisoner" } ] }
  ]
}"#;

/// N2's `docs/api/dialogue/examples/two-keepers.json`, inline until Phase B
/// vendors the file (AC1).
const TWO_KEEPERS: &str = r#"{
  "schema": "dialogue.v1",
  "id": "night_market_keepers",
  "title": "The two keepers",
  "pedagogy": "none",
  "npcs": [
    { "id": "old_wei", "name": "Old Wei", "persona": "a retired ferryman who distrusts strangers",
      "languages": [ { "code": "zh", "speaking": 9, "comprehension": 9 } ] }
  ],
  "facts": [
    { "key": "trust", "type": "number", "default": 0 },
    { "key": "eastern_gate", "type": "string" }
  ],
  "start": "suspicious_of_player",
  "checkpoints": [
    { "id": "suspicious_of_player",
      "goal": "Size the stranger up; warm to them only if they explain why they are here",
      "may_disclose": [],
      "edges": [
        { "id": "asks_about_brother", "to": "reveal_gate",
          "predicate": "trust >= 0.4", "when": "the player has asked about their missing brother" },
        { "id": "turns_hostile", "to": "failure", "when": "the player threatens or insults Old Wei" }
      ] },
    { "id": "reveal_gate", "goal": "Tell the player the eastern gate is guarded, reluctantly",
      "may_disclose": ["eastern_gate"],
      "edges": [ { "id": "thanks", "to": "success", "when": "the player thanks Old Wei or says goodbye" } ] }
  ],
  "x-layout": {
    "checkpoints": { "suspicious_of_player": { "x": 0, "y": 0 }, "reveal_gate": { "x": 240, "y": 0 } },
    "outcomes": { "success": { "x": 480, "y": 0 }, "failure": { "x": 240, "y": 160 } }
  }
}"#;

/// One edit to two-keepers.
type Mutation = Box<dyn FnOnce(&mut Value)>;

fn two_keepers() -> Value {
    serde_json::from_str(TWO_KEEPERS).unwrap()
}

/// `from_value` then `validate`, as `load` runs them.
fn run(value: &Value) -> Result<Vec<Warning>, IrError> {
    validate(&from_value(value)?)
}

/// The path a mutated two-keepers is refused at.
fn refused_at(mutate: impl FnOnce(&mut Value)) -> String {
    let mut value = two_keepers();
    mutate(&mut value);
    match run(&value) {
        Err(err) => err.path,
        Ok(_) => panic!("accepted: {value}"),
    }
}

fn warning_paths(mutate: impl FnOnce(&mut Value)) -> Vec<String> {
    let mut value = two_keepers();
    mutate(&mut value);
    run(&value).unwrap().into_iter().map(|w| w.path).collect()
}

#[test]
fn ir_validates_and_every_refusal_names_its_path() {
    // Both worked examples load, with no warning.
    for text in [MISSING_BROTHER, TWO_KEEPERS] {
        let (_, warnings) = load(text).unwrap();
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    // Each of D2's refusals names its path.
    let cases: Vec<(&str, Mutation)> = vec![
        ("schema", Box::new(|v| v["schema"] = json!("dialogue.v2"))),
        ("id", Box::new(|v| v["id"] = json!("Night-Market"))),
        ("title", Box::new(|v| v["title"] = json!("t".repeat(81)))),
        ("checkpoints[1].id", Box::new(|v| v["checkpoints"][1]["id"] = json!("success"))),
        ("checkpoints[1].edges[0].id", Box::new(|v| v["checkpoints"][1]["edges"][0]["id"] = json!("asks_about_brother"))),
        ("checkpoints[0].edges[0].to", Box::new(|v| v["checkpoints"][0]["edges"][0]["to"] = json!("nowhere"))),
        ("checkpoints[1].may_disclose[0]", Box::new(|v| v["checkpoints"][1]["may_disclose"] = json!(["secret"]))),
        ("facts[0].default", Box::new(|v| v["facts"][0]["default"] = json!("zero"))),
        ("checkpoints[0].edges[1].when", Box::new(|v| v["checkpoints"][0]["edges"][1]["when"] = json!("w".repeat(201)))),
        ("npcs", Box::new(|v| v["npcs"] = json!([]))),
        ("", Box::new(|v| v["x-padding"] = json!("x".repeat(MAX_IR_BYTES)))),
        ("colour", Box::new(|v| v["colour"] = json!("red"))),
        ("checkpoints[0].mood", Box::new(|v| v["checkpoints"][0]["mood"] = json!("tense"))),
        ("npcs[0].x-voice", Box::new(|v| v["npcs"][0]["x-voice"] = json!("deep"))),
    ];
    for (path, mutate) in cases {
        assert_eq!(refused_at(mutate), path);
    }

    // A refusal reads as its path and its reason.
    let err = refused_at(|v| v["checkpoints"][0]["edges"][0]["predicate"] = json!("trst >= 0.4"));
    assert_eq!(err, "checkpoints[0].edges[0].predicate");
    let mut value = two_keepers();
    value["checkpoints"][0]["edges"][0]["predicate"] = json!("trst >= 0.4");
    assert_eq!(run(&value).unwrap_err().to_string(), "checkpoints[0].edges[0].predicate: unknown fact \"trst\"");
}

#[test]
fn ir_accepts_what_the_schema_allows() {
    // A number default of 0.4, kept as written.
    let mut value = two_keepers();
    value["facts"][0]["default"] = json!(0.4);
    let ir = from_value(&value).unwrap();
    assert!(validate(&ir).unwrap().is_empty());
    assert_eq!(ir.facts[0].default, Some(FactValue::Number(serde_json::Number::from_f64(0.4).unwrap())));
    assert_eq!(serde_json::to_value(&ir).unwrap()["facts"][0]["default"], json!(0.4));

    // A fact keyed `not` may be declared and disclosed, never read.
    let mut value = two_keepers();
    value["facts"].as_array_mut().unwrap().push(json!({ "key": "not", "type": "boolean" }));
    value["checkpoints"][1]["may_disclose"] = json!(["eastern_gate", "not"]);
    assert!(run(&value).unwrap().is_empty());

    // `x-layout` is kept, written back, and changes nothing else.
    let ir = from_value(&two_keepers()).unwrap();
    assert!(ir.extensions.contains_key("x-layout"));
    assert_eq!(serde_json::to_value(&ir).unwrap()["x-layout"], two_keepers()["x-layout"]);

    // A pretty-printed document past the cap, whose normal form fits.
    let mut value = two_keepers();
    value["x-padding"] = json!("");
    let compact = serde_json::to_vec(&from_value(&value).unwrap()).unwrap().len();
    value["x-padding"] = json!("x".repeat(MAX_IR_BYTES - compact - 4));
    let pretty = serde_json::to_string_pretty(&value).unwrap();
    assert!(pretty.len() > MAX_IR_BYTES);
    assert!(load(&pretty).is_ok());
}

#[test]
fn text_a_turn_refuses_and_unreachable_checkpoints_are_warnings() {
    let orphan = json!({ "id": "orphan", "goal": "Nobody reaches this", "edges": [{ "id": "back", "to": "reveal_gate", "when": "ever" }] });
    assert_eq!(warning_paths(|v| v["checkpoints"].as_array_mut().unwrap().push(orphan)), ["checkpoints[2]"]);
    assert_eq!(warning_paths(|v| v["facts"][1]["default"] = json!("   ")), ["facts[1].default"]);
    assert_eq!(warning_paths(|v| v["facts"][1]["default"] = json!("north\ngate")), ["facts[1].default"]);
    assert_eq!(warning_paths(|v| v["checkpoints"][1]["goal"] = json!("Tell the player\nabout the gate")), ["checkpoints[1].goal"]);
    assert_eq!(warning_paths(|v| v["checkpoints"][0]["edges"][1]["when"] = json!("  ")), ["checkpoints[0].edges[1].when"]);
}

/// Two checkpoints joined both ways; `back` carries `when` only if `judged`.
fn two_way(judged: bool) -> Value {
    let mut back = json!({ "id": "back", "to": "a", "predicate": "not ready" });
    if judged {
        back["when"] = json!("the player asks to go back");
    }
    json!({
        "schema": "dialogue.v1", "id": "loop", "npcs": [{ "id": "npc", "name": "Npc" }],
        "facts": [{ "key": "ready", "type": "boolean" }],
        "start": "a",
        "checkpoints": [
            { "id": "a", "goal": "First", "edges": [{ "id": "forth", "to": "b", "predicate": "ready" }] },
            { "id": "b", "goal": "Second", "edges": [back, { "id": "done", "to": "success", "when": "the player leaves" }] }
        ]
    })
}

#[test]
fn a_when_less_cycle_is_refused_and_a_judged_one_is_not() {
    let err = run(&two_way(false)).unwrap_err();
    assert!(["checkpoints[0].edges[0]", "checkpoints[1].edges[0]"].contains(&err.path.as_str()), "{err}");
    assert!(err.message.contains("cycle"), "{err}");
    assert!(run(&two_way(true)).unwrap().is_empty());
}
