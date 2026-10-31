# `godot/`

K4 (ADR 1.10.26ab): Lingara for GDScript games. C# projects use
`Lingara.Embed` (`dotnet/`) instead.

```
godot/
├── Cargo.toml · src/        lingara_godot: the GDExtension (gdext, api-4-5), a cdylib never published
├── addon/addons/lingara/    what a game copies in: .gdextension, plugin, autoload, LingaraNpc, README
│   └── bin/<rid>/           built, gitignored: one directory per K8 rid
├── addon/ASSETLIB.md        the Asset Library listing text
├── demo/                    the Godot demo, the EK harness and the headless demo test
├── ci/                      Godot pins and fetch, the addon zip, the headless demo runner
└── scripts/package-apple.sh the macOS framework and the iOS xcframework
```

## The crate

`LingaraEmbed` is a `Node` whose `_process` drains `lingara-embed`'s queue
once per frame (at most `max_events_per_frame`) and emits one signal per
event. Data crosses as `Dictionary`s in the core's own JSON. Only
`variant.rs` and the three `node*` files touch Godot; `convert.rs`,
`commands.rs`, `config.rs`, `token.rs`, `pump.rs` and `binding.rs` are plain
Rust, covered by `cargo test`. `stop()` never joins the core's thread; the
extension's deinitialisation joins every closed handle for at most a second,
so Godot never unmaps code a thread still runs.

gdext tracking is internal: users get a binary, never the crate, so a gdext
bump is a trio release's business. The Godot floor (4.5) moves only in a
trio minor release.

## Updates

An update notice is for you, the developer, and is never drawn for the
player. Set the node's `embed_version` property to the embed version this
build was registered against: the `token_url` route is sent it as
`Lingara-Embed-Version`, and a script that answers `token_needed` itself sends
that header itself and passes the handoff's `update_available` dictionary to
`provide_token_with_update(token, expires_at, update)` (`{}` is none). Once
per session the node emits `update_available(latest, deprecated, message)`
(a null `latest` or `message` is `""`), and the core's warning line reaches
`log_received` and, with `forward_logs`, the Godot log.

## Make targets

| Target | Does |
|---|---|
| `make godot-dist RID=<rid>` | One K8 rid's library into `addon/addons/lingara/bin/<rid>/` (`win-x64` · `osx` · `linux-x64` · `android-arm64` · `ios-arm64`) |
| `make godot-addon` | The host's rid, the inner loop (on macOS, this machine's arch only) |
| `make godot-addon-zip` | `addon/dist/lingara-godot-addon-<version>.zip`, refused while a rid is missing |
| `make test-godot` | The crate's tests, `gdlint`, the snippet's regions, and `--check-only` over the snippet |
| `make conformance-ek-godot` | Every EK case through `LingaraEmbed` |
| `make godot-headless` | The demo's headless test |
| `make godot-demo` | The demo in the editor |

`GODOT` is the official Linux binary for `GODOT_VERSION` (4.5.2, the floor),
fetched and SHA-256-checked from `ci/godot-versions.toml`, unless you pass
your own: `make test-godot GODOT=/Applications/Godot.app/Contents/MacOS/Godot`.
CI runs the three test lanes under 4.5.2 and 4.7.2.
