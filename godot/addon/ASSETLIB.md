# Asset Library listing: Lingara

What a person submits at each release (ADR 1.10.26ab D11; K8 Manual step 7).
K8 uploads nothing to the Asset Library.

| Field | Value |
|---|---|
| Title | Lingara |
| Category | Scripts |
| Godot version | 4.5 (the floor; it runs on every later 4.x) |
| Version | the release's version (`plugin.cfg`'s `version`) |
| Licence | MIT |
| Repository host | Custom |
| Download URL | the **direct** URL of `lingara-embed-godot-<version>.zip` on the public `lingara_embeddable_sdk` release |
| Icon URL | the raw URL of `addons/lingara/icon.svg` at the release tag |
| Issues URL | the public repository's issues page |

**Use the Custom provider with the direct download URL.** A default entry
downloads the repository archive, and `bin/` is not in git, so it would
install an addon with no libraries. The URL changes with every release
asset, so the listing is edited each time.

## Description

> Lingara inside a GDScript game. NPCs answer in the language the player is
> learning, at the player's level, streaming their reply as it is written;
> practice sessions over lesson plans Lingara makes report their results to
> your server. One autoload, signals for everything, and every network call
> off the main thread. Windows, macOS, Linux, Android and iOS (iOS is
> experimental). Needs a Lingara metered client and your own token route.
> C# projects: use Lingara.Embed from NuGet instead.
