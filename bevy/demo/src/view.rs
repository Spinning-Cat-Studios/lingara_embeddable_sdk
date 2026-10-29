//! `DemoViewPlugin` (ADR 1.10.26z D10): two coloured stalls, one text
//! panel, and the keys. It reads state and writes `DemoInput`; it never
//! talks to `bevy_lingara` itself.
//!
//! The keys are [`HELP`], which `H` shows in the window (ADR 15.10.26p D3).

use std::fmt::Write as _;

use bevy::prelude::*;
use bevy_lingara::{LingaraStatus, NpcDialogue, NpcReplied, NpcTranscript, PracticeSession};

use crate::{DemoInput, DemoNpc, DemoState};

/// The README's key table, as the `H` panel shows it. `H` itself is view
/// state, never a `DemoInput`, so it means the same thing in every state.
const HELP: &str = "\
1-3   say a line to the NPC you are talking to
Tab   talk to the other NPC, without walking
W     walk to the other stall
+ / - raise or lower this NPC's level
P     practise, at this NPC's language and level
H     show or hide these keys
During practice:
  1-4    choose an option
  Y / N  grade yourself
  Space  reveal, or go on
  Esc    finish early (it still reports)";

pub struct DemoViewPlugin;

#[derive(Component)]
struct Panel;

#[derive(Component)]
struct HelpPanel;

impl Plugin for DemoViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_stalls).add_systems(Update, (keys, toggle_help, show, log_replies));
    }
}

/// Sans-serif through the OS's font fallback, which supplies the CJK glyphs
/// and the `·` Bevy's built-in face lacks (ADR 15.10.26p D1).
fn font() -> TextFont {
    TextFont { font: FontSource::sans_serif(), font_size: FontSize::Px(18.0), ..default() }
}

/// Translucent, so text over a stall stays readable.
fn backdrop() -> BackgroundColor {
    BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7))
}

fn draw_stalls(mut commands: Commands) {
    commands.spawn(Camera2d);
    let stall = |x: f32, color: Color| (Sprite::from_color(color, Vec2::new(160.0, 160.0)), Transform::from_xyz(x, 120.0, 0.0));
    commands.spawn(stall(-200.0, Color::srgb(0.85, 0.35, 0.25)));
    commands.spawn(stall(200.0, Color::srgb(0.25, 0.55, 0.45)));
    // `left` and `right` fix the width, so parley wraps inside the window;
    // `max_height` and the clip are the backstop against the top edge.
    let node = Node {
        position_type: PositionType::Absolute,
        left: Val::Px(16.0),
        right: Val::Px(16.0),
        bottom: Val::Px(16.0),
        max_height: Val::Percent(60.0),
        overflow: Overflow::clip(),
        padding: UiRect::all(Val::Px(8.0)),
        ..default()
    };
    commands.spawn((Text::new(""), font(), node, backdrop(), Panel));
    let help = Node { position_type: PositionType::Absolute, left: Val::Px(16.0), top: Val::Px(16.0), padding: UiRect::all(Val::Px(8.0)), ..default() };
    commands.spawn((Text::new(HELP), font(), help, backdrop(), Visibility::Hidden, HelpPanel));
}

fn toggle_help(keys: Res<ButtonInput<KeyCode>>, mut help: Single<&mut Visibility, With<HelpPanel>>) {
    if keys.just_pressed(KeyCode::KeyH) {
        **help = if **help == Visibility::Hidden { Visibility::Visible } else { Visibility::Hidden };
    }
}

const DIGITS: [KeyCode; 4] = [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4];

fn keys(keys: Res<ButtonInput<KeyCode>>, state: Res<DemoState>, sessions: Query<&PracticeSession>, mut out: MessageWriter<DemoInput>) {
    let practising = sessions.get(state.player).is_ok_and(|s| !s.is_terminal() && !s.is_detached());
    for (n, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key) {
            out.write(if practising { DemoInput::Choose(n as u32) } else { DemoInput::Say(n) });
        }
    }
    let map = [
        (KeyCode::Tab, DemoInput::SelectOther),
        (KeyCode::KeyW, DemoInput::Walk),
        (KeyCode::Equal, DemoInput::LevelUp),
        (KeyCode::NumpadAdd, DemoInput::LevelUp),
        (KeyCode::Minus, DemoInput::LevelDown),
        (KeyCode::KeyP, DemoInput::StartPractice),
        (KeyCode::KeyY, DemoInput::SelfGrade(true)),
        (KeyCode::KeyN, DemoInput::SelfGrade(false)),
        (KeyCode::Space, DemoInput::Continue),
        (KeyCode::Escape, DemoInput::Finish),
    ];
    out.write_batch(map.into_iter().filter(|(key, _)| keys.just_pressed(*key)).map(|(_, input)| input));
}

fn show(
    state: Res<DemoState>,
    status: Res<LingaraStatus>,
    npcs: Query<(&DemoNpc, &NpcDialogue, Option<&NpcTranscript>)>,
    sessions: Query<&PracticeSession>,
    mut panel: Single<&mut Text, With<Panel>>,
) {
    let mut text = format!("Lingara: {:?} · H: keys\n", status.phase);
    for (i, entity) in state.npcs.iter().enumerate() {
        let Ok((demo, npc, transcript)) = npcs.get(*entity) else { continue };
        let selected = i == state.selected;
        let mark = if selected { ">" } else { " " };
        let _ = writeln!(text, "{mark} {} (level {}) at {}{}", npc.npc().name, npc.level(), demo.scene, if demo.stall == state.stall { " · you are here" } else { "" });
        // Only the selected NPC's last two turns, oldest first, so the
        // panel stays under its height cap (ADR 15.10.26p D2).
        if let Some(t) = transcript.filter(|_| selected) {
            t.turns.iter().skip(t.turns.len().saturating_sub(2)).for_each(|turn| _ = writeln!(text, "    you: {}\n    {}: {}", turn.line, npc.npc().name, turn.reply));
            if let Some(streaming) = &t.streaming {
                let _ = writeln!(text, "    …{streaming}");
            }
        }
    }
    if let Ok(session) = sessions.get(state.player) {
        let s = session.snapshot();
        let _ = writeln!(text, "practice: {:?}, set {}/{}, question {}/{}", s.phase, s.set_index + 1, s.set_count, s.question_index + 1, s.question_count);
        if let Some(q) = &s.question {
            let _ = writeln!(text, "  {}  {:?}", q.prompt, q.options.clone().unwrap_or_default());
        }
    }
    panel.0 = text;
}

/// Every reply in full, whatever the panel shows.
fn log_replies(mut replies: MessageReader<NpcReplied>, npcs: Query<&NpcDialogue>) {
    for reply in replies.read() {
        let name = npcs.get(reply.entity).map(|n| n.npc().name.clone()).unwrap_or_default();
        info!("{name} (level {}): {}", reply.level, reply.text);
    }
}
