//! A `command:` step through the plugin (ADR 1.10.26z D12's table): each
//! K1 `Command` becomes the Message, component or removal a game would use,
//! and only what the plugin does not wrap goes to `Lingara::core().execute`.

use bevy::ecs::entity::Entity;
use bevy::ecs::message::Message;
use bevy_lingara::lingara_embed::{Command, CommandOutcome, DialogueConfig, EmbedError};
use bevy_lingara::{
    NpcCancel, NpcDialogue, NpcSay, NpcTranscript, PracticeAction, PracticeInput, PracticeSession, ProvideToken, SetTokenEndpoint,
    StartPractice, TokenUnavailable, WorldContextChanged,
};
use serde_json::Value;

use crate::host::{Run, refusal};

fn substitute(value: &mut Value, last: Option<u64>) {
    match value {
        Value::String(s) if s == "$last" => *value = last.map_or(Value::Null, Value::from),
        Value::Object(map) => map.values_mut().for_each(|v| substitute(v, last)),
        Value::Array(items) => items.iter_mut().for_each(|v| substitute(v, last)),
        _ => {}
    }
}

/// K1's open request as the component; a bound it refuses is K1's `InvalidInput`.
fn npc(config: DialogueConfig) -> Result<NpcDialogue, EmbedError> {
    let mut npc = NpcDialogue::new(config.npc.name, config.target_lang, config.level)?;
    if let Some(persona) = config.npc.persona {
        npc = npc.with_persona(persona)?;
    }
    if let Some(scene) = config.scene {
        npc = npc.with_scene(scene)?;
    }
    Ok(npc.with_source_lang(config.source_lang).with_window(config.window))
}

/// The dialogue a command addresses, when it addresses one.
fn addressed(command: &Command) -> Option<u64> {
    match command {
        Command::DialogueSay { dialogue, .. }
        | Command::DialogueSetLevel { dialogue, .. }
        | Command::DialogueSetScene { dialogue, .. }
        | Command::DialogueCancel { dialogue }
        | Command::DialogueClose { dialogue }
        | Command::DialogueSnapshot { dialogue }
        | Command::DialogueSetFact { dialogue, .. } => Some(*dialogue),
        _ => None,
    }
}

fn practice(command: &Command) -> Option<(u64, PracticeAction)> {
    Some(match command.clone() {
        Command::PracticeAnswer { session, answer } => (session, PracticeAction::Answer(answer)),
        Command::PracticeChoose { session, option } => (session, PracticeAction::Choose(option)),
        Command::PracticeReveal { session } => (session, PracticeAction::Reveal),
        Command::PracticeSelfGrade { session, correct } => (session, PracticeAction::SelfGrade(correct)),
        Command::PracticeNext { session } => (session, PracticeAction::Next),
        Command::PracticeFinish { session } => (session, PracticeAction::Finish),
        _ => return None,
    })
}

impl Run {
    /// A `Command` in its JSON shape; `"$last"` is the id the latest
    /// `practice_start`, `dialogue_open` or `dialogue_load` answered with.
    pub fn command(&mut self, body: &Value) -> Result<(), String> {
        let mut body = body.clone();
        substitute(&mut body, self.last_id);
        let command: Command = serde_json::from_value(body).map_err(|e| format!("not a Command: {e}"))?;
        if matches!(command, Command::DialogueLoad { .. }) || addressed(&command).is_some_and(|d| self.graphs.contains(&d)) {
            return self.graph(command);
        }
        if let Some((session, action)) = practice(&command) {
            let entity = self.entity(session);
            return self.send(PracticeInput { entity, action });
        }
        match command {
            Command::SendContext { context } => self.send(WorldContextChanged(context)),
            Command::ProvideToken { token, expires_at, update_available } => self.send(ProvideToken::with_update(token, expires_at, update_available)),
            Command::TokenUnavailable { reason } => self.send(TokenUnavailable { reason }),
            Command::SetTokenEndpoint { url, headers } => self.send(SetTokenEndpoint { url, headers }),
            Command::PracticeStart { source } => {
                let entity = self.app()?.world_mut().spawn_empty().id();
                self.send(StartPractice { entity, source })?;
                self.opened(entity, |w, e| w.get::<PracticeSession>(e).map(PracticeSession::id))
            }
            Command::DialogueOpen { config } => self.open(config),
            other => self.dialogue(other),
        }
    }

