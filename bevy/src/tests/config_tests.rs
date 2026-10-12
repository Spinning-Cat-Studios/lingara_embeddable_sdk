//! The kit names itself (D3, EK10).

use super::fake_core::{Fake, app};
use crate::config::LingaraConfig;

/// 1.10.26z AC17: `bevy_lingara/<version>`, then the game's own suffix.
#[test]
fn the_user_agent_names_the_plugin_then_the_game() {
    let kit = format!("bevy_lingara/{}", env!("CARGO_PKG_VERSION"));
    let fake = Fake::default();
    let _plain = app(LingaraConfig::new(), &fake);
    let _named = app(LingaraConfig::new().user_agent_suffix("my-game/1.4"), &fake);
    let suffixes: Vec<Option<String>> = fake.state().starts.iter().map(|c| c.user_agent_suffix.clone()).collect();
    assert_eq!(suffixes, [Some(kit.clone()), Some(format!("{kit} my-game/1.4"))]);
}
