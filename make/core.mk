# core/ — the crate lingara-embed (ADR 1.10.26y).
#
# It joins EK_LANGS as `core`: its harness drives every EK case through the
# Rust surface. Its `server` feature (ADR 1.10.26af) joins ES_LANGS as `core`
# too, with its own harness. It generates nothing, so its codegen targets are
# no-ops that exist only because make/quality.mk's aggregates run them over
# $(LANGS).

EK_LANGS += core
CONFORMANCE_HARNESS_ek_core := $(CARGO) run -q -p lingara-embed-ek-harness
ES_LANGS += core
CONFORMANCE_HARNESS_es_core := $(CARGO) run -q -p lingara-embed-es-harness

CORE_MANIFEST := core/Cargo.toml
CORE_MSRV := 1.87
# D11's five platforms. The macOS universal binary and the static iOS
# library are the bindings' artefacts: this crate only builds at each floor.
CORE_TARGETS := x86_64-pc-windows-msvc aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu aarch64-linux-android aarch64-apple-ios

.PHONY: codegen-core check-codegen-core test-core check-msrv-core check-package-core build-core-targets help-core \
        test-dialogue check-package-dialogue

## Every EK case through the Rust surface. The recipe is make/conformance.mk's
## pattern rule; the name is spelled here so a shipped workflow may call it.
conformance-ek-core:

## Every ES case through the `server` feature (ADR 1.10.26af).
conformance-es-core:

codegen-core:
	@:

check-codegen-core:
	@:

## lingara-dialogue-ir (ADR 28.10.26f D11): clippy and its tests, with and
## without `schema`, the doctests included.
test-dialogue:
	$(CARGO) clippy -p lingara-dialogue-ir --all-targets --features schema -- -D warnings
	$(CARGO) test -p lingara-dialogue-ir
	$(CARGO) test -p lingara-dialogue-ir --features schema

## clippy, both TLS features, `schema`, `server`, the doctests, and the EK
## harness's own tests (the shape vector, the spec limits, the handoff vector).
## The dialogue IR crate first, which the core will pin (D11).
test-core: test-dialogue
	$(CARGO) clippy -p lingara-embed -p lingara-embed-ek-harness -p lingara-embed-es-harness -p lingara-embed-server-snippets --all-targets --features lingara-embed/schema,lingara-embed/server -- -D warnings
	$(CARGO) test -p lingara-embed
	$(CARGO) test -p lingara-embed --features server
	$(CARGO) test -p lingara-embed --no-default-features --features native-tls
	$(CARGO) test -p lingara-embed --features schema --doc
	$(CARGO) test -p lingara-embed-ek-harness

## The packaged crate, its lock removed, builds and tests on Rust 1.87
## outside the workspace, under each TLS feature (D1).
## Unpacked outside the repo, so cargo cannot find the workspace above it.
CORE_MSRV_DIR ?= $(or $(TMPDIR),/tmp)/lingara-embed-msrv

check-msrv-core:
	@rm -rf target/msrv-core $(CORE_MSRV_DIR) && mkdir -p $(CORE_MSRV_DIR)
	$(CARGO) package -p lingara-embed --no-verify --allow-dirty --target-dir target/msrv-core
	tar -xzf target/msrv-core/package/lingara-embed-*.crate -C $(CORE_MSRV_DIR)
	cd $(CORE_MSRV_DIR)/lingara-embed-* && rm -f Cargo.lock && \
	  cargo +$(CORE_MSRV) test --target-dir ../build && \
	  cargo +$(CORE_MSRV) test --no-default-features --features native-tls --target-dir ../build

## `cargo package --list` names nothing outside src/**, README.md, LICENSE
## and cargo's own files.
check-package-core:
	@stray=$$($(CARGO) package -p lingara-embed --list --allow-dirty | grep -vE '^(src/|README\.md$$|LICENSE$$|Cargo\.toml$$|Cargo\.toml\.orig$$|Cargo\.lock$$|\.cargo_vcs_info\.json$$)'); \
	  if [ -n "$$stray" ]; then echo "✗ the package carries: $$stray"; exit 1; fi; \
	  echo "✓ lingara-embed packages src/**, README.md and LICENSE only"

## lingara-dialogue-ir packages only src/**, predicate.ebnf, README.md and
## LICENSE, and its non-dev dependencies are serde, serde_json and the
## optional schemars (ADR 28.10.26f AC25): Backend links it, so it stays light.
check-package-dialogue:
	@stray=$$($(CARGO) package -p lingara-dialogue-ir --list --allow-dirty | grep -vE '^(src/|predicate\.ebnf$$|README\.md$$|LICENSE$$|Cargo\.toml$$|Cargo\.toml\.orig$$|Cargo\.lock$$|\.cargo_vcs_info\.json$$)'); \
	  if [ -n "$$stray" ]; then echo "✗ the package carries: $$stray"; exit 1; fi
	@deps=$$($(CARGO) tree -p lingara-dialogue-ir --edges normal --depth 1 --all-features --prefix none --format '{p}' | tail -n +2 | cut -d' ' -f1 | sort -u); \
	  stray=$$(printf '%s\n' "$$deps" | grep -vxE 'serde|serde_json|schemars'); \
	  if [ -n "$$stray" ]; then echo "✗ lingara-dialogue-ir depends on: $$stray"; exit 1; fi
	@echo "✓ lingara-dialogue-ir packages src/**, predicate.ebnf, README.md and LICENSE, over serde, serde_json and schemars"

## One target of D11 (TARGET=<triple>); the CI matrix runs each on its runner.
build-core-targets:
	$(CARGO) build -p lingara-embed --release --target $(TARGET)

help-core:
	@echo ""
	@echo "core (lingara-embed):"
	@echo "  make test-dialogue         - lingara-dialogue-ir: clippy, its tests, with and without schema"
	@echo "  make check-package-dialogue - Its package list and its three dependencies (AC25)"
	@echo "  make test-core             - test-dialogue, then clippy, both TLS features, schema, doctests, harness tests"
	@echo "  make conformance-ek-core   - Every EK case through the Rust surface"
	@echo "  make conformance-es-core   - Every ES case through the server feature"
	@echo "  make check-msrv-core       - The packaged crate on Rust $(CORE_MSRV), lock removed"
	@echo "  make check-package-core    - The package list holds src/**, README.md, LICENSE"
	@echo "  make build-core-targets TARGET=<triple> - One of: $(CORE_TARGETS)"

HELP_SECTIONS += core
