# `unreal/`

The Unreal Engine 5 plugin over `ffi/`'s C ABI, and everything that tests it
without the engine (ADR 1.10.26ac).

| Path | What it is |
|---|---|
| `LingaraEmbed/` | The plugin a game drops into `Plugins/`: a ThirdParty module wrapping `lingara_embed.h` and the five platforms' libraries, plus a Runtime module with the `ULingaraEmbedSubsystem`, two Blueprint async actions, project settings and the automation specs. See its [README](LingaraEmbed/README.md). |
| `harness/` | An engine-free CMake harness: doctest suites over `lingara_embed.hpp` (C++17 and C++20, exceptions and RTTI off, warnings as errors, ASan and UBSan), and the EK fixture that drives the wrapper against `conformance-server`. |
| `smoke/` | The smallest host project the engine smoke packages the sample in. A kit test, not a showcase. |
| `tools/` | `check_plugin.sh`, `check_coverage.sh`, `package.sh` and their tests; `smoke.sh`, the scripted manual engine smoke. |
| `engine-versions.txt` | The supported engine versions, one `5.<m>` per line. The first line is the oldest supported version. |

## What CI proves, and what only the engine can

The line is one question: does the code include an Unreal header?

| Layer | Proved by | When |
|---|---|---|
| `lingara_embed.hpp`: all nine functions, buffer growth, the ABI check, move semantics | `harness/` doctest suites | every PR, Linux, macOS, Windows |
| The ABI path a game takes, end to end | every EK case through `harness/fixture/` | every PR |
| The plugin's copy of the header | `header_tests.cpp`, `check-codegen-unreal` | every PR |
| Descriptor, settings header, lib/, snippet regions, package shape | `tools/check_plugin.sh` | every PR |
| Decoder and encoder **mention** every K1 tag | `tools/check_coverage.sh` | every PR |
| Decoder and encoder **correctness**, the subsystem, the async actions | automation specs | `make unreal-smoke` only |
| `Build.cs`, UPL and linking on five platforms; the snippet compiles; a device talks to Lingara | `make unreal-smoke` | before the first release that ships the plugin, and before each Fab submission |

So the EK conformance for this surface runs on the layer beneath the engine:
the C++ wrapper over the C ABI, which is exactly what the Runtime module
calls. A change to the decoder, the encoder or the subsystem can merge green
and break in the engine until the next smoke.

## Make targets

| Target | What it does | Needs the engine |
|---|---|---|
| `make test-unreal` | the harness suites (+ sanitizers), the header, plugin and coverage checks, the tools' tests | no |
| `make unreal-tidy` | clang-tidy at 50 lines, 5 parameters, complexity 10 over the wrapper and harness | no |
| `make conformance-ek-unreal` | every EK case through the C++ fixture | no |
| `make codegen-unreal` / `check-codegen-unreal` | copy / compare `ffi/include/lingara_embed.h` | no |
| `make unreal-stage` | `dist/ffi/<rid>/` into the plugin's `lib/<Platform>/` | no |
| `make unreal-package [UE=5.<m>]` | one zip per supported engine version into `dist/unreal/` | no |
| `make unreal-smoke UE_ROOT=<engine> UE=5.<m>` | the four-stage engine smoke | **yes** |
