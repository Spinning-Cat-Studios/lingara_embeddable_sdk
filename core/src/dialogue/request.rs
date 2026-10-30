//! What a turn sends (D8; ADR 28.10.26f D5, D7): the config's local checks,
//! the line's, and B3's body with N1's profile and a graph's fields.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::dialogue::{Dialogue, graph};
use crate::error::EmbedError;
use crate::limits;
use crate::types::{DialogueConfig, NpcLanguage};
use crate::world;

/// N1's profile: at most 8 languages, unique codes, levels 1–9.
fn languages(languages: &[NpcLanguage]) -> Result<(), EmbedError> {
    if languages.len() > limits::NPC_LANGUAGES_MAX_ITEMS {
        return Err(EmbedError::invalid(format!("npc.languages holds at most {} (got {})", limits::NPC_LANGUAGES_MAX_ITEMS, languages.len())));
    }
    let mut codes = HashSet::new();
    for language in languages {
        world::text("npc.languages[].code", &language.code, limits::NPC_LANGUAGE_CODE_MAX_CHARS)?;
        world::level("npc.languages[].speaking", language.speaking)?;
        world::level("npc.languages[].comprehension", language.comprehension)?;
        if !codes.insert(language.code.as_str()) {
            return Err(EmbedError::invalid(format!("npc.languages names `{}` twice", language.code)));
        }
    }
    Ok(())
}

pub(super) fn check(config: &DialogueConfig) -> Result<(), EmbedError> {
    world::npc(&config.npc)?;
    languages(&config.languages)?;
    if let Some(scene) = &config.scene {
        world::text("scene", scene, limits::SCENE_MAX_CHARS)?;
    }
    world::lang("source_lang", &config.source_lang)?;
    world::lang("target_lang", &config.target_lang)?;
    world::level("level", config.level)
}

/// 1–500 characters after trimming, on one line.
pub(super) fn check_line(line: &str) -> Result<String, EmbedError> {
    let line = line.trim();
    if line.contains(['\n', '\r']) {
        return Err(EmbedError::invalid("a dialogue line is one line"));
    }
    world::text("line", line, limits::DIALOGUE_LINE_MAX_CHARS)?;
    Ok(line.to_owned())
}

/// B3's request: the config, the line, and the window as `history`; N1's
/// profile when set; a graph's fields when there is one. Answers the body
/// and the edge ids it offered.
pub(super) fn body(d: &Dialogue, line: &str) -> (Value, Vec<String>) {
    let mut npc = world::npc_json(&d.config.npc);
    if !d.config.languages.is_empty() {
        npc["languages"] = json!(d.config.languages);
    }
    let mut body = json!({
        "npc": npc,
        "source_lang": d.config.source_lang,
        "target_lang": d.config.target_lang,
        "level": d.config.level,
        "line": line,
        "history": d.window.entries(),
    });
    if let Some(scene) = &d.config.scene {
        body["scene"] = json!(scene);
    }
    if let Some(pedagogy) = d.config.pedagogy {
        body["pedagogy"] = json!(pedagogy);
    }
    let sent = d.graph.as_ref().map(|g| graph::turn_fields(g, &mut body)).unwrap_or_default();
    (body, sent)
}
