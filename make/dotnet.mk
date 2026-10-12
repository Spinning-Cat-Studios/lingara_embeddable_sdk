# dotnet/ — NuGet Lingara.Embed, the player half of C# over ffi/ (ADR
# 1.10.26aa D8–D12), and NuGet Lingara.Embed.Server, the server half (ADR
# 1.10.26af D10), from one solution. The server half is pure managed code
# with no PackageReference: its lanes (test-dotnet-server, the ES harness)
# never need the FFI host library.
#
# Every test lane runs against the real host library, built debug by
# `ffi-host` and copied beside each test assembly by dotnet/Directory.Build.props
# (LingaraCopyHostNative). The package itself is packed only by `dotnet-pack`,
# which needs all five dist/ffi/<rid>/ directories and refuses otherwise.

DOTNET ?= dotnet
EK_LANGS += dotnet
CONFORMANCE_HARNESS_ek_dotnet := $(DOTNET) run -c Release --no-build --project dotnet/conformance-ek/Lingara.Embed.Conformance.csproj
ES_LANGS += dotnet
CONFORMANCE_HARNESS_es_dotnet := $(DOTNET) run -c Release --no-build --project dotnet/conformance-es/Lingara.Embed.Server.Conformance.csproj

DOTNET_DIR := dotnet
DOTNET_PKG := $(DOTNET_DIR)/src/Lingara.Embed/Lingara.Embed.csproj
DOTNET_TESTS := $(DOTNET_DIR)/tests/Lingara.Embed.Tests/Lingara.Embed.Tests.csproj
DOTNET_AOT := $(DOTNET_DIR)/tests/Lingara.Embed.AotSmoke/Lingara.Embed.AotSmoke.csproj
DOTNET_BENCH := $(DOTNET_DIR)/tests/Lingara.Embed.Bench/Lingara.Embed.Bench.csproj
DOTNET_GODOT := $(DOTNET_DIR)/tests/Lingara.Embed.GodotSnippet/Lingara.Embed.GodotSnippet.csproj
DOTNET_EK := $(DOTNET_DIR)/conformance-ek/Lingara.Embed.Conformance.csproj
DOTNET_SERVER := $(DOTNET_DIR)/src/Lingara.Embed.Server/Lingara.Embed.Server.csproj
DOTNET_SERVER_TESTS := $(DOTNET_DIR)/tests/Lingara.Embed.Server.Tests/Lingara.Embed.Server.Tests.csproj
DOTNET_SERVER_SNIPPETS := $(DOTNET_DIR)/tests/Lingara.Embed.Server.Snippets/Lingara.Embed.Server.Snippets.csproj
DOTNET_ES := $(DOTNET_DIR)/conformance-es/Lingara.Embed.Server.Conformance.csproj
DOTNET_FLAGS := -c Release --nologo
# The shape pack: test-dotnet's pack over stand-in natives, read by PackageShapeTests.
DOTNET_SHAPE_DIR := target/dotnet-shape
export DOTNET_CLI_TELEMETRY_OPTOUT := 1
export DOTNET_NOLOGO := 1

# This host's RID and native file name, for the AOT smoke and the resolver.
DOTNET_UNAME := $(shell uname -s)-$(shell uname -m)
DOTNET_HOST_RID = $(or $(DOTNET_RID_$(DOTNET_UNAME)),win-x64)
DOTNET_RID_Darwin-arm64 := osx-arm64
DOTNET_RID_Darwin-x86_64 := osx-x64
DOTNET_RID_Linux-x86_64 := linux-x64
DOTNET_RID_Linux-aarch64 := linux-arm64
DOTNET_HOST_LIB =$(if $(filter osx%,$(DOTNET_HOST_RID)),liblingara_embed.dylib,$(if $(filter linux%,$(DOTNET_HOST_RID)),liblingara_embed.so,lingara_embed.dll))

.PHONY: ffi-host dotnet-build test-dotnet dotnet-aot-smoke dotnet-bench dotnet-pack dotnet-godot-snippet \
  check-dotnet-samples codegen-dotnet check-codegen-dotnet help-dotnet \
  test-dotnet-server dotnet-server-es-build

## The debug host library every test lane loads.
ffi-host:
	$(CARGO) build -p lingara-embed-ffi

## The package (both targets), its tests, the AOT smoke, the bench and the EK harness.
dotnet-build: ffi-host
	$(DOTNET) --version
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_PKG)
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_TESTS)
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_BENCH)
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_EK)

