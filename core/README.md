# lingara-embed

Lingara inside a game: the player-side core that every engine kit wraps.
It talks to Lingara on a thread of its own and gives the engine only plain
data. Each frame the engine drains a queue of typed events; it answers with
plain commands. Nothing crosses as a callback, a future or a generic.

The engine kits (Bevy, Unity, Godot, Unreal, the JVM) are built on this
crate. A Rust game may also use it directly.

## The poll loop

```rust,no_run
use lingara_embed::{Embed, EmbedConfig, EmbedEvent, TokenSourceConfig};

let embed = Embed::start(EmbedConfig {
    token_source: TokenSourceConfig::Http {
        url: "https://my-game.example/lingara/token".into(),
        headers: vec![],
    },
    user_agent_suffix: Some("my-game/1.4".into()),
    ..Default::default()
})?;

// Every frame:
for event in embed.drain(256) {
    match event {
        EmbedEvent::Lingara { event } => println!("{} {}", event.event_type, event.data_json),
        EmbedEvent::Practice { session, snapshot } => println!("session {session}: {:?}", snapshot.phase),
        EmbedEvent::NpcTurn { update, .. } => println!("{update:?}"),
        EmbedEvent::Error { error } => eprintln!("{error}"),
        _ => {}
    }
}
# Ok::<(), lingara_embed::EmbedError>(())
```

No method blocks or awaits. Create **one** `Embed` per game: each owns one
OS thread. `close()` returns at once; `close_and_wait(1000)` also joins the
thread, so a host can unload the library after it. Dropping the handle is
`close()`. The queue is bounded (`queue_capacity`, 1024): a game that stops
polling loses nothing, because the stream stops reading and the server
holds the backlog. 256 events per frame is a good cap.

## Tokens

A game holds only an **embed token** (`lgr_et_…`), minted by your server for
one player and valid for 15 minutes. Anything else is refused locally,
whichever source it came from: a client-credentials token belongs on your
server, never in a game binary.

Three sources:

- **Poll** (the default): the core raises `TokenNeeded { request, reason }`
  ahead of the queue and waits for your answer:

  ```rust,no_run
  # use lingara_embed::{Embed, EmbedConfig, EmbedEvent};
  # let embed = Embed::start(EmbedConfig::default())?;
  # let (token, expires_at) = (String::new(), String::new());
  if let Some(EmbedEvent::TokenNeeded { .. }) = embed.poll_event() {
      // ask your server, then:
      embed.provide_token(token, expires_at)?;
  }
  # Ok::<(), lingara_embed::EmbedError>(())
  ```

  `provide_token_for(token, expires_in_secs)` takes the lifetime from the
  mint's `expires_in`, for a host that holds the whole mint answer.
