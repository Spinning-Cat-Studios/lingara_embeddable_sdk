//! The Bevy demo (ADR 1.10.26z D10), as two plugins so its logic runs with
//! no window. `DemoLogicPlugin` holds two NPCs at two levels and a player
//! who can practise, and turns the demo's own `DemoInput` Messages into
//! `bevy_lingara`'s. `DemoViewPlugin` (in `view`) draws it and turns keys
//! into `DemoInput`. A headless test drives the first alone.

pub mod view;

use bevy::prelude::*;
use bevy_lingara::lingara_embed::{Npc, PracticePhase, PracticeSource, WorldContext};
use bevy_lingara::{LingaraConfig, LingaraPlugin, NpcDialogue, NpcSay, PracticeAction, PracticeInput, PracticeSession, StartPractice, WorldContextChanged};

pub use view::DemoViewPlugin;

/// Where the demo gets its tokens and its API: D11's token server, and
/// production unless `api_url` says otherwise. It holds no secret.
#[derive(Debug, Clone)]
pub struct DemoConfig {
    pub token_url: String,
    pub api_url: Option<String>,
}

impl DemoConfig {
    /// `LINGARA_DEMO_TOKEN_URL` and `LINGARA_DEMO_API_URL`, when set.
    pub fn from_env() -> Self {
        let token_url = std::env::var("LINGARA_DEMO_TOKEN_URL").unwrap_or_else(|_| "http://127.0.0.1:8787/token".into());
        Self { token_url, api_url: std::env::var("LINGARA_DEMO_API_URL").ok() }
    }
}

/// One of the two stall-keepers.
#[derive(Component, Debug, Clone)]
pub struct DemoNpc {
    pub stall: usize,
    pub scene: &'static str,
    pub topic: &'static str,
    /// The three lines the player can say (keys 1–3).
    pub lines: [&'static str; 3],
}

/// The entity that holds the player's `PracticeSession`.
#[derive(Component, Debug)]
pub struct Player;

#[derive(Resource, Debug)]
pub struct DemoState {
    pub npcs: [Entity; 2],
    /// Who the player is talking to.
    pub selected: usize,
    /// Which stall the player stands at.
    pub stall: usize,
    pub player: Entity,
}

/// What the player does, from a key or a test.
#[derive(Message, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoInput {
    Say(usize),
    /// Talk to the other NPC without walking.
    SelectOther,
    /// Cross to the other stall: one `world.context_changed`.
    Walk,
    LevelUp,
    LevelDown,
    StartPractice,
    Choose(u32),
    SelfGrade(bool),
    /// Reveal an unanswered question, or go on from a reviewed one.
    Continue,
    Finish,
}

pub struct DemoLogicPlugin {
    config: DemoConfig,
}

impl DemoLogicPlugin {
    pub fn new(config: DemoConfig) -> Self {
        Self { config }
    }
}

impl Plugin for DemoLogicPlugin {
    fn build(&self, app: &mut App) {
        let mut lingara = LingaraConfig::new().token_endpoint(self.config.token_url.clone(), vec![]).source_lang("en").stream_on_start(false);
        if let Some(api) = &self.config.api_url {
            lingara = lingara.base_url(api.clone());
        }
        app.add_plugins(LingaraPlugin::new(lingara))
            .add_message::<DemoInput>()
            .add_systems(Startup, spawn_world)
            .add_systems(Update, (talk, practise));
    }
}

fn npc(name: &str, persona: &str, lang: &str, level: u8, scene: &str) -> NpcDialogue {
    let built = NpcDialogue::new(name, lang, level).and_then(|n| n.with_persona(persona)).and_then(|n| n.with_scene(scene));
    built.expect("the demo's NPCs are within the inbound bounds")
}

fn spawn_world(mut commands: Commands) {
    let mei = DemoNpc { stall: 0, scene: "the night market", topic: "buying food at a market", lines: ["你好！", "饺子多少钱？", "谢谢，再见！"] };
    let tanaka = DemoNpc {
        stall: 1,
        scene: "a quiet tea house",
        topic: "talking about books",
        lines: ["こんにちは、先生。", "最近どんな本を読んでいらっしゃいますか？", "ありがとうございました。"],
    };
    let mei = commands.spawn((npc("Mei", "a dumpling seller at the night market", "zh", 2, mei.scene), mei)).id();
    let tanaka = commands.spawn((npc("Professor Tanaka", "a retired literature professor who loves tea", "ja", 6, tanaka.scene), tanaka)).id();
    let player = commands.spawn(Player).id();
    commands.insert_resource(DemoState { npcs: [mei, tanaka], selected: 0, stall: 0, player });
}