## The pack refusal (AC29) and the shape pack, then xUnit over the real host
## library; then the server half's own lane.
test-dotnet: dotnet-build test-dotnet-server
	DOTNET="$(DOTNET)" SHAPE_DIR="$(DOTNET_SHAPE_DIR)" bash $(DOTNET_DIR)/tests/pack_refusal_test.sh
	LINGARA_EMBED_NUPKG="$$(ls $(CURDIR)/$(DOTNET_SHAPE_DIR)/artifacts/Lingara.Embed.*.nupkg | head -1)" \
	  $(DOTNET) test $(DOTNET_FLAGS) --no-build $(DOTNET_TESTS)

## NativeAOT publish with zero trim/AOT warnings, then the offline lifecycle (AC23).
dotnet-aot-smoke: ffi-host
	$(DOTNET) publish $(DOTNET_FLAGS) -r $(DOTNET_HOST_RID) -o target/dotnet-aot $(DOTNET_AOT)
	cp target/debug/$(DOTNET_HOST_LIB) target/dotnet-aot/
	./target/dotnet-aot/Lingara.Embed.AotSmoke

## Drain of 256 npc_turn fragments, timed (D12; a figure, not a gate).
dotnet-bench: dotnet-build
	$(DOTNET) run $(DOTNET_FLAGS) --no-build --project $(DOTNET_BENCH)

## The nupkg, from all five dist/ffi/<rid>/ (K8 runs this after ffi-dist on its matrix).
dotnet-pack:
	$(DOTNET) pack $(DOTNET_FLAGS) $(DOTNET_PKG) -p:FfiDist=$(or $(FFI_DIST_DIR),$(CURDIR)/dist/ffi) \
	  $(if $(PACK_OUT),-p:PackageOutputPath=$(PACK_OUT),)

## The Godot snippet compiles against Godot.NET.Sdk 4.4 (AC25).
dotnet-godot-snippet:
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_GODOT)

## Each sample's snippet copy is byte-identical to snippets/player/ (AC25).
check-dotnet-samples:
	cmp snippets/player/unity/LingaraEmbedBehaviour.cs $(DOTNET_DIR)/samples/unity/Assets/Scripts/LingaraEmbedBehaviour.cs
	cmp snippets/player/godot-csharp/LingaraEmbedNode.cs $(DOTNET_DIR)/samples/godot/LingaraEmbedNode.cs
	@echo "✓ both samples carry snippets/player/'s copy"

## The header and the message schema this package is held to.
codegen-dotnet: ffi-header ffi-schema

check-codegen-dotnet: check-ffi-header check-ffi-schema

## Every EK case through the public C# API. The recipe is make/conformance.mk's
## pattern rule; the build runs first, since the harness runs --no-build.
conformance-ek-dotnet: dotnet-build

## Lingara.Embed.Server (ADR 1.10.26af): the package under its analysers
## (warnings are errors), the server snippets compiled against it, and xUnit
## (the vectors, the router, E7's webhook vectors, the token cache, the
## package shape). No FFI host library.
test-dotnet-server:
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_SERVER)
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_SERVER_SNIPPETS)
	$(DOTNET) test $(DOTNET_FLAGS) $(DOTNET_SERVER_TESTS)

## The ES harness, built before make/conformance.mk's pattern rule runs it --no-build.
dotnet-server-es-build:
	$(DOTNET) build $(DOTNET_FLAGS) $(DOTNET_ES)

## Every ES case through Lingara.Embed.Server (ADR 1.10.26af).
conformance-es-dotnet: dotnet-server-es-build

help-dotnet:
	@echo ""
	@echo "dotnet (Lingara.Embed):"
	@echo "  make test-dotnet            - The pack refusal, the shape pack, xUnit over the host library"
	@echo "  make dotnet-aot-smoke       - NativeAOT publish (no warnings) and the offline lifecycle"
	@echo "  make dotnet-bench           - Drain of 256 npc_turn fragments, timed"
	@echo "  make dotnet-pack            - The nupkg, from all five dist/ffi/<rid>/"
	@echo "  make dotnet-godot-snippet   - The Godot C# snippet against Godot.NET.Sdk 4.4"
	@echo "  make check-dotnet-samples   - The samples' snippet copies are snippets/player/'s"
	@echo "  make conformance-ek-dotnet  - Every EK case through the C# harness"
	@echo ""
	@echo "dotnet (Lingara.Embed.Server):"
	@echo "  make test-dotnet-server     - The server package, its snippets and xUnit (no FFI host library)"
	@echo "  make conformance-es-dotnet  - Every ES case through the C# server harness"

HELP_SECTIONS += dotnet
