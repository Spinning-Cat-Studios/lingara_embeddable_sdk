//! The demo with no window (ADR 1.10.26z D12): `MinimalPlugins` and
//! `DemoLogicPlugin`, against `conformance-server serve` and the token
//! server's `--fake` mode, both on loopback ports of their own choosing.
//! `make bevy-demo-headless` builds the two binaries first.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use bevy::MinimalPlugins;
use bevy::app::App;
use bevy::ecs::message::{Message, Messages};
use bevy_lingara::lingara_embed::PracticePhase;
use bevy_lingara::{NpcReplied, PracticeFinished, PracticeSession};
use bevy_lingara_demo::{DemoConfig, DemoInput, DemoLogicPlugin, DemoState};
use serde_json::Value;

const CASE: &str = "demo.talk-and-practise";

/// A child process killed when the test ends, however it ends.
struct Server(Child);

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A workspace binary beside this test's own `deps/` directory.
fn binary(name: &str) -> PathBuf {
    let exe = std::env::current_exe().expect("the test's path");
    let bin = exe.parent().and_then(Path::parent).expect("target/<profile>").join(name);
    assert!(bin.exists(), "{} is not built: run `make bevy-demo-headless`", bin.display());
    bin
}

/// Starts `name` and waits for its `listening <port>` line.
fn start(name: &str, args: &[&str]) -> (Server, u16) {
    let mut child = Command::new(binary(name)).args(args).current_dir(root()).stdout(Stdio::piped()).spawn().expect("spawn");
    let mut line = String::new();
    BufReader::new(child.stdout.take().expect("stdout")).read_line(&mut line).expect("a first line");
    let port = line.trim().strip_prefix("listening ").and_then(|p| p.parse().ok()).unwrap_or_else(|| panic!("{name} said {line:?}"));
    (Server(child), port)
}

fn take<M: Message>(app: &mut App) -> Vec<M> {
    app.world_mut().resource_mut::<Messages<M>>().drain().collect()
}

/// Frames until `done` holds, for at most 10 s of real I/O on loopback.
fn until<T>(app: &mut App, what: &str, mut done: impl FnMut(&mut App) -> Option<T>) -> T {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        app.update();
        if let Some(found) = done(app) {
            return found;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("timed out waiting for {what}");
}

fn press(app: &mut App, input: DemoInput) {
    app.world_mut().write_message(input);
    app.update();
}

fn phase(app: &App) -> Option<PracticePhase> {
    let player = app.world().resource::<DemoState>().player;
    app.world().get::<PracticeSession>(player).map(|s| s.snapshot().phase)
}

fn talk(app: &mut App, input: DemoInput) -> NpcReplied {
    press(app, input);
    until(app, "an NPC reply", |app| take::<NpcReplied>(app).pop())
}

fn practise(app: &mut App) -> PracticeFinished {
    press(app, DemoInput::StartPractice);
    until(app, "the plan", |app| (phase(app) == Some(PracticePhase::Unanswered)).then_some(()));
    for input in [DemoInput::Choose(1), DemoInput::Continue, DemoInput::SelfGrade(true), DemoInput::Continue, DemoInput::Continue] {
        press(app, input);
    }
    until(app, "the report", |app| take::<PracticeFinished>(app).pop())
}

/// 1.10.26z AC11: turns at levels 2, 6 and 3 (the third with Mei's
/// two-entry window, which the case checks), a practice that reaches
/// `reported`, and a `pass` verdict: exactly one report was sent.
#[test]
fn the_demo_talks_at_two_levels_and_practises() {
    let cases = root().join("bevy/demo/tests/cases");
    let (_api, api) = start("conformance-server", &["serve", "--cases", cases.to_str().unwrap(), "--port", "0"]);
    let (_tokens, tokens) = start("lingara-demo-token-server", &["--fake", "--port", "0"]);
    let control = |verb: &str| format!("http://127.0.0.1:{api}/__conformance/{verb}/{CASE}");
    let http = reqwest::blocking::Client::new();
    http.post(control("arm")).send().and_then(|r| r.error_for_status()).expect("armed");

    let config = DemoConfig { token_url: format!("http://127.0.0.1:{tokens}/token"), api_url: Some(format!("http://127.0.0.1:{api}")) };
    let mut app = App::new();
    app.add_plugins(MinimalPlugins).add_plugins(DemoLogicPlugin::new(config));
    app.update();
    let levels = [talk(&mut app, DemoInput::Say(0)).level, {
        press(&mut app, DemoInput::SelectOther);
        talk(&mut app, DemoInput::Say(0)).level
    }, {
        press(&mut app, DemoInput::SelectOther);
        press(&mut app, DemoInput::LevelUp);
        talk(&mut app, DemoInput::Say(1)).level
    }];
    assert_eq!(levels, [2, 6, 3]);
    let finished = practise(&mut app);
    assert_eq!(finished.snapshot.phase, PracticePhase::Reported);

    let verdict: Value = http.post(control("finish")).json(&serde_json::json!({})).send().and_then(|r| r.json()).expect("a verdict");
    assert_eq!(verdict["result"], "pass", "{verdict}");
}