/// Lines, the selected NPC, its level, and walking between stalls.
fn talk(
    mut inputs: MessageReader<DemoInput>,
    mut state: ResMut<DemoState>,
    mut npcs: Query<(&DemoNpc, &mut NpcDialogue)>,
    mut say: MessageWriter<NpcSay>,
    mut context: MessageWriter<WorldContextChanged>,
) {
    for input in inputs.read() {
        let entity = state.npcs[state.selected];
        match *input {
            DemoInput::Say(n) => {
                if let Ok((demo, _)) = npcs.get(entity) {
                    say.write(NpcSay { entity, line: demo.lines[n.min(2)].into() });
                }
            }
            DemoInput::SelectOther => state.selected = 1 - state.selected,
            DemoInput::LevelUp | DemoInput::LevelDown => {
                if let Ok((_, mut dialogue)) = npcs.get_mut(entity) {
                    let level = if *input == DemoInput::LevelUp { dialogue.level().saturating_add(1) } else { dialogue.level().saturating_sub(1) };
                    let _ = dialogue.set_level(level);
                }
            }
            DemoInput::Walk => {
                (state.stall, state.selected) = (1 - state.stall, 1 - state.stall);
                if let Ok((demo, dialogue)) = npcs.get(state.npcs[state.stall]) {
                    context.write(WorldContextChanged(arrived(demo, dialogue)));
                }
            }
            _ => {}
        }
    }
}

/// The context of the stall the player just walked to.
fn arrived(demo: &DemoNpc, npc: &NpcDialogue) -> WorldContext {
    WorldContext {
        scene: demo.scene.into(),
        source_lang: "en".into(),
        target_lang: npc.target_lang().into(),
        level: npc.level(),
        npc: Some(Npc { name: npc.npc().name.clone(), persona: npc.npc().persona.clone() }),
        tags: vec![],
        generate: false,
    }
}

/// A practice over the selected NPC's language and level, and its answers.
fn practise(
    mut inputs: MessageReader<DemoInput>,
    state: Res<DemoState>,
    npcs: Query<(&DemoNpc, &NpcDialogue)>,
    sessions: Query<&PracticeSession>,
    mut out: ParamSet<(MessageWriter<StartPractice>, MessageWriter<PracticeInput>)>,
) {
    let entity = state.player;
    let session = sessions.get(entity).ok();
    for input in inputs.read() {
        if *input != DemoInput::StartPractice {
            for action in actions(*input, session) {
                out.p1().write(PracticeInput { entity, action });
            }
            continue;
        }
        let Ok((demo, dialogue)) = npcs.get(state.npcs[state.selected]) else { continue };
        let source = PracticeSource::Request {
            topic: demo.topic.into(),
            source_lang: "en".into(),
            target_lang: dialogue.target_lang().into(),
            level: dialogue.level(),
            tags: vec![],
        };
        out.p0().write(StartPractice { entity, source });
    }
}

/// The learner's input as K1 calls. A self-graded question is answered
/// (the text is not graded) and then graded by the learner, in one frame.
fn actions(input: DemoInput, session: Option<&PracticeSession>) -> Vec<PracticeAction> {
    let unanswered = session.is_some_and(|s| s.snapshot().phase == PracticePhase::Unanswered);
    let self_graded = session.and_then(|s| s.snapshot().question.as_ref()).is_some_and(|q| q.self_graded);
    match input {
        DemoInput::Choose(n) => vec![PracticeAction::Choose(n)],
        DemoInput::SelfGrade(c) if unanswered && self_graded => vec![PracticeAction::Answer(String::new()), PracticeAction::SelfGrade(c)],
        DemoInput::SelfGrade(c) => vec![PracticeAction::SelfGrade(c)],
        DemoInput::Continue if unanswered => vec![PracticeAction::Reveal],
        DemoInput::Continue => vec![PracticeAction::Next],
        DemoInput::Finish => vec![PracticeAction::Finish],
        _ => vec![],
    }
}
