//! The frozen snapshot the server judges against (ADR 1.10.26x D3, D5).
//!
//! It opens `versions/<current>.{openapi,asyncapi}.json` beside the registry,
//! never the live `openapi.json`: a kit is held to the API version it pins.
//! `current` is the newest `supported` or `lts` entry, the clients'
//! `current_id` rule.

use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize)]
struct Registry {
    #[serde(default)]
    version: Vec<Entry>,
}

#[derive(Debug, Deserialize)]
struct Entry {
    id: String,
    state: String,
}

/// One frozen version, both halves.
#[derive(Debug, Clone)]
pub struct Frozen {
    pub id: String,
    pub openapi: Value,
    pub asyncapi: Value,
}

/// Where an operation lives, and the JSON schema its request body must meet.
#[derive(Debug, Clone)]
pub struct OperationRef {
    pub method: String,
    pub path: String,
    pub body_schema: Option<Value>,
}

const METHODS: [&str; 5] = ["get", "put", "post", "delete", "patch"];

impl Frozen {
    /// The registry's current version, read from the files beside it.
    pub fn load(registry: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(registry).map_err(|e| format!("{}: {e}", registry.display()))?;
        let parsed: Registry = toml::from_str(&text).map_err(|e| format!("{}: {e}", registry.display()))?;
        let current = parsed
            .version
            .iter()
            .rev()
            .find(|v| v.state == "supported" || v.state == "lts")
            .ok_or_else(|| format!("{}: no supported or lts version", registry.display()))?;
        let dir = registry.parent().unwrap_or(Path::new(".")).join("versions");
        let half = |kind: &str| -> Result<Value, String> {
            let path = dir.join(format!("{}.{kind}.json", current.id));
            let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
            serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
        };
        Ok(Self { id: current.id.clone(), openapi: half("openapi")?, asyncapi: half("asyncapi")? })
    }

    /// The operation with this `operationId`, if the version has it.
    pub fn operation(&self, operation_id: &str) -> Option<OperationRef> {
        let paths = self.openapi.get("paths")?.as_object()?;
        for (path, item) in paths {
            for method in METHODS {
                let Some(op) = item.get(method) else { continue };
                if op.get("operationId").and_then(Value::as_str) == Some(operation_id) {
                    let body_schema = op.pointer("/requestBody/content/application~1json/schema").cloned();
                    return Some(OperationRef { method: method.to_uppercase(), path: path.clone(), body_schema });
                }
            }
        }
        None
    }

    /// True when the AsyncAPI half names a message `name`.
    pub fn has_message(&self, name: &str) -> bool {
        let messages = self.asyncapi.pointer("/components/messages").and_then(Value::as_object);
        messages.is_some_and(|m| m.values().any(|msg| msg.get("name").and_then(Value::as_str) == Some(name)))
    }

    /// A schema root that resolves `#/components/...` against the OpenAPI document.
    pub fn schema_root(&self, schema: &Value) -> Value {
        serde_json::json!({ "components": self.openapi.get("components").cloned().unwrap_or_default(), "allOf": [schema] })
    }
}

/// True when `path` fills the template `template` (`{param}` is one segment).
pub fn path_matches(template: &str, path: &str) -> bool {
    let want: Vec<&str> = template.trim_matches('/').split('/').collect();
    let got: Vec<&str> = path.trim_matches('/').split('/').collect();
    want.len() == got.len()
        && want.iter().zip(&got).all(|(w, g)| (w.starts_with('{') && w.ends_with('}') && !g.is_empty()) || w == g)
}
