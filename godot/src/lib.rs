//! `lingara_godot`: the GDExtension behind `addons/lingara/` (ADR
//! 1.10.26ab). One `LingaraEmbed` node over K1's `lingara-embed`, on the
//! main thread only: K1's own thread never touches a Godot object.
//!
//! `convert.rs`, `commands.rs`, `config.rs`, `token.rs`, `port.rs`,
//! `pump.rs` and `binding.rs` hold no Godot type, so `cargo test` covers
//! them; `variant.rs`, `embed_node.rs` and the three `node_*` files are the
//! Godot surface the in-engine tests reach.

use godot::init::{ExtensionLibrary, InitStage, gdextension};

mod binding;
mod commands;
mod config;
mod convert;
mod embed_node;
mod node_graph;
mod node_methods;
mod node_practice;
mod node_support;
mod port;
mod pump;
mod token;
mod variant;

#[cfg(test)]
mod tests;

struct LingaraGodot;

#[gdextension]
unsafe impl ExtensionLibrary for LingaraGodot {
    /// Unloading joins every closed handle, bounded, so no K1 thread still
    /// runs code from this library when Godot unmaps it (D2). gdext 0.5
    /// names D2's `on_level_deinit` `on_stage_deinit`.
    fn on_stage_deinit(stage: InitStage) {
        if stage == InitStage::Scene {
            pump::LIBRARY.join_all(pump::UNLOAD_JOIN_MS);
        }
    }
}
