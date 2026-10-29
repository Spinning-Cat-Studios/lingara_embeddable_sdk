# lingara-dialogue-ir

The Lingara dialogue graph format, `dialogue.v1`: its Rust types, its
load-time checks, and the fact-predicate language an edge's `predicate` is
written in.

A dialogue is a cast of NPCs, the facts a game tracks, and checkpoints joined
by edges. An edge's `predicate` is a game condition the embed core evaluates
(`trust >= 0.4`); its `when` is a conversational condition the server's judge
evaluates. An edge with no `when` is crossed by the core alone.

This crate is the one implementation of those rules. The embed core
(`lingara-embed`, which re-exports it as `lingara_embed::dialogue_ir`) runs a
dialogue with it, and the Lingara server validates a saved one with the same
parser, so the two can never disagree about what a predicate means. It depends
on `serde` and `serde_json` only (`schemars` behind the optional `schema`
feature): no async runtime, no HTTP client.

## Loading and validating

```rust
let text = r#"{
  "schema": "dialogue.v1", "id": "missing_brother",
  "npcs": [{ "id": "elena", "name": "Elena" }],
  "facts": [{ "key": "trust", "type": "number", "default": 0 }],
  "start": "introduction",
  "checkpoints": [
    { "id": "introduction", "goal": "Elena is wary and asks who the player is",
      "edges": [{ "id": "gate", "to": "success", "predicate": "trust >= 0.4",
                  "when": "the player asks where soldiers might take a prisoner" }] }
  ]
}"#;
let (ir, warnings) = lingara_dialogue_ir::load(text)?;
assert_eq!(ir.start, "introduction");
assert!(warnings.is_empty());
# Ok::<(), lingara_dialogue_ir::IrError>(())
```

`from_json` reads a document and refuses an unknown member or a member of the
wrong type; `validate` checks the schema's bounds and the seven rules the
schema cannot say. Every refusal is an `IrError` naming a dotted JSON path,
such as `checkpoints[1].edges[0].predicate: unknown fact "trst"`. An
unreachable checkpoint, and text the schema allows but a turn refuses (a blank
`goal`, a two-line `when`), are warnings, not refusals. Top-level `x-` members
(a canvas's `x-layout`) are kept and ignored.

The frozen `dialogue.v1` bounds are in `limits`, under the schema's names
(`MAX_TITLE_CHARS`, `MAX_PREDICATE_CHARS`, `MAX_IR_BYTES` …).

## Predicates

```rust
use lingara_dialogue_ir::{FactDecl, FactType, FactValue, predicate};
use std::collections::BTreeMap;

let facts = [FactDecl { key: "trust".into(), kind: FactType::Number, default: None }];
let parsed = predicate::check("trust >= 0.4", &facts)?;
let mut set = BTreeMap::new();
set.insert("trust".to_string(), FactValue::Number(serde_json::Number::from_f64(0.5).unwrap()));
assert!(predicate::eval(&parsed, &|key| predicate::value(&set, &facts, key)));
# Ok::<(), predicate::PredicateError>(())
```

The grammar is `predicate.ebnf`, shipped in this crate, and the crate's tests
hold the parser to every example it lists. Within `dialogue.v1` the grammar
only ever accepts more, so an older server never refuses what a newer kit
loads. A fact read by a predicate must be declared; a fact with no value set
and no `default` is *unset*, and every comparison over it is `false`. Declare
a `default` and you never meet that rule.

## Pinning

Pin an exact version (`lingara-dialogue-ir = "=X.Y.Z"`), as the embed kits do:
the version moves with every embed SDK release, and the grammar and checks a
server runs should be the ones the game's kit runs.
