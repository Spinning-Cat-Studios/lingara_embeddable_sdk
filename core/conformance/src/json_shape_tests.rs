//! `conformance/vectors/json-shape.json` against the crate (D10). Here, not
//! in `core/src/tests/`, because the packaged crate carries no
//! `conformance/`.

use std::collections::BTreeSet;

use lingara_embed::*;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::matching::without_nulls;

fn vector() -> Value {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../conformance/vectors/json-shape.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("json-shape.json")).expect("JSON")
}

/// The variant tags the type's own schema declares, so a variant added to
/// the crate without an example fails here.
fn declared<T: schemars::JsonSchema>() -> BTreeSet<String> {
    fn walk(v: &Value, out: &mut BTreeSet<String>) {
        if let Some(tag) = v.pointer("/properties/type/const").and_then(Value::as_str) {
            out.insert(tag.to_owned());
        }
        for tag in v.pointer("/properties/type/enum").and_then(Value::as_array).into_iter().flatten() {
            out.extend(tag.as_str().map(str::to_owned));
        }
        let children = v.get("oneOf").or_else(|| v.get("anyOf")).and_then(Value::as_array);
        children.into_iter().flatten().for_each(|c| walk(c, out));
    }
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    let mut tags = BTreeSet::new();
    walk(&schema, &mut tags);
    tags
}

/// The secret-bearing values, as they serialise.
fn redacted(mut example: Value) -> Value {
    let secret = "[REDACTED]";
    if example["type"] == "provide_token" || example["type"] == "provide_token_for" {
        example["token"] = secret.into();
    }
    for header in example.get_mut("headers").and_then(Value::as_array_mut).into_iter().flatten() {
        header["value"] = secret.into();
    }
    example
}

fn tagged<T: DeserializeOwned + Serialize + schemars::JsonSchema>(name: &str, vector: &Value) {
    let examples = vector["tagged"][name].as_array().unwrap_or_else(|| panic!("no {name} examples"));
    let tags: BTreeSet<String> = examples.iter().map(|e| e["type"].as_str().expect("a type tag").to_owned()).collect();
    assert_eq!(tags, declared::<T>(), "{name}: one example per variant");
    for example in examples {
        let value: T = serde_json::from_value(example.clone()).unwrap_or_else(|e| panic!("{name} {example}: {e}"));
        let back = serde_json::to_value(&value).unwrap();
        assert_eq!(without_nulls(&back), without_nulls(&redacted(example.clone())), "{name}");
    }
}

fn words<T: DeserializeOwned + Serialize + schemars::JsonSchema>(name: &str, vector: &Value) {
    let words = vector["words"][name].as_array().unwrap_or_else(|| panic!("no {name} words"));
    let schema = serde_json::to_value(schemars::schema_for!(T)).unwrap();
    let declared: BTreeSet<&str> = schema["enum"].as_array().or_else(|| schema["oneOf"].as_array()).expect("an enum").iter().flat_map(|v| v.as_str().or_else(|| v["const"].as_str())).collect();
    let listed: BTreeSet<&str> = words.iter().filter_map(Value::as_str).collect();
    assert_eq!(listed, declared, "{name}: every word");
    for word in words {
        let value: T = serde_json::from_value(word.clone()).unwrap_or_else(|e| panic!("{name} {word}: {e}"));
        assert_eq!(&serde_json::to_value(value).unwrap(), word);
    }
}

// 1.10.26y AC17
#[test]
fn every_variant_matches_the_shape_vector() {
    let v = vector();
    tagged::<Command>("Command", &v);
    tagged::<CommandOutcome>("CommandOutcome", &v);
    tagged::<EmbedEvent>("EmbedEvent", &v);
    tagged::<StreamState>("StreamState", &v);
    tagged::<NpcTurnUpdate>("NpcTurnUpdate", &v);
    tagged::<PracticeSource>("PracticeSource", &v);
    tagged::<TokenSourceConfig>("TokenSourceConfig", &v);
    tagged::<LingaraEventDetail>("LingaraEventDetail", &v);
    words::<TokenReason>("TokenReason", &v);
    words::<ErrorKind>("ErrorKind", &v);
    words::<LogLevel>("LogLevel", &v);
    words::<CloseReason>("CloseReason", &v);
    words::<PracticePhase>("PracticePhase", &v);
    words::<PracticeOutcome>("PracticeOutcome", &v);
    words::<DialoguePhase>("DialoguePhase", &v);
    words::<Speaker>("Speaker", &v);
    every_command_is_its_flat_method(&v);
}

