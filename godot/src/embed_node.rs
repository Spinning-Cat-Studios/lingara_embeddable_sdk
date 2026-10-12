//! `LingaraEmbed`, the node a Godot game talks to (ADR 1.10.26ab D2, D3).
//! Its `_process` drains K1's queue once per frame and emits one signal per
//! event; its methods take and return `Dictionary`s in K1's own JSON.
//! Everything below the Godot surface is `binding.rs`; this file declares
//! the class, its properties and signals, and its lifecycle. The flat
//! methods are `#[godot_api(secondary)]` blocks in `node_methods.rs` and
//! `node_practice.rs`.

use godot::classes::node::ProcessMode;
use godot::classes::{INode, Node};
use godot::global::push_warning;
use godot::prelude::*;
use lingara_embed::{Command, EmbedError};
use serde_json::Value;

use crate::binding::{Binding, Returns};
use crate::commands;
use crate::config::Props;
use crate::node_support::{forward_log, setting, start_embed};
use crate::pump::LIBRARY;
use crate::variant::{from_dictionary, to_variant};

#[derive(GodotClass)]
#[class(base = Node)]
pub struct LingaraEmbed {
    base: Base<Node>,
    binding: Binding,
    #[export]
    base_url: GString,
    #[export]
    token_url: GString,
    #[export]
    api_version: GString,
    #[export]
    autostart: bool,
    #[export]
    stream_on_start: bool,
    #[export]
    queue_capacity: i64,
    #[export]
    log_level: GString,
    #[export]
    user_agent_suffix: GString,
    #[export]
    max_events_per_frame: i64,
    #[export]
    forward_logs: bool,
    #[export]
    extra_config: VarDictionary,
}

#[godot_api]
impl INode for LingaraEmbed {
    fn init(base: Base<Node>) -> Self {
        let k1 = Props::default();
        base.to_init_gd().set_process_mode(ProcessMode::ALWAYS);
        Self {
            base,
            binding: Binding::new(LIBRARY.clone(), start_embed()),
            base_url: setting("base_url", GString::from(k1.base_url.as_str())),
            token_url: setting("token_url", GString::new()),
            api_version: setting("api_version", GString::new()),
            autostart: setting("autostart", true),
            stream_on_start: setting("stream_on_start", k1.stream_on_start),
            queue_capacity: k1.queue_capacity,
            log_level: GString::from(k1.log_level.as_str()),
            user_agent_suffix: GString::new(),
            max_events_per_frame: 256,
            forward_logs: true,
            extra_config: VarDictionary::new(),
        }
    }

    fn ready(&mut self) {
        if self.autostart {
            self.start();
        }
    }

    fn process(&mut self, _delta: f64) {
        let max = u32::try_from(self.max_events_per_frame.max(1)).unwrap_or(u32::MAX);
        self.binding.frame(max);
        self.flush();
    }

    fn exit_tree(&mut self) {
        self.binding.stop();
    }
}

#[godot_api]
impl LingaraEmbed {
    #[signal]
    fn core_event(event: VarDictionary);
    #[signal]
    fn started();
    #[signal]
    fn token_needed(request: i64, reason: GString);
    #[signal]
    fn stream_changed(state: VarDictionary, cursor: GString);
    #[signal]
    fn event_received(id: GString, event_type: GString, created_at: GString, data: VarDictionary);
    #[signal]
    fn world_sent(request: i64, event_id: GString, reaction: VarDictionary);
    #[signal]
    fn practice_changed(session: i64, snapshot: VarDictionary);
    #[signal]
    fn practice_finished(session: i64, snapshot: VarDictionary);
    #[signal]
    fn npc_turn_opened(dialogue: i64, request: i64);
    #[signal]
    fn npc_fragment(dialogue: i64, text: GString);
    #[signal]
    fn npc_notice(dialogue: i64, code: GString, message: GString);
    #[signal]
    fn npc_replied(dialogue: i64, text: GString);
    #[signal]
    fn npc_turn_cancelled(dialogue: i64);
    #[signal]
    fn error_raised(error: VarDictionary);
    #[signal]
    fn log_received(level: GString, target: GString, message: GString);
    #[signal]
    fn closed(reason: GString);

    /// Builds K1's handle from the properties and emits `started`.
    #[func]
    pub fn start(&mut self) -> bool {
        let config = self.props().config(self.binding.tokens.source(&self.token_url.to_string()));
        let started = self.binding.start(config);
        if started == Some(true) {
            push_warning(&["LingaraEmbed: another Lingara node is already running; one per game (two streams, two token sources)".to_variant()]);
        }
        self.flush();
        started.is_some()
    }

    /// K1's `close()`: returns at once and never joins (D2).
    #[func]
    pub fn stop(&mut self) {
        self.binding.stop();
    }

    #[func]
    pub fn is_running(&self) -> bool {
        self.binding.is_running()
    }

    #[func]
    pub fn status(&self) -> Variant {
        to_variant(&self.binding.status())
    }

    #[func]
    pub fn last_error(&self) -> Variant {
        to_variant(&self.binding.last_error())
    }

    /// Any K1 `Command`: `{"ok": …}` or `{"error": …}`.
    #[func]
    pub fn execute(&mut self, command: VarDictionary) -> Variant {
        let answer = self.binding.execute(from_dictionary(&command));
        self.flush();
        to_variant(&answer)
    }
}

impl LingaraEmbed {
    fn props(&self) -> Props {
        Props {
            base_url: self.base_url.to_string(),
            api_version: self.api_version.to_string(),
            stream_on_start: self.stream_on_start,
            queue_capacity: self.queue_capacity,
            log_level: self.log_level.to_string(),
            user_agent_suffix: self.user_agent_suffix.to_string(),
            extra_config: from_dictionary(&self.extra_config),
        }
    }

    /// A flat method: `kind`'s command, its answer, then every owed signal
    /// emitted before the value returns to GDScript.
    pub(crate) fn flat(&mut self, kind: &str, fields: Vec<(&str, Value)>, returns: Returns) -> Variant {
        self.run(commands::build(kind, fields), returns)
    }

    pub(crate) fn run(&mut self, command: Result<Command, EmbedError>, returns: Returns) -> Variant {
        let answer = self.binding.call(command, returns);
        self.flush();
        to_variant(&answer)
    }

    pub(crate) fn binding_mut(&mut self) -> &mut Binding {
        &mut self.binding
    }

    /// Emits the owed signals. `base_mut()` lets a connected handler call
    /// back into this node while it runs.
    pub(crate) fn flush(&mut self) {
        for call in self.binding.take_signals() {
            if call.name == "log_received" && self.forward_logs {
                forward_log(&call);
            }
            let args: Vec<Variant> = call.args.iter().map(to_variant).collect();
            self.base_mut().emit_signal(call.name, &args);
        }
    }
}
