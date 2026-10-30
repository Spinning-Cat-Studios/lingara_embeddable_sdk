//! `NpcDialogue` (D7): K1's `DialogueConfig` fields on an entity, bounded at
//! construction with K1's own `limits`, so an over-long persona fails in the
//! game's code rather than at the server.
//!
//! A graph NPC (ADR 28.10.26g D6) is the same component opened with K1's
//! `DialogueLoad` instead of `DialogueOpen`, so the lifecycle is shared.

use std::collections::BTreeMap;

use bevy::ecs::component::Component;
use lingara_embed::dialogue_ir::DialogueIr;
use lingara_embed::{Command, DialogueConfig, DialogueSource, EmbedError, ErrorKind, GraphSession, Npc, WindowEntry, limits};

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
    graph: Option<Graph>,
}

/// A graph NPC's dialogue, as the canonical text `DialogueLoad` sends, and
/// the cast member this entity is.
#[derive(Debug, Clone, PartialEq)]
struct Graph {
    ir_json: String,
    npc_id: String,
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
        Ok(Self { npc, scene: None, target_lang: target_lang.into(), source_lang: None, level: check_level(level)?, window: Vec::new(), graph: None })
    }

    /// The cast member `npc_id` of `ir` (built or parsed), speaking
    /// `target_lang` to a learner at `level`, as in [`NpcDialogue::new`].
    /// The plugin opens it with `DialogueLoad`, sending `ir`'s canonical
    /// text; K1 validates it there, with its JSON paths.
    pub fn graph(ir: &DialogueIr, npc_id: impl Into<String>, target_lang: impl Into<String>, level: u8) -> Result<Self, EmbedError> {
        let npc_id = npc_id.into();
        let Some(profile) = ir.npcs.iter().find(|n| n.id == npc_id) else {
            return Err(EmbedError::new(ErrorKind::InvalidInput, format!("the dialogue's cast has no NPC \"{npc_id}\"")));
        };
        let npc = Npc { name: profile.name.clone(), persona: profile.persona.clone() };
        let graph = Some(Graph { ir_json: ir.to_canonical_json(), npc_id });
        Ok(Self { npc, scene: None, target_lang: target_lang.into(), source_lang: None, level: check_level(level)?, window: Vec::new(), graph })
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

    /// K1's open request, `DialogueLoad` for a graph NPC; an unset source
    /// language falls back to the config's.
    pub(crate) fn open_command(&self, default_source: Option<&str>) -> Command {
        let source_lang = self.source_lang.as_deref().or(default_source).unwrap_or_default().to_owned();
        let (scene, window, target_lang, level) = (self.scene.clone(), self.window.clone(), self.target_lang.clone(), self.level);
        if let Some(graph) = &self.graph {
            let source = DialogueSource::Ir { ir_json: graph.ir_json.clone() };
            let npc = Some(graph.npc_id.clone());
            let session = GraphSession { npc, source_lang, target_lang, level, scene, facts: BTreeMap::new(), checkpoint: None, window };
            return Command::DialogueLoad { source, session };
        }
        // N1's profile on a free-form NPC is not this component's: a graph
        // NPC carries it in its IR.
        let config = DialogueConfig { npc: self.npc.clone(), scene, source_lang, target_lang, level, window, languages: Vec::new(), pedagogy: None };
        Command::DialogueOpen { config }
    }
}