    /// A graph dialogue's command, through `Lingara::core()` (ADR 28.10.26f D8).
    fn graph(&mut self, command: Command) -> Result<(), String> {
        let loads = matches!(command, Command::DialogueLoad { .. });
        match self.core(|core| core.execute(command))? {
            Ok(CommandOutcome::Id { id }) if loads => {
                self.last_id = Some(id);
                self.graphs.insert(id);
            }
            Ok(_) => {}
            Err(e) => self.seen.push_back(refusal(&e)),
        }
        self.pump()
    }

    fn dialogue(&mut self, command: Command) -> Result<(), String> {
        match command {
            Command::DialogueSay { dialogue, line } => {
                let entity = self.entity(dialogue);
                self.send(NpcSay { entity, line })
            }
            Command::DialogueCancel { dialogue } => {
                let entity = self.entity(dialogue);
                self.send(NpcCancel { entity })
            }
            Command::DialogueSetLevel { dialogue, level } => self.edit(dialogue, |npc| npc.set_level(level)),
            Command::DialogueSetScene { dialogue, scene } => self.edit(dialogue, |npc| npc.set_scene(scene)),
            Command::DialogueClose { dialogue } => {
                let entity = self.entity(dialogue);
                self.app()?.world_mut().entity_mut(entity).remove::<NpcDialogue>();
                self.pump()
            }
            other => {
                if let Err(e) = self.core(|core| core.execute(other))? {
                    self.seen.push_back(refusal(&e));
                }
                self.pump()
            }
        }
    }

    fn open(&mut self, config: DialogueConfig) -> Result<(), String> {
        match npc(config) {
            Ok(npc) => {
                let entity = self.app()?.world_mut().spawn(npc).id();
                self.pump()?;
                self.opened(entity, |w, e| w.get::<NpcTranscript>(e).and_then(NpcTranscript::id))
            }
            Err(e) => {
                self.seen.push_back(refusal(&e));
                Ok(())
            }
        }
    }

    /// The id the plugin's component on `entity` now holds, if any.
    fn opened(&mut self, entity: Entity, id: fn(&bevy::ecs::world::World, Entity) -> Option<u64>) -> Result<(), String> {
        if let Some(id) = id(self.app()?.world(), entity) {
            self.last_id = Some(id);
            self.entities.insert(id, entity);
        }
        Ok(())
    }

    /// The entity holding K1's `id`; an unknown id gets a fresh, empty one,
    /// so the plugin answers `UnknownId` as the core would.
    fn entity(&mut self, id: u64) -> Entity {
        if let Some(&entity) = self.entities.get(&id) {
            return entity;
        }
        let entity = self.app.as_mut().map(|a| a.world_mut().spawn_empty().id()).unwrap_or(Entity::PLACEHOLDER);
        self.entities.insert(id, entity);
        entity
    }

    fn edit(&mut self, dialogue: u64, change: impl FnOnce(&mut NpcDialogue) -> Result<(), EmbedError>) -> Result<(), String> {
        let entity = self.entity(dialogue);
        let refused = match self.app()?.world_mut().get_mut::<NpcDialogue>(entity) {
            Some(mut npc) => change(&mut npc).err(),
            None => None,
        };
        if let Some(e) = refused {
            self.seen.push_back(refusal(&e));
        }
        self.pump()
    }

    fn send<M: Message>(&mut self, message: M) -> Result<(), String> {
        self.app()?.world_mut().write_message(message);
        self.pump()
    }
}
