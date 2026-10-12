# Lingara Embed for Unreal Engine

Lingara inside your game: practice sessions and NPC conversation in the
learner's target language, from Blueprints or C++.

**Supported:** Unreal Engine 5.6, 5.7 and 5.8 (the three newest UE5 minors,
never below 5.5), on Win64, Mac, Linux, Android (arm64) and iOS.
**Tested on:** not yet tested in the engine. This line names the engine
versions of the last recorded engine smoke.

The plugin uses no engine API newer than 5.0, so a studio on an older 5.x
can build it from source, but only the versions above are supported.

## Install

Until `1.0.0`, unzip the Release bundle's zip for your engine version
(`LingaraEmbed-<version>-UE5.<m>.zip`) into your project's `Plugins/`
directory, so you have `Plugins/LingaraEmbed/`. From `1.0.0` the plugin is
also on Fab.

Then enable **Lingara Embed** in Edit → Plugins and restart the editor.

## Settings

Project Settings → Plugins → **Lingara Embed**:

| Setting | Default | |
|---|---|---|
| Base Url | empty | empty uses the core's own default |
| Auto Start | off | start when the game instance starts |
| Log Level | Core Default | the core's own logs, into `LogLingaraEmbed` |
| Max Events Per Tick | 256 | a flood carries over to the next frame |

**Never put a client secret in a game.** There is no client id, client secret
or token setting, and there never will be: anything in `DefaultGame.ini`
ships inside your game.

## The token flow

Lingara never sees your game's players directly. Your own server holds the
client credentials, mints a short-lived **native embed token** for a player
(`POST /v1/embed/tokens` **without** an `origin`: native tokens are minted
without one), and hands it to the game as `{ "token", "expires_at" }`.

In the game, bind **On Token Needed** and answer it:

- **Fetch Token From URL** (a Blueprint async node) POSTs `{}` to your
  endpoint with the headers you give it (your game session), reads `token`
  and `expires_at`, and calls **Provide Token**. On any failure it calls
  **Token Unavailable**, so waiting calls fail at once.
- Or call **Set Token Endpoint** once, and the core fetches tokens itself.

While you develop, `tools/demo-token-server` in this repository plays your
server on `http://127.0.0.1:8787/token`.

## Blueprints

Everything is on **Get Game Instance Subsystem → Lingara Embed**:

- **Start** / **Stop**, and one node per core command: **Provide Token**,
  **Send Context**, **Practice Start** … **Practice Finish**, **Dialogue
  Open** … **Get Dialogue Snapshot**. Each has `Ok` and `Refused` exec pins
  and an `Error` out pin; a refusal is never an exception.
- One delegate per event: **On Token Needed**, **On Stream Changed**, **On
  Lingara Event**, **On World Sent**, **On Practice**, **On Npc Turn**, **On
  Error**, **On Closed**, plus **On State Changed** and **On Unknown Event**.
- **Lingara Say** says a line and streams the reply back as **On Fragment**,
  then **On Finished**, or **On Failed**.

The sample under `Content/Samples/` shows all of it in Blueprint.

## C++

```cpp
ULingaraEmbedSubsystem* Lingara = GetGameInstance()->GetSubsystem<ULingaraEmbedSubsystem>();
Lingara->OnNpcTurn.AddDynamic(this, &AMyActor::HandleNpcTurn);
ELingaraCallResult Result;
FLingaraEmbedError Error;
Lingara->Start(FLingaraEmbedConfig(), Result, Error);
```

`snippets/player/unreal/` in the source repository is a complete actor.

### Plain C++, outside Unreal

`lingara_embed.hpp` is an engine-free C++17 wrapper over `lingara_embed.h`.
It needs no engine, no exceptions and no RTTI:

```cpp
#include "lingara_embed.hpp"

lingara_embed::Status status = 0;
auto client = lingara_embed::Client::create(R"({"stream_on_start": false})", status);
std::vector<uint8_t> events(16 * 1024);
std::string_view event;
while (running) {
    while (client.poll(events, event) == LINGARA_EMBED_OK) {
        handle(event);  // K1's JSON: parse it with the library you already use
    }
    sleep_until_next_frame();
}
```

Poll from one thread, and keep a client's calls off other threads while it
is being destroyed.

## Things to know

- **The library is a launch-time dependency everywhere but Win64.** On Win64
  the plugin loads `lingara_embed.dll` itself at startup; if it is missing,
  Lingara is disabled for that run (`On State Changed` reports Disabled) and
  the game carries on. On Mac, Linux and Android a missing library fails the
  game's module load, and on iOS it is linked into the executable.
- **Stop at a loading screen.** Stopping joins the core's network thread and
  can hitch a frame by up to a second.
- **One learner per game instance.** For local split-screen, use
  `lingara_embed::Client` directly, one per player.
- The subsystem lives on the game instance, so a conversation survives a
  level change.
