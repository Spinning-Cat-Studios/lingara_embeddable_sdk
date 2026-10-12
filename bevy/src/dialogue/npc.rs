//! `NpcDialogue` (D7): K1's `DialogueConfig` fields on an entity, bounded at
//! construction with K1's own `limits`, so an over-long persona fails in the
//! game's code rather than at the server.

use bevy::ecs::component::Component;
use lingara_embed::{DialogueConfig, EmbedError, ErrorKind, Npc, WindowEntry, limits};

/// An NPC the player can talk to. Only `level` and `scene` change on a live
/// NPC; a different persona or language is a different NPC, so insert a new
/// `NpcDialogue` (which closes the old dialogue and starts a new transcript).
#[derive(Component, Debug, Clone, PartialEq)]
pub struct NpcDialogue {
    npc: Npc,
    scene: Option<String>,
    target_lang: String,
    source_lang: Option<String>,
    level: u8,
    window: Vec<WindowEntry>,
}

/// 1 to `max` Unicode scalar values, not blank: K1's own rule.
fn text(field: &str, max: usize, value: &str) -> Result<(), EmbedError> {
    let n = value.chars().count();
    if value.trim().is_empty() || n > max {
        return Err(EmbedError::new(ErrorKind::InvalidInput, format!("{field} must be 1–{max} characters, not blank (got {n})")));
    }
    Ok(())
}

fn check_level(level: u8) -> Result<u8, EmbedError> {
    if !(limits::LEVEL_MIN..=limits::LEVEL_MAX).contains(&level) {
        let message = format!("level must be {}–{} (got {level})", limits::LEVEL_MIN, limits::LEVEL_MAX);
        return Err(EmbedError::new(ErrorKind::InvalidInput, message));
    }
    Ok(level)
}

impl NpcDialogue {
    /// An NPC called `name` (1–32 characters) speaking `target_lang` at
    /// `level` (1–9).
    pub fn new(name: impl Into<String>, target_lang: impl Into<String>, level: u8) -> Result<Self, EmbedError> {
        let name = name.into();
        text("npc.name", limits::NPC_NAME_MAX_CHARS, &name)?;
        let npc = Npc { name, persona: None };
        Ok(Self { npc, scene: None, target_lang: target_lang.into(), source_lang: None, level: check_level(level)?, window: Vec::new() })
    }

    /// Who the NPC is, in 1–120 characters.
    pub fn with_persona(mut self, persona: impl Into<String>) -> Result<Self, EmbedError> {
        let persona = persona.into();
        text("npc.persona", limits::NPC_PERSONA_MAX_CHARS, &persona)?;
        self.npc.persona = Some(persona);
        Ok(self)
    }

    /// Where the conversation happens, in 1–160 characters.
    pub fn with_scene(mut self, scene: impl Into<String>) -> Result<Self, EmbedError> {
        self.set_scene(Some(scene.into()))?;
        Ok(self)
    }

    /// Defaults to `LingaraConfig::source_lang`.
    pub fn with_source_lang(mut self, lang: impl Into<String>) -> Self {
        self.source_lang = Some(lang.into());
        self
    }

    /// A conversation the game saved from `dialogue_snapshot().window`; K1
    /// cuts and trims it on open.
    pub fn with_window(mut self, window: Vec<WindowEntry>) -> Self {
        self.window = window;
        self
    }

    /// The next turn uses it, the window kept.
    pub fn set_level(&mut self, level: u8) -> Result<(), EmbedError> {
        self.level = check_level(level)?;
        Ok(())
    }

    pub fn set_scene(&mut self, scene: Option<String>) -> Result<(), EmbedError> {
        if let Some(scene) = &scene {
            text("scene", limits::SCENE_MAX_CHARS, scene)?;
        }
        self.scene = scene;
        Ok(())
    }

    pub fn level(&self) -> u8 {
        self.level
    }

    pub fn scene(&self) -> Option<&str> {
        self.scene.as_deref()
    }

    pub fn npc(&self) -> &Npc {
        &self.npc
    }

    pub fn target_lang(&self) -> &str {
        &self.target_lang
    }

    /// K1's open request; an unset source language falls back to the config's.
    pub(crate) fn config(&self, default_source: Option<&str>) -> DialogueConfig {
        let source_lang = self.source_lang.as_deref().or(default_source).unwrap_or_default().to_owned();
        let (npc, scene, window) = (self.npc.clone(), self.scene.clone(), self.window.clone());
        DialogueConfig { npc, scene, source_lang, target_lang: self.target_lang.clone(), level: self.level, window }
    }
}
