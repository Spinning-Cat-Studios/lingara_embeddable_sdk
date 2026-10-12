# Lingara.Embed

Lingara inside a Unity or Godot C# game: practice, NPC dialogue and the world
event stream, polled from your game loop. This is the **player half**. It runs
in the game, holds no secret, and never mints a token. The server half, which
mints tokens and verifies webhooks, is the separate package
`Lingara.Embed.Server`.

One native core (`lingara-embed`) does the networking on its own thread. This
package moves its JSON through a small C ABI. It has no dependencies and uses
no reflection, so IL2CPP and NativeAOT compile it unchanged.

## Install

**Unity 2022.3 LTS or later**: install [NuGetForUnity](https://github.com/GlitchEnzo/NuGetForUnity),
then search for `Lingara.Embed`. NuGetForUnity places each platform's native
library and sets its import settings.

**Godot 4.4 or later, C# (.NET 8)**: add the package to your game's `.csproj`:

```xml
<PackageReference Include="Lingara.Embed" Version="x.y.z" />
```

Godot desktop exports carry the matching native library. Godot's C# Android
and iOS exports are experimental in Godot itself, so this package supports
them on a best-effort basis.

**GDScript or C#? Pick one.** A GDScript project uses the Godot addon (K4).
A C# project uses this package. A project that loads both runs two cores and
two connections for one player.

## The loop

```csharp
using Lingara.Embed;
using Lingara.Embed.Events;

var client = LingaraEmbedClient.Create(new EmbedOptions { UserAgentSuffix = "my-game/1.4" });
var events = new List<EmbedEvent>();

// once a frame, on the main thread
events.Clear();
client.Drain(events);              // at most 256 a frame; the rest wait for the next
foreach (var e in events)
{
    switch (e)
    {
        case EmbedEvent.TokenNeeded need: /* ask your server, then ProvideToken */ break;
        case EmbedEvent.NpcTurn turn: /* append turn.Update's text */ break;
        case EmbedEvent.Failed failed: /* failed.Error.Request names the command */ break;
        case EmbedEvent.Log log: /* route to your engine's log */ break;
    }
}

// at quit
client.Dispose();
```

Create **one client per player**. Each client owns its own I/O thread and
connection, so a split-screen game creates one per local player and no more.

## Tokens

The core asks for a token with a `TokenNeeded` event. It asks once at start,
again before the token expires, and again if the server rejects it. Your game
asks **your** server, which mints an embed token with `Lingara.Embed.Server`.
Then the game hands it over:

```csharp
client.ProvideToken(handoff.token, handoff.expires_at);   // or ProvideTokenFor(token, expiresIn)
client.TokenUnavailable("offline");                       // your server call failed: fail the waiters now
```

- **Mint native tokens without `origin`.** A native game sends no origin
  header, and an origin-bound token is refused.
- **Never put a client secret in a game.** Anything in a game binary can be
  extracted. Mint on your server and hand the game only the short-lived
  `lgr_et_` token.
- **Or let the core fetch tokens.** Set `EmbedOptions.TokenEndpoint` to your
  server's URL and the core POSTs `{}` there and reads the handoff body
  itself. `SetTokenEndpoint` re-points it mid-session, for example after a
  player signs in.

## What it can do

| Area | Methods | Answers with |
|---|---|---|
| Stream | `StreamPause`, `StreamResume`, `Status.Cursor` | `StreamChanged`, `LingaraReceived` |
| World | `SendContext` | `WorldSent` |
| Practice | `PracticeStart`, `PracticeAnswer`, `PracticeChoose`, `PracticeReveal`, `PracticeSelfGrade`, `PracticeNext`, `PracticeFinish`, `GetPracticeSnapshot` | the new snapshot, in the call; `Practice` for I/O |
| Dialogue | `DialogueOpen`, `DialogueSay`, `DialogueSetLevel`, `DialogueSetScene`, `DialogueCancel`, `DialogueClose`, `GetDialogueSnapshot` | `NpcTurn` (`Opened`, `Fragment`, `Finished`) |

A demo plan must be started with `new PracticeSource.Plan { PlanId = …, Report = false }`,
so playing it reports nothing.

## Errors

A command the core refuses throws `EmbedException`. Its `Error` is the core's
error: `Kind`, `Message`, `Retryable`, and `Status` and `Code` for a server
refusal. A failure that arrives later is never thrown. It is an
`EmbedEvent.Failed` in the queue, and its `Error.Request` names the command
it answers.

## Platforms

Windows x64, macOS (universal), Linux x64 (glibc 2.28 or later), Android
arm64 (API 23 or later) and iOS arm64 (13.0 or later, statically linked). On
iOS, set `LingaraEmbedClient.NativeMode = NativeLibraryMode.StaticInternal`
before the first `Create`. The Unity component in the docs does this for you.
