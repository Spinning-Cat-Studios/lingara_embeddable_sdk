//! `LingaraEmbed`'s token, stream, world and dialogue methods (ADR
//! 1.10.26ab D3, D4): each one K1's flat method of the same name, with
//! GDScript types. Ids are K1's, unchanged; a refusal returns D3's sentinel
//! after emitting `error_raised`.

use godot::prelude::*;

use crate::binding::Returns;
use crate::commands;
use crate::embed_node::LingaraEmbed;
use crate::variant::from_dictionary;

fn text(s: &GString) -> serde_json::Value {
    s.to_string().into()
}

#[godot_api(secondary)]
impl LingaraEmbed {
    /// The handoff body's two keys: `provide_token(body.token, body.expires_at)`.
    #[func]
    pub fn provide_token(&mut self, token: GString, expires_at: GString) -> bool {
        self.flat("provide_token", vec![("token", text(&token)), ("expires_at", text(&expires_at))], Returns::Bool).to()
    }

    /// B1's whole answer forwarded: the lifetime from its `expires_in`.
    #[func]
    pub fn provide_token_for(&mut self, token: GString, expires_in_secs: i64) -> bool {
        self.flat("provide_token_for", vec![("token", text(&token)), ("expires_in_secs", expires_in_secs.into())], Returns::Bool).to()
    }

    /// The game could not fetch a token: K1's waiting calls fail now.
    #[func]
    pub fn token_unavailable(&mut self, reason: GString) {
        self.flat("token_unavailable", vec![("reason", text(&reason))], Returns::Unit);
    }

    /// Before `start()`, kept for it in memory; after, K1's own command.
    #[func]
    pub fn set_token_endpoint(&mut self, url: GString, headers: VarDictionary) -> bool {
        let answer = self.binding_mut().set_token_endpoint(&url.to_string(), from_dictionary(&headers));
        self.flush();
        answer
    }

    #[func]
    pub fn stream_pause(&mut self) {
        self.flat("stream_pause", Vec::new(), Returns::Unit);
    }

    #[func]
    pub fn stream_resume(&mut self) {
        self.flat("stream_resume", Vec::new(), Returns::Unit);
    }

    /// K1's `WorldContext`; answered by `world_sent`.
    #[func]
    pub fn send_context(&mut self, context: VarDictionary) -> i64 {
        self.flat("send_context", vec![("context", from_dictionary(&context))], Returns::Id).to()
    }

    /// K1's `DialogueConfig`: `npc { name, persona? }`, `scene?`,
    /// `source_lang`, `target_lang`, `level`, `window?`.
    #[func]
    pub fn dialogue_open(&mut self, config: VarDictionary) -> i64 {
        self.flat("dialogue_open", vec![("config", from_dictionary(&config))], Returns::Id).to()
    }

    /// The turn's request id; `-1` (`busy`) while a turn is in flight.
    #[func]
    pub fn dialogue_say(&mut self, dialogue: i64, line: GString) -> i64 {
        self.flat("dialogue_say", vec![("dialogue", dialogue.into()), ("line", text(&line))], Returns::Id).to()
    }

    #[func]
    pub fn dialogue_set_level(&mut self, dialogue: i64, level: i64) -> bool {
        self.flat("dialogue_set_level", vec![("dialogue", dialogue.into()), ("level", level.into())], Returns::Bool).to()
    }

    /// An empty scene clears it.
    #[func]
    pub fn dialogue_set_scene(&mut self, dialogue: i64, scene: GString) -> bool {
        let scene = commands::scene(&scene.to_string());
        self.flat("dialogue_set_scene", vec![("dialogue", dialogue.into()), ("scene", scene)], Returns::Bool).to()
    }

    #[func]
    pub fn dialogue_cancel(&mut self, dialogue: i64) -> bool {
        self.flat("dialogue_cancel", vec![("dialogue", dialogue.into())], Returns::Bool).to()
    }

    #[func]
    pub fn dialogue_close(&mut self, dialogue: i64) -> bool {
        self.flat("dialogue_close", vec![("dialogue", dialogue.into())], Returns::Bool).to()
    }

    /// Its `window` can seed the next `dialogue_open`; a graph's `graph`,
    /// the next `dialogue_load` (`node_graph.rs`). A stored graph still
    /// being fetched answers `phase: "loading"` and no `graph`.
    #[func]
    pub fn dialogue_snapshot(&mut self, dialogue: i64) -> VarDictionary {
        self.flat("dialogue_snapshot", vec![("dialogue", dialogue.into())], Returns::Dialogue).to()
    }
}
