//! The windowed Bevy demo. Start the token server first (README.md).

use bevy::prelude::*;
use bevy_lingara_demo::{DemoConfig, DemoLogicPlugin, DemoViewPlugin};

fn main() {
    App::new().add_plugins(DefaultPlugins).add_plugins(DemoLogicPlugin::new(DemoConfig::from_env())).add_plugins(DemoViewPlugin).run();
}
