# Snippets

The code the docs show beside each answer, compiled or run in a kit's own
tests. Two tiers, by role rather than by package:

```
snippets/
├── player/<surface>/   bevy · unity · godot-csharp · gdscript · unreal · kotlin · java · web
├── server/<lang>/      typescript · rust · go · java · kotlin · ruby · php · csharp
└── dialogue/           missing-brother.json, the worked dialogue's IR
```

- **A surface is a product a developer recognises,** not a package directory:
  `unity` and `godot-csharp` are two surfaces over one `dotnet/` package.
- **There is no `player/core/`.** The Rust player examples are the core's
  doctests; Rust's server examples are `server/rust/`.
- **Rust, TypeScript, Kotlin and Java appear in both tiers,** so a language
  name alone never names a directory.

## Format

Each file is plain source in its language. A region the docs read is marked
with a comment pair in that language's comment syntax (`#` for Ruby and
shell, `<!--` … `-->` for HTML), the format the docs site's region reader
takes (29.9.26w D3):

```
// lingara:begin mint
…
// lingara:end
```

A key may open several regions in one file; the docs join them in file
order. A key in two files is an error. An `install.*` file is shown whole
and carries no markers.

`{{version}}` in a snippet is replaced with the released version when it is
shown. Each surface's `install.*` file is the one install line its package's
release proves.

## Keys

Each directory carries the regions the docs page for that surface reads. The
player surfaces: `install`, `tokenSource`, `poll` (`gdscript` and `unreal`
have no `install`: they install from a store by hand). The server languages:
`install`, `mint`, `deletePlayer`, `webhook`, `widgetSnippet` (ADR 1.10.26af D13).

The four surfaces with a dialogue builder (`bevy`, `unity`, `godot-csharp`,
`gdscript`) also carry `dialogueGraph`: the worked missing-brother dialogue,
built with that kit's builder and loaded with `DialogueLoad` (ADR 28.10.26g
D8). `unreal`, `kotlin`, `java` and `web` have no builder and no such key.

`dialogue/missing-brother.json` is that dialogue's IR, shown whole like an
`install.*` file: byte-equal to
`conformance/vectors/dialogue/missing-brother.comparison.json`
(`make check-dialogue-snippet`), and the Bevy region's bytes are tested
against it.

Each kit writes its own surfaces' snippets in the change that lands it.
