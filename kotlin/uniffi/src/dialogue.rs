//! The NPC dialogue commands on `EmbedHandle` (ADR 1.10.26ae D2). A reply
//! streams back as `NpcTurn` events, fragment by fragment.

use lingara_embed as core;

use crate::convert::{fail, id_out, known, narrow, to_core};
use crate::handle::{EmbedHandle, Outcome};
use crate::types::{DialogueConfig, DialogueSnapshot, DialogueSource, FactValue, GraphSession};

#[uniffi::export]
impl EmbedHandle {
    /// Opens a conversation with an NPC; its id.
    pub fn dialogue_open(&self, config: DialogueConfig) -> Outcome<i64> {
        let config: core::DialogueConfig = to_core("config", &config)?;
        id_out(self.embed.dialogue_open(config).map_err(fail)?)
    }

    /// Sends the player's line; the request id its `NpcTurn` events carry.
    pub fn dialogue_say(&self, dialogue: i64, line: String) -> Outcome<i64> {
        id_out(self.embed.dialogue_say(narrow("dialogue", dialogue)?, &line).map_err(fail)?)
    }

    pub fn dialogue_set_level(&self, dialogue: i64, level: i32) -> Outcome<()> {
        let level = narrow("level", level)?;
        self.embed.dialogue_set_level(narrow("dialogue", dialogue)?, level).map_err(fail)
    }

    pub fn dialogue_set_scene(&self, dialogue: i64, scene: Option<String>) -> Outcome<()> {
        self.embed.dialogue_set_scene(narrow("dialogue", dialogue)?, scene).map_err(fail)
    }

    pub fn dialogue_cancel(&self, dialogue: i64) -> Outcome<()> {
        self.embed.dialogue_cancel(narrow("dialogue", dialogue)?).map_err(fail)
    }

    pub fn dialogue_close(&self, dialogue: i64) -> Outcome<()> {
        self.embed.dialogue_close(narrow("dialogue", dialogue)?).map_err(fail)
    }

    pub fn dialogue_snapshot(&self, dialogue: i64) -> Outcome<Option<DialogueSnapshot>> {
        self.embed.dialogue_snapshot(narrow("dialogue", dialogue)?).as_ref().map(known).transpose()
    }

    /// Loads a `dialogue.v1` graph and opens a run of it; its dialogue id
    /// (ADR 28.10.26f). `CheckpointReached` follows.
    pub fn dialogue_load(&self, source: DialogueSource, session: GraphSession) -> Outcome<i64> {
        let source: core::DialogueSource = to_core("source", &source)?;
        let session: core::GraphSession = to_core("session", &session)?;
        id_out(self.embed.dialogue_load(source, session).map_err(fail)?)
    }

    /// Sets a fact the graph declared.
    pub fn dialogue_set_fact(&self, dialogue: i64, key: String, value: FactValue) -> Outcome<()> {
        let value: core::FactValue = to_core("value", &value)?;
        self.embed.dialogue_set_fact(narrow("dialogue", dialogue)?, &key, value).map_err(fail)
    }

    /// Checks a `dialogue.v1` document as `dialogue_load` would, creating nothing.
    pub fn dialogue_validate(&self, ir_json: String) -> Outcome<()> {
        self.embed.dialogue_validate(&ir_json).map_err(fail)
    }
}
