# Changelog

## v0.1.0-alpha.6 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Fixed

- The repository now ships its `.gitattributes`, so a Windows checkout gets LF line endings and the C header and JSON schema freshness tests pass there; under v0.1.0-alpha.5 they reported `include/lingara_embed.h` and `schema/messages.schema.json` as stale on Windows. The packages themselves are unchanged.

## v0.1.0-alpha.5 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Fixed

- The first release to reach the registries. v0.1.0-alpha.1 through v0.1.0-alpha.4 stopped in their release builds and were never published (alpha.4's Bevy leg ran out of disk); this version carries the same fifteen packages, changed only in their version.

## v0.1.0-alpha.4 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Fixed

- The first release to reach the registries. v0.1.0-alpha.1, v0.1.0-alpha.2 and v0.1.0-alpha.3 stopped in their release builds and were never published; this version carries the same fifteen packages, changed only in their version.

## v0.1.0-alpha.3 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Fixed

- Never published: like v0.1.0-alpha.1 and v0.1.0-alpha.2, its release build stopped before the registries. It carries the same fifteen packages, changed only in their version.

## v0.1.0-alpha.2 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Fixed

- Never published: like v0.1.0-alpha.1, its release build stopped before the registries. It carries the same fifteen packages, changed only in their version.

## v0.1.0-alpha.1 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Added

The first release: fifteen packages, every one a pre-release, at one version. Each line names the package's release id, then where it is published.

- `core`: the crate `lingara-embed` on crates.io, the player core and, behind its `server` feature, the Rust server kit.
- `bevy`: the crate `bevy_lingara` on crates.io.
- `dotnet`: `Lingara.Embed` on NuGet, with its five native libraries.
- `dotnetserver`: `Lingara.Embed.Server` on NuGet.
- `typescript`: `@lingara/embed` on npm, under the `next` tag; the CDN build is the same package on jsDelivr.
- `kotlin`: `com.getlingara:lingara-embed-kotlin` on Maven Central, the JVM player kit for Kotlin, libGDX and Android.
- `java`: `com.getlingara:lingara-embed-java` on Maven Central, the Java facade over it.
- `nativesdesktop`: `com.getlingara:lingara-embed-natives-desktop` on Maven Central, the Windows, macOS and Linux libraries where JNA looks for them.
- `nativesandroid`: `com.getlingara:lingara-embed-natives-android` on Maven Central, the arm64 library as an AAR.
- `kotlinserver`: `com.getlingara:lingara-embed-server-kotlin` on Maven Central.
- `javaserver`: `com.getlingara:lingara-embed-server-java` on Maven Central.
- `go`: the module `github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go`.
- `ruby`: the gem `lingara-embed` on RubyGems.
- `php`: `spinningcatstudios/lingara-embed` on Packagist.
- `godot`: the GDScript addon on the Godot Asset Library, downloaded from this release's `lingara-embed-godot-<version>.zip`. The Unreal plugin is not released yet.


- `@lingara/embed`: the `<lingara-embed>` element. One tag and a `tokenProvider` frame Lingara practice in a website, over protocol v1 of the hosted practice page; no runtime dependencies, SSR-safe, and a script-tag build with its SRI value.
- The C ABI (`lingara_embed.h`): nine functions that move the core's own JSON through caller-owned buffers, for Windows x64, macOS (universal), Linux x64 (glibc 2.28), Android arm64 and iOS arm64. Each platform's libraries and the header ship as a release asset for C and C++ callers.
- `Lingara.Embed` on NuGet: the player half of C# for Unity 2022.3+ (Mono and IL2CPP) and Godot 4.4+ C#. It has no dependencies and uses no reflection. It ships the five native libraries, and the Unity and Godot snippets show the token, poll, dialogue and practice flow.
- The server kits, in eight languages: `@lingara/embed/server`, `lingara-embed`'s `server` feature, the Go module, `com.getlingara:lingara-embed-server-java` and `-server-kotlin`, the `lingara-embed` gem, `spinningcatstudios/lingara-embed` and `Lingara.Embed.Server`. Each mints a player's token, hands it to the player as exactly `{token, expires_at}`, renders the widget snippet, verifies a webhook and routes it to the player it concerns through a `PlayerDirectory` you implement over your own store, and deletes a player. Seven sit on their language's `lingara` library at one floor (`LIBRARY_FLOOR`, 0.2.0-alpha.1); `Lingara.Embed.Server` (.NET 8) carries its own minimal client and webhook verifier and has no dependencies.
- The ES conformance contract: seven behaviours, 21 cases run against every server kit, and the `snippet.json` and `token-handoff.json` vectors.
- `bevy_lingara` for Bevy 0.20: a plugin over `lingara-embed` that drains its event queue once per frame and never blocks it. An NPC the player talks to is an `NpcDialogue` component with an `NpcTranscript` beside it; a practice session is a `PracticeSession` component; tokens, world context and every core event are Bevy Messages. It adds only Bevy's ECS to a game's build, and passes every EK case.
- The Bevy demo: two NPCs in two languages at two levels, and a practice session over a plan Lingara makes, with a headless test.
- The Godot addon for GDScript (Godot 4.5 and later): one `LingaraEmbed` node, added as the `Lingara` autoload, that drains `lingara-embed`'s event queue once per frame and emits one signal per event, with methods that take and return `Dictionary`s in the core's own JSON. `LingaraNpc` is a talking character in one node. Windows x64, macOS (universal), Linux x64, Android arm64 and iOS arm64 (experimental). It passes every EK case under Godot 4.5 and 4.7. C# projects use `Lingara.Embed` instead.
- The Godot demo: two NPCs in two languages at two levels, typed lines, and a practice session over a plan Lingara makes, with a headless test.
- The Unreal plugin `LingaraEmbed`, source for Unreal Engine 5.6, 5.7 and 5.8: a game instance subsystem that drains the core's queue once per tick and broadcasts one Blueprint delegate per event, one Blueprint node per core command (with `Ok` and `Refused` exec pins), the **Lingara Say** and **Fetch Token From URL** async nodes, and Project Settings that hold no credential. Win64, Mac, Linux, Android arm64 and iOS. Not yet tested in the engine: its engine-free half (the C++ wrapper over the C ABI) passes every EK case.
- `lingara_embed.hpp`: an engine-free C++17 wrapper over the C ABI, header-only, with no exceptions or RTTI, for C++ callers outside Unreal too.
- The demo token server: a loopback stand-in for a game's own token route, for the demos, with a `--fake` mode that needs no credentials.

### Changed

### Fixed

### Removed
