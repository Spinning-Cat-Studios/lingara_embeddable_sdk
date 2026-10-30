//! `LingaraEmbed`'s graph dialogue methods (ADR 28.10.26f D8): each K1's
//! flat method of the same name, with GDScript types. The story arrives as
//! the `checkpoint_reached` and `dialogue_ended` signals; the idiomatic
//! builder is K2's.

use godot::prelude::*;

use crate::binding::Returns;
use crate::embed_node::LingaraEmbed;
use crate::variant::{from_dictionary, from_variant};

#[godot_api(secondary)]
impl LingaraEmbed {
    /// K1's graph dialogue: `source` is `{type: "ir", ir_json}` or `{type:
    /// "stored", dialogue_id}` (ADR 28.10.26i), `session` K1's
    /// `GraphSession`. `checkpoint_reached` follows; for a stored source,
    /// after `dialogue_loaded` (or `dialogue_load_failed` instead).
    #[func]
    pub fn dialogue_load(&mut self, source: VarDictionary, session: VarDictionary) -> i64 {
        self.flat("dialogue_load", vec![("source", from_dictionary(&source)), ("session", from_dictionary(&session))], Returns::Id).to()
    }

    /// A fact the graph declared: a `bool`, a number or a `String`.
    #[func]
    pub fn dialogue_set_fact(&mut self, dialogue: i64, key: GString, value: Variant) -> bool {
        let fields = vec![("dialogue", dialogue.into()), ("key", key.to_string().into()), ("value", from_variant(&value))];
        self.flat("dialogue_set_fact", fields, Returns::Bool).to()
    }

    /// A `dialogue.v1` document checked as `dialogue_load` would, creating nothing.
    #[func]
    pub fn dialogue_validate(&mut self, ir_json: GString) -> bool {
        self.flat("dialogue_validate", vec![("ir_json", ir_json.to_string().into())], Returns::Bool).to()
    }
}
