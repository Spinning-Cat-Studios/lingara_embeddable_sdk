//! A procedural NPC (ADR 3.11.26j D10): the same component as a graph NPC,
//! opened with `DialogueLoad` over a procedural document's canonical text.

use lingara_embed::dialogue_ir::Document;
use lingara_embed::dialogue_ir::procedural::ProceduralDialogue;
use lingara_embed::{EmbedError, ErrorKind, Npc};

use super::super::bounds::check_level;
use super::{Graph, NpcDialogue};

impl NpcDialogue {
    /// The cast member `npc_id` of a [`ProceduralDialogue`], built or parsed,
    /// opened with `DialogueLoad` as [`NpcDialogue::graph`] opens a static
    /// one. The core plans its first beats before the first
    /// `CheckpointReached`, and each answer arrives as a `DialoguePlanned`
    /// Message.
    pub fn procedural(doc: &ProceduralDialogue, npc_id: impl Into<String>, target_lang: impl Into<String>, level: u8) -> Result<Self, EmbedError> {
        let npc_id = npc_id.into();
        let Some(profile) = doc.npc(&npc_id) else {
            return Err(EmbedError::new(ErrorKind::InvalidInput, format!("the dialogue's cast has no NPC \"{npc_id}\"")));
        };
        let npc = Npc { name: profile.name.clone(), persona: profile.persona.clone() };
        let ir_json = Document::Procedural(doc.clone()).to_canonical_json();
        let graph = Some(Graph::Inline { ir_json, npc_id });
        Ok(Self { npc, scene: None, target_lang: target_lang.into(), source_lang: None, level: check_level(level)?, window: Vec::new(), graph })
    }
}
