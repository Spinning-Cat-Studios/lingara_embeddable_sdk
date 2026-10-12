//! Shared test fixtures: a small frozen version in B1–B3's shapes, and cases
//! written as YAML the way a case file is.

use serde_json::json;

use crate::case::Case;
use crate::spec::Frozen;

/// The operations EK and ES name, with the request schemas the judge reads.
pub fn frozen() -> Frozen {
    let openapi = json!({
        "paths": {
            "/v1/events/stream": { "get": { "operationId": "streamEvents" } },
            "/v1/events": { "post": { "operationId": "sendEvent", "requestBody": { "content": { "application/json": {
                "schema": { "type": "object", "required": ["type"], "properties": { "type": { "type": "string" } } } } } } } },
            "/v1/embed/tokens": { "post": { "operationId": "createEmbedToken", "requestBody": { "content": { "application/json": {
                "schema": { "$ref": "#/components/schemas/EmbedTokenRequest" } } } } } },
            "/v1/embed/players/{player_ref}": { "delete": { "operationId": "deleteEmbedPlayer" } },
            "/v1/embed/dialogue/turns": { "post": { "operationId": "sendDialogueTurn" } }
        },
        "components": { "schemas": { "EmbedTokenRequest": {
            "type": "object", "required": ["player_ref"], "additionalProperties": false,
            "properties": { "player_ref": { "type": "string", "minLength": 1, "maxLength": 128 },
                            "scopes": { "type": "array", "items": { "type": "string" } },
                            "origin": { "type": "string" } } } } }
    });
    let asyncapi = json!({ "components": { "messages": { "PracticeCompleted": { "name": "practice.completed" } } } });
    Frozen { id: "2026-10-fixture".into(), openapi, asyncapi }
}

/// A case from YAML, panicking on a fixture that does not parse.
pub fn case(yaml: &str) -> Case {
    serde_yaml::from_str(yaml).unwrap_or_else(|e| panic!("fixture case does not parse: {e}\n{yaml}"))
}

/// Writes `files` (relative path → text) under a fresh temp directory.
pub fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    for (rel, text) in files {
        let path = dir.path().join(rel);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }
    dir
}
