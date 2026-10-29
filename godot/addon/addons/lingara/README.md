# Lingara for Godot (GDScript)

**GDScript projects use this addon. C# projects use
[`Lingara.Embed`](https://www.nuget.org/packages/Lingara.Embed) from NuGet**,
which is typed C# over the same core: see its Godot C# quickstart. A project
that loads both runs two cores and two connections for one player.

Lingara inside a Godot game: NPCs that answer in the language the player is
learning, at the player's level, and practice sessions whose results reach
your server. The addon is one GDExtension node, `LingaraEmbed`, added as the
`Lingara` autoload. Every network call runs on the core's own thread; the
node drains the core's events once per frame and emits one signal per event,
so a stalled network never delays a frame.

## Install

1. Install **Lingara** from the Asset Library (or copy `addons/lingara/` from
   the release zip into your project).
2. Enable it: **Project → Project Settings → Plugins → Lingara**. That adds
   the `Lingara` autoload and the `lingara/*` project settings.
3. Set **lingara/token_url** to your server's token route (below), or leave it
   empty and answer `token_needed` yourself.

**macOS: a zip unzipped by hand may be quarantined.** The macOS library
carries only an ad-hoc signature, not a Developer ID, so when a browser
downloads the release zip and Finder unzips it, Gatekeeper refuses to load
`liblingara_godot` ("Apple cannot check it for malicious software"). Installing
from the Asset Library, or through the AssetLib tab's **Import…** with the
zip itself, avoids it. If you copied the folder by hand, clear the flag and
restart the editor:

```sh
xattr -dr com.apple.quarantine addons/lingara
```

**A working sample** is in the repository, not in this addon:
[`godot/demo`](https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/tree/main/godot/demo),
a night market with two NPCs at two levels and a practice session. It runs
beside the repository's demo token server, so it needs a clone, a Rust
toolchain and a metered client; its README has the steps.

Tested on Godot **4.5** and **4.7**. The floor is 4.5: the library is built
against the 4.5 API, and GDExtension loads it on every later 4.x.

## Project settings

| Setting | Default | |
|---|---|---|
| `lingara/base_url` | `https://api.getlingara.com` | |
| `lingara/token_url` | empty | set: the core `POST`s `{}` there for each token; empty: `token_needed` |
| `lingara/api_version` | empty | a `Lingara-Version` pin |
| `lingara/autostart` | `true` | false: call `Lingara.start()` yourself, after your own sign-in |
| `lingara/stream_on_start` | `true` | false: a game that never listens pays for no stream |

The node's properties take these as defaults; a value set on the node, in a
scene or in code before `start()`, wins. `queue_capacity`, `log_level`,
`user_agent_suffix`, `max_events_per_frame` (256), `forward_logs` and
`extra_config` (`event_types`, `resume_cursor`, `token_wait_timeout_ms`,
`stream_idle_timeout_ms`) are node properties only.

**Anything in ProjectSettings ships inside the exported game.** Never put a
secret or a session header there.

## Tokens

The game never holds a client secret. Your own server mints a player's embed
token with your server kit and returns exactly
`{"token": "lgr_et_…", "expires_at": "…"}`. Mint it **without an `origin`**:
a native game sends none, so an origin-bound token is refused on every call.

Two ways to get it to the addon:

- **`token_url`.** The core `POST`s `{}` to your route on its own thread and
  reads the two keys; no GDScript runs for a refresh. If the route needs the
  player's session, call `Lingara.set_token_endpoint(url, {"cookie": …})` at
  run time: before `start()` it is kept in memory for it (set
  `lingara/autostart` false and call `start()` from your bootstrap), after it
  re-points the live core.
- **The signal round trip.** With `token_url` empty, `token_needed(request,
  reason)` fires; fetch the body however your game logs in and call
  `Lingara.provide_token(body.token, body.expires_at)`, or
  `token_unavailable(reason)` if you cannot. A server that forwards the whole
  mint answer may call `provide_token_for(token, expires_in)` instead.

A token is never stored in a property, a scene or a log. **The scopes each
feature needs**, chosen at the mint:

| Feature | Scopes |
|---|---|
| The stream | `events:read` |
| `send_context` | `events:write`, plus `lesson_plans:write` with `generate` |
| Practice from a plan id | `lesson_plans:read` |
| Practice from a request | `events:write`, `lesson_plans:write`, `lesson_plans:read`; `events:read` optional |
| Practice from `create` | `lesson_plans:write` (and `lesson_plans:read`) |
| The practice report | `events:write`, `embed:play` |
| Dialogue | `embed:play` |

## NPCs

Add a `LingaraNpc` child per character and set `npc_name`, `target_lang`,
`source_lang` and `level` (1–9) in the inspector; `persona` and `scene` are
optional. It opens its dialogue once the core is running.

```gdscript
@onready var mei: LingaraNpc = $Mei

func _ready() -> void:
	mei.thinking.connect(func(): bubble.text = "…")
	mei.fragment.connect(func(text): bubble.text += text)
	mei.replied.connect(func(text): bubble.text = text)

func talk(line: String) -> void:
	if not mei.say(line):
		pass  # busy: a turn is in flight, and turns are never queued (each is billed)
```

Setting `level` or `scene` at run time keeps the conversation. The core keeps
the last 12 lines; to carry a conversation across a reload, save
`Lingara.dialogue_snapshot(mei.dialogue_id()).window` and set
`initial_window` before the NPC opens.

## Practice

```gdscript
var session := Lingara.practice_start({"type": "request", "topic": "buying food",
		"source_lang": "en", "target_lang": "zh", "level": 3})
```

Each snapshot arrives as `practice_changed(session, snapshot)`; answer with
`practice_choose(session, option)`, `practice_answer(session, text)`,
`practice_reveal`, `practice_self_grade(session, correct)` and
`practice_next`. Each of those also returns the new snapshot.
`practice_finished` fires once, when the session is `reported`, `ended` or
`failed`. **A demo plan must be practised with `report: false`**
(`{"type": "plan", "plan_id": …, "report": false}`): Lingara refuses a report
on one.

## Signals

Every payload is a `Dictionary` in the core's own JSON (the keys in
`lingara-embed`'s README). Each event first arrives whole on
`core_event(event)`, then on its own signal; an event newer than this addon
arrives on `core_event` alone.

| Signal | Arguments |
|---|---|
| `started` | — |
| `token_needed` | `request: int`, `reason: String` (`initial` · `expiring` · `rejected`) |
| `stream_changed` | `state: Dictionary` (`type`: `connecting` · `live` · `paused` · `down`), `cursor: String` |
| `event_received` | `id`, `event_type`, `created_at: String`, `data: Dictionary` |
| `world_sent` | `request: int`, `event_id: String`, `reaction: Dictionary` |
| `practice_changed` / `practice_finished` | `session: int`, `snapshot: Dictionary` |
| `npc_turn_opened` | `dialogue: int`, `request: int` |
| `npc_fragment` / `npc_replied` | `dialogue: int`, `text: String` |
| `npc_notice` | `dialogue: int`, `code: String`, `message: String` |
| `npc_turn_cancelled` | `dialogue: int` |
| `error_raised` | `error: Dictionary` (`kind`, `message`, `status`, `code`, `retry_after_ms`, `retryable`, `request`) |
| `log_received` | `level`, `target`, `message: String` |
| `closed` | `reason: String` (`requested` · `internal`) |

**Refusals.** A method the core refuses returns `-1`, `false` or `{}`, emits
`error_raised` before it returns, and leaves the error in `last_error()`:

```gdscript
var turn := Lingara.dialogue_say(npc_id, line)
if turn < 0:
	print(Lingara.last_error().message)
```

With `forward_logs` on, the core's warnings and errors also reach the
editor's output.

## Platforms

| Platform | Notes |
|---|---|
| Windows x64 | — |
| macOS 11+ (universal) | A notarised export must sign the framework: Godot's macOS export signs embedded frameworks when code signing is configured |
| Linux x64 | glibc 2.28 or newer |
| Android arm64 (API 23+) | Enable the **INTERNET** permission in the export preset, or every call fails. No x86_64 build, so not on an x86_64 emulator |
| iOS arm64 (13.0+) | gdext's iOS support is **experimental** upstream and this platform is verified on a device by hand, not in CI. Exports go through Xcode, which embeds and signs the framework. No simulator slice |

**Web exports are not supported.** Exclude `addons/lingara/` from your web
preset, keep your calls behind `ClassDB.class_exists("LingaraEmbed")`, and
reach the autoload with `get_node_or_null("/root/Lingara")` rather than the
global name.

## Licence

MIT. See `LICENSE`.