fn round_trip<T: DeserializeOwned + Serialize>(example: &Value) -> T {
    let value: T = serde_json::from_value(example.clone()).unwrap_or_else(|e| panic!("{example}: {e}"));
    assert_eq!(without_nulls(&serde_json::to_value(&value).unwrap()), without_nulls(example), "{example}");
    value
}

// 28.10.26f AC14
#[test]
fn graph_variants_match_the_shape_vector() {
    let v = vector();
    // The graph's own types; its commands and events ride the three
    // enums above, whose tag sets this pins.
    tagged::<DialogueSource>("DialogueSource", &v);
    words::<CrossedVia>("CrossedVia", &v);
    words::<DialogueOutcome>("DialogueOutcome", &v);
    words::<Pedagogy>("Pedagogy", &v);
    let tags = |name: &str| v["tagged"][name].as_array().unwrap().iter().map(|e| e["type"].as_str().unwrap().to_owned()).collect::<BTreeSet<_>>();
    for tag in ["dialogue_load", "dialogue_set_fact", "dialogue_validate"] {
        assert!(tags("Command").contains(tag), "{tag}");
    }
    for tag in ["checkpoint_reached", "dialogue_ended"] {
        assert!(tags("EmbedEvent").contains(tag), "{tag}");
    }
    // A tag has one example, so the graph's struct-level shapes live apart.
    let config: DialogueConfig = round_trip(&v["structs"]["DialogueConfig"][0]);
    assert_eq!((config.languages.len(), config.pedagogy), (2, Some(Pedagogy::None)));
    let snapshot: DialogueSnapshot = round_trip(&v["structs"]["DialogueSnapshot"][0]);
    assert_eq!(snapshot.graph.and_then(|g| g.outcome), Some(DialogueOutcome::Success));
    every_command_is_its_flat_method(&v);

    // The C ABI's committed message schema is schemars' over these types
    // (`make check-ffi-schema` holds it byte-equal); it names every new tag.
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ffi/schema/messages.schema.json");
    let committed = std::fs::read_to_string(path).expect("messages.schema.json");
    for tag in ["dialogue_load", "dialogue_set_fact", "dialogue_validate", "checkpoint_reached", "dialogue_ended", "ended"] {
        assert!(committed.contains(&format!("\"{tag}\"")), "messages.schema.json lacks {tag}: run make ffi-schema");
    }
}

// 28.10.26i AC7
#[test]
fn stored_variants_match_the_shape_vector() {
    let v = vector();
    let tags = |name: &str| v["tagged"][name].as_array().unwrap().iter().map(|e| e["type"].as_str().unwrap().to_owned()).collect::<BTreeSet<_>>();
    assert!(tags("DialogueSource").contains("stored"));
    assert!(tags("EmbedEvent").contains("dialogue_loaded") && tags("EmbedEvent").contains("dialogue_load_failed"));
    assert!(v["words"]["DialoguePhase"].as_array().unwrap().contains(&Value::from("loading")));
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ffi/schema/messages.schema.json");
    let committed = std::fs::read_to_string(path).expect("messages.schema.json");
    for tag in ["stored", "dialogue_loaded", "dialogue_load_failed", "loading"] {
        assert!(committed.contains(&format!("\"{tag}\"")), "messages.schema.json lacks {tag}: run make ffi-schema");
    }
}

/// What a call answered: the outcome's tag, or the error's kind.
fn shape(result: Result<CommandOutcome, EmbedError>) -> String {
    match result {
        Ok(outcome) => serde_json::to_value(outcome).unwrap()["type"].as_str().unwrap_or_default().to_owned(),
        Err(e) => format!("{:?}", e.kind),
    }
}

