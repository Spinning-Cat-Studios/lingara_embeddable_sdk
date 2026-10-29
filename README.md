# Lingara embed kits

Put Lingara inside your game or website. A player practises the language
they are learning in your world: an NPC speaks at their level, and their
practice counts on Lingara. They need no Lingara account.

## Two tiers

Every integration has two halves, and they never hold the same secret.

| Tier | Runs on | Holds | Does |
|---|---|---|---|
| **Player kit** | the player's device: a game, or a web page | an embed token, and nothing else | streams events, reports practice, holds NPC conversations |
| **Server kit** | your server | your OAuth client's secret and your webhook signing secret | mints embed tokens for your players, deletes players, receives signed webhooks |

Anything shipped inside a game binary or a web page is public, so a player
kit never sees your client secret.

## The token flow

1. The player kit asks your server for a token.
2. Your server mints one for that player with a server kit
   (`POST /v1/embed/tokens`). An embed token starts `lgr_et_` and lives
   900 seconds.
3. The player kit uses it until it nears expiry, then asks your server again.
   Lingara never refreshes it.

Only a metered OAuth client may mint, and the usage is billed to it.

## Install

Every package is a **pre-release** until `1.0.0`, and all sixteen share one
version: install the newest `0.1.0-alpha.N` (npm's `next` tag).

| Surface | Package |
|---|---|
| Rust (player core; server kit behind `server`) | crates.io `lingara-embed` |
| Bevy | crates.io `bevy_lingara` |
| The dialogue graph format (`dialogue.v1`: its types, checks and predicate language, for a server that validates one) | crates.io `lingara-dialogue-ir` |
| Unity, Godot C# | NuGet `Lingara.Embed` |
| Godot (GDScript) | the Godot Asset Library's **Lingara**, or this repository's release asset `lingara-embed-godot-<version>.zip` |
| Kotlin, Java, libGDX, Android | `maven.getlingara.com` (the repository block is in each kit's README) `com.getlingara:lingara-embed-kotlin` or `-java`, with `lingara-embed-natives-desktop` or `lingara-embed-natives-android` |
| Websites | npm `@lingara/embed` (also on jsDelivr) |
| C and C++ | this repository's release assets: `lingara-embed-ffi-<version>-<platform>.zip` for `win-x64`, `osx`, `linux-x64`, `android-arm64` and `ios-arm64` (a CMake package: plain, or by CPM or FetchContent), a vcpkg overlay port, and a Conan recipe; all give `find_package(lingara-embed)`. See [cpp/](cpp/README.md) |
| Unreal Engine 5 | **not released yet**: the plugin is built and tested without the engine, and ships once it has run inside one |
| Server: TypeScript | npm `@lingara/embed`, `/server` |
| Server: Rust | crates.io `lingara-embed`, feature `server` |
| Server: Go | `github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/go` |
| Server: Java, Kotlin | `maven.getlingara.com` `com.getlingara:lingara-embed-server-java`, `lingara-embed-server-kotlin` |
| Server: Ruby | RubyGems `lingara-embed` |
| Server: PHP | Packagist `spinningcatstudios/lingara-embed` |
| Server: C# | NuGet `Lingara.Embed.Server` |

macOS libraries carry only an ad-hoc signature. A zip downloaded in a browser
may meet Gatekeeper's quarantine; installing through NuGet, Maven or the Godot
editor avoids it.

## Conformance

`conformance/` holds the two contracts every kit is held to, and the
conformance server that plays Lingara for both tiers. See
[`conformance/README.md`](conformance/README.md).

## Licence

MIT. See [`LICENSE`](LICENSE).
