# bevy_lingara

Lingara inside a Bevy game: a plugin over [`lingara-embed`](https://crates.io/crates/lingara-embed).
An NPC the player talks to in the language they are learning is a component
on an entity; so is a practice session. The plugin drains the core's event
queue once per frame and never blocks it: every network call runs on the
core's own thread, never on Bevy's.

```sh
cargo add bevy_lingara
```

## Bevy versions

Each release supports exactly one Bevy minor. When Bevy ships a new minor,
a release that moves to it follows within 30 days.

| bevy_lingara | Bevy |
|---|---|
| 0.0 | 0.20 |

## Adding the plugin

```rust,no_run
use bevy::prelude::*;
use bevy_lingara::{LingaraConfig, LingaraPlugin};

App::new()
    .add_plugins(DefaultPlugins)
    .add_plugins(LingaraPlugin::new(
        LingaraConfig::new()
            .token_endpoint("https://game.example.com/lingara-token", vec![])
            .source_lang("en"),
    ))
    .run();
```

The plugin's systems run in three public sets: `LingaraSystems::Drain` and
`LingaraSystems::Apply` in `PreUpdate`, so your `Update` systems see this
frame's state, and `LingaraSystems::Send` in `PostUpdate`, which hands this
frame's Messages to the core. Order your own systems against them with
`.after(LingaraSystems::Apply)`. Read `LingaraStatus` for the handle's phase
and the stream's state; keep its `embed.cursor` and pass it back with
`LingaraConfig::resume_cursor` to resume the stream with no gap after a
restart.

With `autostart(false)` nothing starts until you send `LingaraConnect`, for
example after your own sign-in. `LingaraDisconnect` closes the handle, and
the `Lingara` resource goes once the core's `Closed` has been drained.
Quitting never waits on the network.

## Tokens

The game never holds a client secret. Your own server mints a player's embed
token and returns exactly `{"token": "lgr_et_…", "expires_at": "…"}` (the
handoff body your server kit's `token_handoff` writes). Mint it **without an
`origin`**: a native game sends none, so an origin-bound token is refused on
every call. A player token can hold `events:read`, `events:write`,
`lesson_plans:read`, `lesson_plans:write` and `embed:play`.

Three ways to get it to the plugin:

| `LingaraConfig` | For |
|---|---|
| `.token_endpoint(url, headers)` | The common case. The core `POST`s to your route with your game's own session headers, off the frame. A player who signs in mid-session: send `SetTokenEndpoint` with the new URL and headers |
| `.token_source(Arc<dyn EmbedTokenSource>)` | Your own async token code, run on the core's thread |
| neither | Your session lives in ECS: read `LingaraTokenNeeded`, ask your server, and answer with `ProvideToken::new(token, expires_at)`, or `TokenUnavailable` |

`ProvideToken` has private fields and a redacting `Debug`, and no component
or resource ever holds a token.

## Updates

An update notice is for you, the developer, and is never drawn for the
player: read it and log it, or build your own prompt from it. Set
`LingaraConfig::new().embed_version("1.1.0")` to the embed version this build
was registered against; the token endpoint is sent it as
`Lingara-Embed-Version`. A game answering `LingaraTokenNeeded` itself sends
that header itself, and passes the handoff's notice on with
`ProvideToken::with_update(token, expires_at, UpdateAvailable::from_value(v))`.
Once per handle the plugin writes a `LingaraUpdateAvailable { latest,
deprecated, message }` message, and the core's warning line reaches
`bevy::log`.

## NPC dialogue

```rust,ignore
let mei = NpcDialogue::new("Mei", "zh", 2)?.with_persona("a market seller")?;
commands.spawn(mei);
// later, when the player picks a line:
say.write(NpcSay { entity, line: "How much are the dumplings?".into() });
```

`NpcTranscript` appears beside it: `phase`, the reply as it streams, a
`notice`, and the finished `turns`. `NpcReplied` carries each full reply.
Only `level` and `scene` change on a live NPC (`set_level`, `set_scene`); a
different persona or language is a new `NpcDialogue`.

One turn is in flight per NPC: a line said while one is streaming is refused
with `Busy` as a `LingaraError`, and nothing is queued. **Turns are billed and
never retried**, by the plugin or the core; each spends one of the player's
NPC turns (20 an hour, 50 a day). `NpcCancel` drops a reply in flight, and it
is still billed. To carry a conversation across a restart, read
`Lingara::core()`'s `dialogue_snapshot(id).window` and insert the NPC again
`with_window`.

## Graph NPCs

A story as a graph (checkpoints joined by edges) is the same component,
opened with `DialogueLoad` instead. Build the dialogue with
`lingara_embed::dialogue_ir::build`, or parse a saved one with
`dialogue_ir::from_json`, and name the cast member this entity is:

```rust,ignore
let story = missing_brother()?; // Dialogue::new(…)…build()
commands.spawn(NpcDialogue::graph(&story, "elena", "es", 2)?);
// the game sets facts; a predicate that reads one may cross an edge:
set.write(NpcSetFact { entity, key: "alarm_raised".into(), value: FactValue::Boolean(true) });
```

`CheckpointReached { entity, checkpoint, edge, via }` follows the load and
every crossing, and `DialogueEnded { entity, outcome }` the end of the story;
`NpcTranscript::checkpoint` holds where it is. `NpcSay`, `with_scene`,
`with_source_lang`, `set_level` and removal (which closes the dialogue) work
as for a free-form NPC.

## Practice

```rust,ignore
start.write(StartPractice {
    entity,
    source: PracticeSource::Request { topic: "ordering food".into(), source_lang: "en".into(), target_lang: "zh".into(), level: 2, tags: vec![] },
});
```

The entity then holds `PracticeSession`, the core's snapshot: the phase, the
question, the last result. Send each learner action as `PracticeInput`; the
new snapshot is in the component the same frame. The core grades, and sends
exactly one result report, which reaches your webhook as
`practice.completed`. `PracticeFinished` is written once, at the end.

A plan your game already holds is `PracticeSource::Plan { plan_id, report }`.
**A demo plan must use `report: false`**: practising one is allowed,
reporting it is refused. Remove the component when the learner leaves: a
session left mid-way is finished, and reports what was done. A game killed
outright loses that session's report.

## Sending context

`WorldContextChanged` tells Lingara where the player is. Each one spends one
of the player's inbound events (60 an hour, 300 a day), so send it on a scene
change, never from a per-frame system.

## Errors

Every failure is a `LingaraError`: the core's `EmbedError` (`kind`, `code`,
`status`, `retryable`, `retry_after_ms`) and the entity it belongs to. One
from a Message you sent this frame arrives the next frame.

## Platforms

Windows x64, macOS, Linux x64, Android arm64 and iOS arm64: exactly what
`lingara-embed` builds for. **`wasm32` is not a target**; a web game uses the
`<lingara-embed>` widget instead.

## Licence

MIT.
