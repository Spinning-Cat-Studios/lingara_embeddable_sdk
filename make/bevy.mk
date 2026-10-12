# bevy/ — the crate bevy_lingara, its EK harness and the Bevy demo; and
# tools/demo-token-server, which the demo (and K4's, K7's) run against
# (ADR 1.10.26z).
#
# Every recipe names its toolchain. Bevy's MSRV is above the toolchain the
# rest of the workspace is tested on, and rustup reads a rust-toolchain.toml
# from the working directory, which is always the repo root here, so a file
# in bevy/ would be ignored and one at the root would move every other lane.
# This is make/core.mk's `cargo +$(CORE_MSRV)` shape (D1), and like it the
# command is a bare `cargo`: `$(CARGO)` may be a toolchain's own binary
# (mise exports one), which has no rustup `+toolchain` handling. CI installs
# the named toolchain; locally it must be in the rustup home on PATH.

EK_LANGS += bevy
BEVY_TOOLCHAIN ?= 1.98.1
BEVY_CARGO := cargo +$(BEVY_TOOLCHAIN)
CONFORMANCE_HARNESS_ek_bevy := $(BEVY_CARGO) run -q -p bevy-lingara-ek-harness

BEVY_PACKAGES := -p bevy_lingara -p bevy-lingara-ek-harness -p bevy_lingara_demo -p lingara_demo_token_server -p bevy-lingara-snippets
BEVY_TOKEN_PORT ?= 8787
# test-tools runs on the workspace toolchain, below Bevy's MSRV: these
# members are test-bevy's and bevy-demo-headless's.
WORKSPACE_TEST_EXCLUDES += --exclude bevy_lingara --exclude bevy-lingara-ek-harness --exclude bevy_lingara_demo --exclude bevy-lingara-snippets

.PHONY: codegen-bevy check-codegen-bevy test-bevy clippy-bevy bevy-demo bevy-demo-headless check-bevy-mobile check-bevy-wasm-fails check-package-bevy help-bevy

## Every EK case through the plugin. The recipe is make/conformance.mk's
## pattern rule; the name is spelled here so a shipped workflow may call it.
conformance-ek-bevy:

codegen-bevy:
	@:

check-codegen-bevy:
	@:

## The plugin's unit tests, its package and README tests, the token
## server's, and the snippets' region guards.
test-bevy:
	$(BEVY_CARGO) test -p bevy_lingara -p lingara_demo_token_server -p bevy-lingara-snippets

## clippy over every Bevy package, warnings denied. Needs the clippy
## component on $(BEVY_TOOLCHAIN).
clippy-bevy:
	$(BEVY_CARGO) clippy $(BEVY_PACKAGES) --all-targets -- -D warnings

## The demo against the token server on $(BEVY_TOKEN_PORT): start
## `$(BEVY_CARGO) run -p lingara_demo_token_server` first (bevy/demo/README.md).
bevy-demo:
	$(BEVY_CARGO) run -p bevy_lingara_demo

## The demo with no window, against `conformance-server serve` and the
## token server's --fake mode (D12).
bevy-demo-headless:
	$(CARGO) build -q -p lingara-embed-conformance-server
	$(BEVY_CARGO) build -q -p lingara_demo_token_server
	$(BEVY_CARGO) test -p bevy_lingara_demo

## The two mobile targets, as core builds them (D9). The operator installs
## the targets into $(BEVY_TOOLCHAIN), and cargo-ndk for Android.
check-bevy-mobile:
	$(BEVY_CARGO) ndk -t arm64-v8a --platform 23 check -p bevy_lingara
	IPHONEOS_DEPLOYMENT_TARGET=13.0 $(BEVY_CARGO) check -p bevy_lingara --target aarch64-apple-ios

## wasm32 is not a target (D9): the build must fail, on lingara-embed's message.
check-bevy-wasm-fails:
	@if out=$$($(BEVY_CARGO) check -p bevy_lingara --target wasm32-unknown-unknown 2>&1); then \
	  echo "✗ bevy_lingara built for wasm32"; exit 1; fi; \
	  case "$$out" in *"does not target wasm32"*) echo "✓ wasm32 refused: lingara-embed does not target wasm32";; \
	  *) echo "$$out" | tail -20; echo "✗ wasm32 failed, but not on lingara-embed's refusal"; exit 1;; esac

## `cargo package --list` names nothing outside src/**, README.md, LICENSE
## and cargo's own files.
check-package-bevy:
	@stray=$$($(BEVY_CARGO) package -p bevy_lingara --list --allow-dirty | grep -vE '^(src/|README\.md$$|LICENSE$$|Cargo\.toml$$|Cargo\.toml\.orig$$|Cargo\.lock$$|\.cargo_vcs_info\.json$$)'); \
	  if [ -n "$$stray" ]; then echo "✗ the package carries: $$stray"; exit 1; fi; \
	  echo "✓ bevy_lingara packages src/**, README.md and LICENSE only"

help-bevy:
	@echo ""
	@echo "bevy (bevy_lingara, on Rust $(BEVY_TOOLCHAIN)):"
	@echo "  make test-bevy              - The plugin, the token server and the snippets"
	@echo "  make clippy-bevy            - clippy over every Bevy package"
	@echo "  make conformance-ek-bevy    - Every EK case through the plugin"
	@echo "  make bevy-demo              - The windowed demo (token server first)"
	@echo "  make bevy-demo-headless     - The demo's headless test"
	@echo "  make check-bevy-mobile      - Android arm64 (API 23) and iOS arm64 (13.0)"
	@echo "  make check-bevy-wasm-fails  - wasm32 is refused"
	@echo "  make check-package-bevy     - The package list holds src/**, README.md, LICENSE"

HELP_SECTIONS += bevy
