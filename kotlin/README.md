# lingara-embed-kotlin

Lingara inside a JVM game: libGDX on the desktop or Android, or any Kotlin
game with a frame loop. The kit starts Lingara's native core once, and the
game drains its events once a frame on the game thread. Tokens, the event
stream, practice sessions and NPC dialogue behave exactly as they do in the
Bevy, Godot and Unity kits, because one conformance suite holds them all.

## Install

Two lines: the kit, and the natives for the platform. In a libGDX project the
desktop natives go in the `lwjgl3` module and the Android natives in the
`android` module, where libGDX's own natives already sit.

The kit is published to Lingara's own Maven repository,
`https://maven.getlingara.com`, so the build names it beside the dependencies
(in `settings.gradle.kts`, or each module's `repositories`). The versions
published before the move stay on Maven Central too; new ones are only here.

```kotlin
repositories {
    exclusiveContent {
        forRepository { maven("https://maven.getlingara.com") }
        filter { includeGroup("com.getlingara") }
    }
    mavenCentral()
}
```

```kotlin
// core
implementation("com.getlingara:lingara-embed-kotlin:<version>")
// lwjgl3 (Windows x64, macOS arm64 and x64, Linux x64 with glibc 2.28+)
implementation("com.getlingara:lingara-embed-natives-desktop:<version>")
// android (arm64-v8a, API 23+)
implementation("com.getlingara:lingara-embed-natives-android:<version>")
```

The natives line is `implementation`, not `runtimeOnly`: it also brings JNA,
in the variant the platform needs, and the kit's types refer to it. All four
artefacts share one version. JDK 17 or newer; Kotlin 2.2 or newer reads it.

## The game loop

```kotlin
val lingara = LingaraEmbed.start(
    EmbedConfig(
        tokenSource = TokenSourceConfig.Http(
            url = "https://my-game.example/lingara/token",
            headers = listOf(Header("Authorization", Secret("Bearer $session"))),
        ),
        userAgentSuffix = "my-game/1.4",
    ),
)

// render(): one native call, at most 256 events, in queue order
lingara.drain(256) { event ->
    when (event) {
        is EmbedEvent.Lingara -> world.onLingara(event.event)
        is EmbedEvent.Practice -> practiceUi.show(event.session, event.snapshot)
        is EmbedEvent.NpcTurn -> npcBubble.apply(event.dialogue, event.update)
        is EmbedEvent.Error -> Gdx.app.error("lingara", event.error.message)
        else -> Unit
    }
}

// dispose()
lingara.close()
```

- Every method may be called from any thread; drain on one, the game thread.
  The core does all I/O on its own thread, so Android's `StrictMode` never
  sees network on the main thread.
- `practiceAnswer`, `practiceChoose` and the other practice calls return the
  graded snapshot in the same call, so the answer shows this frame.
- `close()` waits at most a second for the core's thread and frees it.
  `closeAndWait(ms)` says whether it joined in time; `closeNow()` never waits.
- `EmbedEvent.Unknown` is an event from a newer core: its `json` holds it.

## Tokens

The game's server mints the embed token (a server kit does it in a few lines);
the player kit only ever holds that token, never a client secret.

- **Http source** (recommended): the core POSTs `{}` to your endpoint with the
  headers you give it, and reads `{token, expires_at}`. After the player
  signs in, `setTokenEndpoint(url, headers)` swaps the headers.
- **`TokenFetcher`**: for a session that lives in JVM code (an OkHttp cookie
  jar, an auth SDK). It runs on the kit's own daemon thread, never the game
  thread; return `IssuedToken.expiringAt(Secret(token), expiresAt)`, or throw.
- **Neither**: `EmbedEvent.TokenNeeded` reaches your handler, and you answer
  with `provideToken`.

Every token and header value is a `Secret`, whose `toString()` is
`[REDACTED]`, so a config, a header or a command can be logged whole.

## Android

- **Lifecycle.** Call `streamPause()` in libGDX's `pause()` and
  `streamResume()` in `resume()`, so a backgrounded game holds no connection.
- **R8.** `lingara-embed-natives-android` carries the keep rules JNA needs, so
  a minified build needs nothing of yours.
- **Emulators.** The kit ships arm64-v8a only: use an arm64 image (every Apple
  silicon emulator is one). An x86_64 emulator gets
  `LingaraNativesMissingException`.

`LingaraNativesMissingException` also names the artefact to add when the
natives are missing on the desktop, or are of another version than the kit.

## A Flow, if you want one

The kit has no coroutines, so a frame-loop game pays for none. An Android app
that wants a `Flow` can wrap `drain` in a ticker:

```kotlin
fun LingaraEmbed.events(periodMs: Long = 16): Flow<EmbedEvent> = flow {
    while (true) { drain().forEach { emit(it) }; delay(periodMs) }
}
```

## The example

[`examples/libgdx/`](examples/libgdx/) is a minimal libGDX game with desktop
and Android launchers, built against this repository's own code.

## The public package

The generated classes (`EmbedEvent`, `PracticeSnapshot`, `EmbedConfig` and the
rest) are the kit's types. `EmbedHandle` and UniFFI's own helpers
(`FfiConverter*`, `RustBuffer`) share the package; they are UniFFI's, not
part of this kit's promise.
