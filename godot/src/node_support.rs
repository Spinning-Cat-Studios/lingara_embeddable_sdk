//! The node's three free helpers (ADR 1.10.26ab D2, D3): project settings
//! as property defaults, the real start function, and `Log` routing.

use godot::classes::ProjectSettings;
use godot::global::{print_verbose, push_error, push_warning};
use godot::meta::{FromGodot, ToGodot};
use godot::obj::Singleton;
use lingara_embed::Embed;
use serde_json::Value;

use crate::binding::StartFn;
use crate::convert::SignalCall;
use crate::pump::Core;

/// A `lingara/<name>` project setting, else `default` (D2).
pub fn setting<T: FromGodot>(name: &str, default: T) -> T {
    let settings = ProjectSettings::singleton();
    let key = format!("lingara/{name}");
    if !settings.has_setting(&key) {
        return default;
    }
    settings.get_setting(&key).try_to::<T>().unwrap_or(default)
}

pub fn start_embed() -> StartFn {
    Box::new(|config| Embed::start(config).map(|e| Box::new(e) as Box<dyn Core>))
}

/// D3's `Log` routing: `error` to `push_error`, `warn` to `push_warning`,
/// the rest to `print_verbose`.
pub fn forward_log(call: &SignalCall) {
    let text = |i: usize| call.args.get(i).and_then(Value::as_str).unwrap_or_default().to_owned();
    let line = format!("[lingara {}] {}", text(1), text(2)).to_variant();
    match text(0).as_str() {
        "error" => push_error(&[line]),
        "warn" => push_warning(&[line]),
        _ => print_verbose(&[line]),
    }
}