/// Each `Command` through `execute` answers as its flat method does.
fn every_command_is_its_flat_method(v: &Value) {
    let start = || Embed::start(EmbedConfig { base_url: "http://127.0.0.1:9".into(), stream_on_start: false, ..EmbedConfig::default() }).unwrap();
    let (by_command, by_method) = (start(), start());
    for example in v["tagged"]["Command"].as_array().unwrap() {
        let command: Command = serde_json::from_value(example.clone()).unwrap();
        let got = shape(by_command.execute(command.clone()));
        let want = shape(flat(&by_method, command));
        assert_eq!(got, want, "{example}");
    }
}

fn done(r: Result<(), EmbedError>) -> Result<CommandOutcome, EmbedError> {
    r.map(|()| CommandOutcome::Done {})
}

fn practice(r: Result<PracticeSnapshot, EmbedError>) -> Result<CommandOutcome, EmbedError> {
    r.map(|s| CommandOutcome::Practice { snapshot: Some(s) })
}

fn flat(e: &Embed, command: Command) -> Result<CommandOutcome, EmbedError> {
    let id = |r: Result<u64, EmbedError>| r.map(|id| CommandOutcome::Id { id });
    match command {
        Command::ProvideToken { token, expires_at } => done(e.provide_token(token, expires_at)),
        Command::ProvideTokenFor { token, expires_in_secs } => done(e.provide_token_for(token, expires_in_secs)),
        Command::TokenUnavailable { reason } => done(e.token_unavailable(&reason)),
        Command::SetTokenEndpoint { url, headers } => done(e.set_token_endpoint(&url, headers)),
        Command::StreamPause => done(e.stream_pause()),
        Command::StreamResume => done(e.stream_resume()),
        Command::SendContext { context } => id(e.send_context(context)),
        Command::PracticeStart { source } => id(e.practice_start(source)),
        Command::PracticeSnapshot { session } => Ok(CommandOutcome::Practice { snapshot: e.practice_snapshot(session) }),
        Command::PracticeAnswer { session, answer } => practice(e.practice_answer(session, &answer)),
        Command::PracticeChoose { session, option } => practice(e.practice_choose(session, option)),
        Command::PracticeReveal { session } => practice(e.practice_reveal(session)),
        Command::PracticeSelfGrade { session, correct } => practice(e.practice_self_grade(session, correct)),
        Command::PracticeNext { session } => practice(e.practice_next(session)),
        Command::PracticeFinish { session } => practice(e.practice_finish(session)),
        other => flat_dialogue(e, other),
    }
}

fn flat_dialogue(e: &Embed, command: Command) -> Result<CommandOutcome, EmbedError> {
    match command {
        Command::DialogueOpen { config } => e.dialogue_open(config).map(|id| CommandOutcome::Id { id }),
        Command::DialogueSay { dialogue, line } => e.dialogue_say(dialogue, &line).map(|id| CommandOutcome::Id { id }),
        Command::DialogueSetLevel { dialogue, level } => done(e.dialogue_set_level(dialogue, level)),
        Command::DialogueSetScene { dialogue, scene } => done(e.dialogue_set_scene(dialogue, scene)),
        Command::DialogueCancel { dialogue } => done(e.dialogue_cancel(dialogue)),
        Command::DialogueClose { dialogue } => done(e.dialogue_close(dialogue)),
        Command::DialogueSnapshot { dialogue } => Ok(CommandOutcome::Dialogue { snapshot: e.dialogue_snapshot(dialogue) }),
        Command::DialogueLoad { source, session } => e.dialogue_load(source, session).map(|id| CommandOutcome::Id { id }),
        Command::DialogueSetFact { dialogue, key, value } => done(e.dialogue_set_fact(dialogue, &key, value)),
        Command::DialogueValidate { ir_json } => done(e.dialogue_validate(&ir_json)),
        other => panic!("no flat method for {other:?}: add it to this table"),
    }
}