- **Http**: the core `POST`s `{}` to your URL with your headers (your game
  session's cookie or bearer) and reads `{"token": "lgr_et_…", "expires_at":
  "…"}`. Any other field is ignored, so an endpoint that forwards the whole
  mint answer works too. `set_token_endpoint` re-points it, for a player who
  signs in mid-session. Header values are secrets: no rendering shows them.
- **A Rust trait**: `Embed::start_with_source(config, Arc<dyn EmbedTokenSource>)`.

A token is asked for again at `lifetime − min(60 s, lifetime / 2)`: 840 s for
a 900 s token. A device clock can be wrong by hours, so a lifetime outside
(0 s, 1 h] is taken as 300 s, with a `Log` warning. After a `401` the source
is asked once more (`reason: rejected`) and the call retried once.

**Mint native tokens without `origin`.** A token minted with an origin is
the browser widget's; a native kit never sends the origin header, so such a
token is refused on every call.

**The scopes each feature needs** (chosen at the mint):

| Feature | Scopes |
|---|---|
| The stream | `events:read` |
| `send_context` | `events:write`, plus `lesson_plans:write` with `generate` |
| Practice from a plan id | `lesson_plans:read` |
| Practice from a request | `events:write`, `lesson_plans:write`, `lesson_plans:read`; `events:read` optional |
| Practice from `Create` | `lesson_plans:write` (and `lesson_plans:read`) |
| The practice report | `events:write`, `embed:play` |
| Dialogue | `embed:play` |

A missing scope is the server's `403`, surfaced as `Forbidden`. A game whose
tokens lack `events:read` sets `stream_on_start: false`; otherwise its stream
goes `Down` at every new token.

## The stream

The stream resumes from its cursor after a drop (`Last-Event-ID`), treats
45 s without a byte (three missed heartbeats) as a drop, and delivers each
event id once. Save `status().cursor` and pass it back as `resume_cursor` to
resume across a restart. `stream_pause()` holds no connection, for a game in
the background. Every event type reaches you as `Lingara { event }` with its
raw `data_json`; the two plan events are also typed in `event.detail`.
`stream_idle_timeout_ms` applies to every stream, NPC turns included: a
short value cuts a slow reply.

## The world

`send_context(WorldContext { scene, … })` tells Lingara where the player
is. Each send spends one of the player's inbound cells (60 an hour, 300 a
day), so send on a scene change, never every frame. The bounds are in
`lingara_embed::limits`.

## Practice

```rust,no_run
# use lingara_embed::{Embed, EmbedConfig, PracticeSource};
# let embed = Embed::start(EmbedConfig::default())?;
let session = embed.practice_start(PracticeSource::Plan { plan_id: "…".into(), report: true })?;
// …the plan arrives as a Practice event; then, in the same frame:
let snapshot = embed.practice_answer(session, "ni hao")?;
# Ok::<(), lingara_embed::EmbedError>(())
```

- **A demo plan must be started with `report: false`.** A guest may read
  and practise demo plans, but Lingara refuses a result on one, and the plan
  carries no flag the core could read.
- A question with `options` is answered by index (`practice_choose`).
  `free_response`, `phrase_translation` and `conversational_response` are
  graded by the learner (`practice_self_grade`) after seeing the model
  answer. Anything else is a typed answer, graded after folding case,
  whitespace and punctuation.
- **One report per session**: exactly one `world.practice_completed`, when
  the last set completes or at `practice_finish`, under one idempotency key.
  Each report spends one inbound cell. **Call `practice_finish` when the game
  quits** mid-session: a game that is killed loses that session's report.

## Dialogue

`dialogue_open` holds an NPC; `dialogue_say` sends one line and streams the
reply as `NpcTurn` updates: `Opened`, `Fragment`, `Notice`, then `Finished`
with the whole reply. The core keeps the last 12 lines and nothing else;
pass `dialogue_snapshot().window` to the next `dialogue_open` to carry a
conversation across a restart. Each turn spends one of the player's NPC
cells (20 an hour, 50 a day) and is billed to you, so **the core never
retries a turn**: say the line again if you want one.

## Errors

Every failure is an `EmbedError` with one closed `ErrorKind` and a
`retryable` flag that depends on the kind alone. `message` is English, for
logs. Diagnostics go through the `log` facade (no logger is installed) and,
at or above `log_level`, as `Log` events for an engine with no Rust logger.

Every public method validates its input and returns an error rather than
panicking; a panic on the I/O thread closes the handle with
`Closed { reason: internal }`. Under `panic = "abort"` nothing can be caught.

## Platforms

Windows x64, macOS (arm64 and x86_64), Linux x64 (glibc 2.28), Android arm64
(API 23) and iOS arm64 (13.0). TLS is `rustls` with bundled roots on every
target; the `native-tls` feature is for desktop games that want the OS
store. No wasm32: in a browser, use the widget. MSRV: Rust 1.87.

Features: `rustls` (default), `native-tls`, `schema` (derives
`schemars::JsonSchema` on every public type, for a binding's message schema),
`server` (below; adds no dependency, and a game never needs it).

## Server feature

Your game's or site's server vouches for each player. `--features server`
adds `lingara_embed::server`, four jobs over the `lingara` client you
already hold:

```rust,ignore
use std::sync::Arc;
use lingara_embed::server::*;

// Lost on restart and not shared between instances: implement
// PlayerDirectory over your own database in production.
let players = Arc::new(MemoryPlayerDirectory::new());
let embed = EmbedServer::new(client).with_directory(players.clone());

// Your token route (rate-limit it per session): mint, then hand off.
let minted = embed.mint_token(&player_ref, MintOptions::default()).await?;
let body = token_handoff(&minted); // {"token":"lgr_et_…","expires_at":"…"}

// Your webhook route, on the raw body: 204, 401, 400 or 500.
let router = PlayerRouter::new(lingara::events::Webhook::with_secrets(secrets)?, players);
let status = router.handle(&raw_body, &headers, |routed| async move { Ok(()) }).await;

// Your account-deletion path.
embed.delete_player(&player_ref).await?;
```

- **`mint_token`** makes one `POST` per call: there is no cache, so a token
  `delete_player` revoked is never handed out again. It remembers the
  player's `subject` in your directory after a success only. Pass
  `MintOptions { origin }` for a web page; a game's server mints without one.
- **`token_handoff`** is exactly `{"token","expires_at"}`, the body every
  player kit's token source and the widget's `tokenProvider` read.
- **`PlayerDirectory`** is the one thing a kit cannot derive: Lingara's
  `subject` is pairwise. Keep a table of `player_ref` and `subject`, unique
  on `subject` (a player who links an account owns a second one).
- **`PlayerRouter::handle`** verifies first, so a forged event never reaches
  your directory. Lingara redelivers every non-`2xx`: deduplicate on the
  event id, and enqueue slow work, since delivery expects a `2xx` within 10 s.
- **`widget_snippet`** renders `<lingara-embed>` and its two module scripts.
  Pass your page's CSP `nonce` if its `script-src` allows no inline script.
  The token provider uses `credentials: "same-origin"`, so a token endpoint
  on another origin gets no cookies and must authenticate some other way.

The whole examples, on axum, are `snippets/server/rust/` in this repository.
