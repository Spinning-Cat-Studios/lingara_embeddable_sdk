//! `DemoViewPlugin` (ADR 1.10.26z D10): two coloured stalls, one text
//! panel, and the keys. It reads state and writes `DemoInput`; it never
//! talks to `bevy_lingara` itself.
//!
//! Keys: `1`–`3` say a line, `Tab` talks to the other NPC, `W` walks to the
//! other stall, `+` / `-` change the NPC's level, `P` practises. During a
//! practice: `1`–`4` choose, `Y` / `N` grade yourself, `Space` goes on,
//! `Esc` finishes early.

use std::fmt::Write as _;

use bevy::prelude::*;
use bevy_lingara::{LingaraStatus, NpcDialogue, NpcReplied, NpcTranscript, PracticeSession};

use crate::{DemoInput, DemoNpc, DemoState};

pub struct DemoViewPlugin;

#[derive(Component)]
struct Panel;

impl Plugin for DemoViewPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Startup, draw_stalls).add_systems(Update, (keys, show, log_replies));
    }
}

fn draw_stalls(mut commands: Commands) {
    commands.spawn(Camera2d);
    let stall = |x: f32, color: Color| (Sprite::from_color(color, Vec2::new(160.0, 160.0)), Transform::from_xyz(x, 120.0, 0.0));
    commands.spawn(stall(-200.0, Color::srgb(0.85, 0.35, 0.25)));
    commands.spawn(stall(200.0, Color::srgb(0.25, 0.55, 0.45)));
    let node = Node { position_type: PositionType::Absolute, left: Val::Px(16.0), bottom: Val::Px(16.0), ..default() };
    commands.spawn((Text::new(""), node, Panel));
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
    let mut text = format!("Lingara: {:?}\n", status.phase);
    for (i, entity) in state.npcs.iter().enumerate() {
        let Ok((demo, npc, transcript)) = npcs.get(*entity) else { continue };
        let mark = if i == state.selected { ">" } else { " " };
        let _ = writeln!(text, "{mark} {} (level {}) at {}{}", npc.npc().name, npc.level(), demo.scene, if demo.stall == state.stall { " · you are here" } else { "" });
        if let Some(t) = transcript {
            t.turns.iter().rev().take(2).for_each(|turn| _ = writeln!(text, "    you: {}\n    {}: {}", turn.line, npc.npc().name, turn.reply));
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

/// Bevy's built-in font has no CJK glyphs, so each reply is logged too.
fn log_replies(mut replies: MessageReader<NpcReplied>, npcs: Query<&NpcDialogue>) {
    for reply in replies.read() {
        let name = npcs.get(reply.entity).map(|n| n.npc().name.clone()).unwrap_or_default();
        info!("{name} (level {}): {}", reply.level, reply.text);
    }
}
