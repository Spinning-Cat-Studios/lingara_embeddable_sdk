# Changelog

## v0.2.0-alpha.5 — 2026-10-30


Generated from Lingara API 2026-10-expensive-sauger (development) at spec backend@76f880270d80e3a46ad92e74b492a5bbcc5dd9c9.

### Added

### Changed

### Fixed

- Rust: the core's own tests pass again when run from the published crate, and when the whole workspace is tested at once. A test of a stored dialogue read its example from outside the crate, so it could not compile from the packaged `.crate`, and a test of a graph turn checked the order of the keys in its `facts` object, which JSON does not fix. Neither change touches the shipped libraries.

### Removed

## v0.2.0-alpha.4 — 2026-10-30


Generated from Lingara API 2026-10-expensive-sauger (development) at spec backend@76f880270d80e3a46ad92e74b492a5bbcc5dd9c9.

### Added

### Changed

### Fixed

- The core builds from source again. `v0.2.0-alpha.2` and `v0.2.0-alpha.3` shipped without two of `lingara-dialogue-ir`'s source files (the `build` module's `fact` and `parts`), so the crate and every kit built on it failed to compile with `file not found for module`. The files now ship. `lingara_dialogue_ir::build` keeps its name and contents, and no behaviour changes.

### Removed

## v0.2.0-alpha.3 — 2026-10-30


Generated from Lingara API 2026-10-expensive-sauger (development) at spec backend@76f880270d80e3a46ad92e74b492a5bbcc5dd9c9.

### Added

### Changed

### Fixed

- The core builds again from source. `v0.2.0-alpha.2` left out two of `lingara-dialogue-ir`'s source files (`src/build/fact.rs` and `src/build/parts.rs`), which a repository-wide ignore of `build/` directories had hidden, so the crate and every kit built on it failed to compile with `file not found for module`. The files are back, and the ignore now leaves that directory alone. No behaviour changes.

### Removed

## v0.2.0-alpha.2 — 2026-10-30


Generated from Lingara API 2026-10-expensive-sauger (development) at spec backend@76f880270d80e3a46ad92e74b492a5bbcc5dd9c9.

### Added

- Dialogue graphs: every kit can run a `dialogue.v1` story. `dialogue_load` opens a run of a graph (its IR as text), `dialogue_set_fact` sets the facts its predicates read, and `dialogue_validate` checks a document without opening anything. A turn sends the current checkpoint, the edges whose predicates hold (in the order the dialogue lists them) and only the facts the checkpoint may disclose; an edge with no `when` is crossed with no turn. Two new events, `checkpoint_reached` and `dialogue_ended`, say where the story went, and a dialogue's snapshot carries its `graph` so a game can save and restore it. Graphs need `enable_npc_dialogue_graphs` on the client. Rust re-exports the IR crate as `lingara_embed::dialogue_ir`; C#, Godot, Kotlin, Java and Unreal gain the three commands and two events (the idiomatic builders follow).
- Loading a dialogue by id: `dialogue_load` takes `{ type: stored, dialogue_id }`, the store id of a dialogue saved and activated on the Lingara platform. The core fetches the active revision with the player's embed token (which needs `dialogues:read`), caches it per handle for five minutes, and then runs it as an inline graph, so an activated revision reaches new conversations without a game update. The dialogue answers in a new phase, `loading`, until `dialogue_loaded` (with the revision) or `dialogue_load_failed` arrives. C#, Godot, Kotlin, Java, Unreal and Bevy see the new source and the two events.
- NPC fluency: a dialogue's config may carry the NPC's `languages` (each with a speaking and a comprehension level, 1–9) and a `pedagogy` (`subtle` or `none`).

### Changed

- Every kit now needs the `lingara` library at `0.2.0-alpha.6` or later, the release that carries the graph turn and its `transition` frame. In Java, `new DialogueConfig(…)` takes the two new fields (`List.of(), null` for none); Kotlin defaults them.

### Fixed

### Removed

## v0.2.0-alpha.1 — 2026-10-29


Generated from Lingara API 2026-10-expensive-sauger (development) at spec backend@6b556aa7c240143e192e5570f29c1151c4196afd.

### Added

### Changed

### Fixed

- C and C++: the release check on each `lingara-embed-ffi` zip no longer fails a complete zip at random, with a wrong `no LICENSE` (or another file it had found) and a `printf: write error: Broken pipe`. The core's test of an NPC turn's request now checks only the fields the core sets, so it keeps passing when a newer `lingara` adds a defaulted field such as `pedagogy`. Neither change touches the shipped libraries.

### Removed

## v0.1.0-alpha.9 — 2026-10-29


Generated from Lingara API 2026-10-expensive-sauger (development) at spec backend@6b556aa7c240143e192e5570f29c1151c4196afd.

### Added

- `lingara-dialogue-ir`, a new crate on crates.io: the `dialogue.v1` dialogue graph format's Rust types, its load-time checks (each refusal naming its JSON path), and the fact-predicate language an edge's `predicate` is written in, with its grammar shipped as `predicate.ebnf`. It depends only on `serde` and `serde_json`, so a server can validate a dialogue with the same parser the game runs (ADR 28.10.26f, Phase A).

### Changed

### Fixed

### Removed

## v0.1.0-alpha.8 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Added

### Changed

### Fixed

- C and C++: the Linux and Android `liblingara_embed.so` now carries a `DT_SONAME`, so a consumer records the bare `liblingara_embed.so` and finds it through its RUNPATH. Without one, a vcpkg manifest-mode build linked a build-tree-relative path, and the binary failed to load from any other working directory.

### Removed

## v0.1.0-alpha.7 — 2026-10-13


Generated from Lingara API 2026-10-virile-dragon (development) at spec backend@df3362ff852184f65973a1b7bbf159b696125755.

### Added

- C and C++: each release carries a vcpkg overlay port (`lingara-embed-vcpkg-port-<version>.zip`) and a Conan 2 recipe (`lingara-embed-conan-recipe-<version>.zip`) over the prebuilt libraries, and every `lingara-embed-ffi` zip is now a CMake package: `find_package(lingara-embed)` gives `lingara::embed` (C) and `lingara::embed_cpp` (C++17). The ffi zip's files moved under `include/`, `lib/`, `bin/` and `share/lingara-embed/`, beside a `LICENSE`.

### Changed

### Fixed

### Removed

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
