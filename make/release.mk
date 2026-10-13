# Release (ADR 1.10.26ag). Ships with the repository.
#
# `languages.toml` is what a release publishes, and tools/release/release.toml
# the native matrix and what each package must hold. release-manifest is the
# clients' tool, never copied here (D2): it is installed from the public
# lingara_api_clients repository at RELEASE_MANIFEST_TAG into .tools/
# (gitignored), the first time a target needs it.
#
# release.yml calls only the targets below, never a kit's own: the leak scan
# reads every word after a shipped workflow's `make` as a target a shipped
# make file must define, and a target spelt from a variable is none (D4). So
# each languages.toml id has one row here, filled from its kit's make file:
#
#   RELEASE_BUILD_<id>   the kit's own check and test targets
#   RELEASE_TIER_<id>    the conformance tiers it passes (ek, es, both, or none)
#   RELEASE_PACK_<id>    the command that packs it (empty: nothing to pack)
#
# and, where they differ from the id, RELEASE_LANG_<id> (the kit's name in
# EK_LANGS / ES_LANGS) and RELEASE_PACKED_<id> (the file check-package.sh
# opens, for a package with natives inside).

# A widening of release-manifest is a clients release first, then this pin
# (D2, Consequences).
RELEASE_MANIFEST_TAG := v0.2.0-alpha.1
RELEASE_MANIFEST_REPO := https://github.com/Spinning-Cat-Studios/lingara_api_clients
RELEASE_MANIFEST_ROOT := .tools/release-manifest-$(RELEASE_MANIFEST_TAG)
RELEASE_MANIFEST := $(RELEASE_MANIFEST_ROOT)/bin/release-manifest
RELEASE_VERSION = $(shell tr -d '[:space:]' < VERSION)
# What the Release names its assets with: the tag, `v` and all.
RELEASE_TAG ?= $(or $(TAG),v$(RELEASE_VERSION))
RELEASE_TOML := bash tools/release/release-toml.sh
GRADLE ?= ./gradlew

# ── The rows (Step 0: each kit's make file, read 2026-10-12) ───────────────

RELEASE_BUILD_core := check-codegen-core test-core check-msrv-core check-package-core
RELEASE_TIER_core := ek es
RELEASE_PACK_core = $(CARGO) package -p lingara-embed --locked

# Packaged with the core, so bevy_lingara resolves against the unpublished
# lingara-embed (Cargo >= 1.90's multi-package packaging).
RELEASE_BUILD_bevy := check-codegen-bevy test-bevy clippy-bevy check-package-bevy
RELEASE_TIER_bevy := ek
RELEASE_PACK_bevy = $(BEVY_CARGO) package -p lingara-embed -p bevy_lingara --locked

RELEASE_BUILD_dotnet := check-codegen-dotnet test-dotnet
RELEASE_TIER_dotnet := ek
RELEASE_PACK_dotnet = $(MAKE) --no-print-directory dotnet-pack
RELEASE_PACKED_dotnet = dotnet/artifacts/Lingara.Embed.$(RELEASE_VERSION).nupkg

RELEASE_BUILD_dotnetserver := test-dotnet-server
RELEASE_TIER_dotnetserver := es
RELEASE_LANG_dotnetserver := dotnet
RELEASE_PACK_dotnetserver = $(DOTNET) pack $(DOTNET_FLAGS) $(DOTNET_SERVER)

# The element is a frame, not a binding, so only the `/server` subpath runs a
# tier (ES); the element's own contract is its postMessage vector, in test-typescript.
RELEASE_BUILD_typescript := check-codegen-typescript test-typescript check-typescript
RELEASE_TIER_typescript := es
RELEASE_PACK_typescript = cd $(TS_DIR) && npm pack

RELEASE_BUILD_kotlin := check-codegen-kotlin test-kotlin
RELEASE_TIER_kotlin := ek
RELEASE_BUILD_java := check-codegen-java test-java
RELEASE_TIER_java := ek

# The natives run no tier of their own: the kotlin and java legs' EK runs
# load them. Their jar and AAR are built for check-package.sh only; Central
# receives what publish-maven builds.
RELEASE_BUILD_nativesdesktop := check-jvm-shape
RELEASE_PACK_nativesdesktop = $(GRADLE) $(GRADLE_FLAGS) :natives-desktop:jar
RELEASE_PACKED_nativesdesktop = natives-desktop/build/libs/natives-desktop-$(RELEASE_VERSION).jar
RELEASE_BUILD_nativesandroid := check-jvm-shape
RELEASE_PACK_nativesandroid = $(GRADLE) $(GRADLE_FLAGS) :natives-android:bundleReleaseAar
RELEASE_PACKED_nativesandroid = natives-android/build/outputs/aar/natives-android-release.aar

RELEASE_BUILD_kotlinserver := check-codegen-kotlin-server test-kotlin-server
RELEASE_TIER_kotlinserver := es
RELEASE_LANG_kotlinserver := kotlin-server
RELEASE_BUILD_javaserver := check-codegen-java-server test-java-server
RELEASE_TIER_javaserver := es
RELEASE_LANG_javaserver := java-server

RELEASE_BUILD_go := check-codegen-go test-go
RELEASE_TIER_go := es
RELEASE_BUILD_ruby := check-codegen-ruby test-ruby
RELEASE_TIER_ruby := es
RELEASE_BUILD_php := check-codegen-php test-php
RELEASE_TIER_php := es

RELEASE_BUILD_godot := check-codegen-godot test-godot check-godot-addon-zip
RELEASE_TIER_godot := ek
RELEASE_PACK_godot = $(MAKE) --no-print-directory godot-addon-zip
RELEASE_PACKED_godot = godot/addon/dist/lingara-godot-addon-$(RELEASE_VERSION).zip

# Not in languages.toml until K5's first engine smoke (D1): the row is ready
# for the entry that joins then.
RELEASE_BUILD_unreal := check-codegen-unreal test-unreal
RELEASE_TIER_unreal := ek
RELEASE_PACK_unreal = $(MAKE) --no-print-directory unreal-package unreal-bundle
RELEASE_PACKED_unreal = $(UNREAL_DIST)/lingara-embed-unreal-$(RELEASE_VERSION).zip

# The linux-x64 natives build inside release.toml's manylinux_2_28 container
# (D3): it holds a C toolchain and Python, so the toolchains go in first, into
# target/container/, as the workspace's owner. The checks run on the host.
# PATH is prepended inside the container, never set with -e: that replaces the
# image's own, which holds its entrypoint (/usr/local/bin) and gcc-toolset.
RELEASE_CONTAINER_ENV := -e HOME=/work/target/container/home -e CARGO_HOME=/work/target/container/cargo \
  -e RUSTUP_HOME=/work/target/container/rustup
RELEASE_CONTAINER_SETUP := export PATH="/work/target/container/cargo/bin:/opt/python/cp312-cp312/bin:$$PATH" && \
  mkdir -p "$$HOME" && \
  { command -v rustup >/dev/null || curl -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal --no-modify-path; } && \
  rustup toolchain install stable $(GODOT_TOOLCHAIN) --profile minimal && \
  python3 -m pip install --quiet --user ziglang && \
  { command -v cargo-zigbuild >/dev/null || cargo install cargo-zigbuild --locked; }

.PHONY: release-manifest-install bump-version release-matrix release-build release-conformance release-pack \
        release-natives release-ffi-zip release-cpp-ports unreal-bundle check-release-tree check-release-tools help-release

$(RELEASE_MANIFEST):
	$(CARGO) install --git $(RELEASE_MANIFEST_REPO) --tag $(RELEASE_MANIFEST_TAG) --locked --root $(RELEASE_MANIFEST_ROOT) release-manifest

## The pinned release-manifest; CI passes RELEASE_MANIFEST_ROOT to choose
## where, and puts its bin/ on PATH for tools/release/*.sh.
release-manifest-install: $(RELEASE_MANIFEST)

## Write V to VERSION, every version file and every `since = "next"`, then
## regenerate what each kit writes from VERSION, so check-codegen stays green.
bump-version: $(RELEASE_MANIFEST)
	@test -n "$(V)" || { echo "Usage: make bump-version V=X.Y.Z[-pre.N]" >&2; exit 2; }
	$(RELEASE_MANIFEST) bump $(V)
	$(MAKE) codegen

## The GitHub Actions matrix release.yml fans out over, for TAG.
release-matrix: $(RELEASE_MANIFEST)
	@test -n "$(TAG)" || { echo "Usage: make release-matrix TAG=vX.Y.Z[-pre.N]" >&2; exit 2; }
	@$(RELEASE_MANIFEST) matrix $(TAG)

# An id with no row is refused by name, never run as an empty make.
release-row = $(if $(filter undefined,$(origin RELEASE_BUILD_$(ID))),$(error ID=$(ID) has no row in make/release.mk),)

## One package's release build: its kit's own check and test targets.
release-build:
	@test -n "$(ID)" || { echo "Usage: make release-build ID=<languages.toml id>" >&2; exit 2; }
	$(release-row)$(MAKE) $(RELEASE_BUILD_$(ID))

## Its conformance tiers; pass CONFORMANCE_SERVER to reuse a built server.
release-conformance:
	@test -n "$(ID)" || { echo "Usage: make release-conformance ID=<languages.toml id>" >&2; exit 2; }
	$(release-row)$(if $(RELEASE_TIER_$(ID)),$(MAKE) $(foreach t,$(RELEASE_TIER_$(ID)),conformance-$(t)-$(or $(RELEASE_LANG_$(ID)),$(ID))),@echo "- $(ID): no tier of its own")

## Its pack step, then check-package.sh over the result when natives are inside.
release-pack:
	@test -n "$(ID)" || { echo "Usage: make release-pack ID=<languages.toml id>" >&2; exit 2; }
	$(release-row)$(if $(RELEASE_PACK_$(ID)),$(RELEASE_PACK_$(ID)),@echo "- $(ID): nothing to pack")
	$(if $(RELEASE_PACKED_$(ID)),bash tools/release/check-package.sh package $(ID) $(RELEASE_PACKED_$(ID)),)

## Every [[native]] of release.toml that applies to RID, in the platform's
## container where it names one. The checks are check-package.sh arch and
## version-check.sh, on the host.
release-natives:
	@test -n "$(RID)" || { echo "Usage: make release-natives RID=<rid>" >&2; exit 2; }
	@# Read before looping: a failing reader piped into `while read` builds
	@# nothing and exits 0.
	@container=$$($(RELEASE_TOML) container $(RID)) || exit 1; \
	natives=$$($(RELEASE_TOML) natives $(RID)) || exit 1; \
	[ -n "$$natives" ] || { echo "✗ $(RID): release.toml names no native"; exit 1; }; \
	printf '%s\n' "$$natives" | while read -r args; do \
	  echo "→ make $$args$${container:+ (in $$container)}"; \
	  if [ -n "$$container" ]; then \
	    docker run --rm --user "$$(id -u):$$(id -g)" -v "$(CURDIR):/work" -w /work $(RELEASE_CONTAINER_ENV) "$$container" \
	      bash -c '$(RELEASE_CONTAINER_SETUP) && make --no-print-directory '"$$args" || exit 1; \
	  else $(MAKE) --no-print-directory $$args || exit 1; fi; \
	done

## lingara-embed-ffi-<tag>-<rid>.zip: an install tree (ADR 8.10.26q D4) of
## dist/ffi/<rid>/'s library, K3's C header, K5's C++ wrapper, the stamped
## CMake package and LICENSE. FFI_ZIP_OUT moves it (make test-cpp does).
FFI_ZIP_OUT ?= dist/release
FFI_ZIP_VERSION = $(patsubst v%,%,$(RELEASE_TAG))
# CMake versions are numeric, so the package version drops any pre-release.
FFI_ZIP_CMAKE_VERSION = $(firstword $(subst -, ,$(FFI_ZIP_VERSION)))
FFI_ZIP_STAGE = target/ffi-zip/$(RID)
FFI_ZIP = $(FFI_ZIP_OUT)/lingara-embed-ffi-$(RELEASE_TAG)-$(RID).zip
release-ffi-zip:
	@test -n "$(RID)" || { echo "Usage: make release-ffi-zip RID=<rid> [TAG=v…]" >&2; exit 2; }
	@test -d dist/ffi/$(RID) || { echo "✗ no dist/ffi/$(RID): run make ffi-dist RID=$(RID)"; exit 1; }
	@rm -rf $(FFI_ZIP_STAGE) && mkdir -p $(FFI_ZIP_STAGE)/include $(FFI_ZIP_STAGE)/lib $(FFI_ZIP_STAGE)/share/lingara-embed $(FFI_ZIP_OUT)
	cp ffi/include/lingara_embed.h unreal/LingaraEmbed/Source/ThirdParty/LingaraEmbedLibrary/include/lingara_embed.hpp $(FFI_ZIP_STAGE)/include/
	cp LICENSE $(FFI_ZIP_STAGE)/
	@# The DLL under bin/, its import library under lib/. Windows' static
	@# lingara_embed.lib is never shipped: it carries its own CRT (D2).
	@for f in dist/ffi/$(RID)/*; do case "$${f##*/}" in \
	  *.h|lingara_embed.lib) ;; \
	  *.dll) mkdir -p $(FFI_ZIP_STAGE)/bin && cp "$$f" $(FFI_ZIP_STAGE)/bin/ ;; \
	  *) cp "$$f" $(FFI_ZIP_STAGE)/lib/ ;; esac; done
	@for f in lingara-embed-config lingara-embed-config-version; do \
	  sed -e 's|@VERSION@|$(FFI_ZIP_VERSION)|g' -e 's|@CMAKE_VERSION_NUMBER@|$(FFI_ZIP_CMAKE_VERSION)|g' \
	    cpp/cmake/$$f.cmake.in > $(FFI_ZIP_STAGE)/share/lingara-embed/$$f.cmake || exit 1; done
	@rm -f $(FFI_ZIP)
	@# Python's zipfile, not zip: Git Bash on the win-x64 leg has no zip, and
	@# every natives leg has Python. Naming the top-level entries stores the
	@# tree at the archive's root, with no wrapping directory.
	cd $(FFI_ZIP_STAGE) && "$$(command -v python3 || command -v python)" -m zipfile \
	  -c $(abspath $(FFI_ZIP)) *
	@echo "✓ $(FFI_ZIP)"

## The vcpkg port and Conan recipe zips for TAG, from the five ffi zips in
## ZIPS (ADR 8.10.26q D5). The recipe is make/cpp.mk's cpp-ports.
release-cpp-ports:
	@test -n "$(ZIPS)" || { echo "Usage: make release-cpp-ports ZIPS=<dir of the five ffi zips> [TAG=v…]" >&2; exit 2; }
	$(MAKE) --no-print-directory cpp-ports ZIPS=$(ZIPS) RELEASE_TAG=$(RELEASE_TAG)

## K5's per-engine Fab zips, unchanged, in one bundle beside them (D1): one
## Release asset whatever unreal/engine-versions.txt lists.
unreal-bundle:
	@cd $(UNREAL_DIST) 2>/dev/null || { echo "✗ no $(UNREAL_DIST): run make unreal-package"; exit 1; }
	@zips=""; for ue in $$(grep -E '^5\.[0-9]+$$' $(UNREAL_VERSIONS) | tr -d '\r'); do \
	  zip=LingaraEmbed-$(RELEASE_VERSION)-UE$$ue.zip; \
	  [ -f $(UNREAL_DIST)/$$zip ] || { echo "✗ no $(UNREAL_DIST)/$$zip: run make unreal-package"; exit 1; }; \
	  zips="$$zips $$zip"; done; \
	rm -f $(UNREAL_DIST)/lingara-embed-unreal-$(RELEASE_VERSION).zip; \
	cd $(UNREAL_DIST) && zip -q lingara-embed-unreal-$(RELEASE_VERSION).zip $$zips; \
	echo "✓ $(UNREAL_DIST)/lingara-embed-unreal-$(RELEASE_VERSION).zip:$$zips"

## languages.toml agrees with the tree (check-publishable runs it).
check-release-tree: $(RELEASE_MANIFEST)
	$(RELEASE_MANIFEST) check

## The release scripts' own tests, offline: check-package.sh and prove-install.sh.
check-release-tools:
	bash tools/release/check_package_test.sh
	bash tools/release/prove_install_test.sh

help-release:
	@echo ""
	@echo "Release:"
	@echo "  make release-manifest-install    - The pinned release-manifest ($(RELEASE_MANIFEST_TAG)), into .tools/"
	@echo "  make bump-version V=<ver>        - Write the version everywhere, then make codegen"
	@echo "  make release-matrix TAG=<t>      - The release matrix for a tag"
	@echo "  make release-build ID=<id>       - One package's checks and tests"
	@echo "  make release-conformance ID=<id> - One package's conformance tiers"
	@echo "  make release-pack ID=<id>        - Pack it, then check-package over natives inside"
	@echo "  make release-natives RID=<rid>   - Every native build of release.toml for one platform"
	@echo "  make release-ffi-zip RID=<rid>   - The C ABI's release zip for one platform"
	@echo "  make release-cpp-ports ZIPS=<d>  - The vcpkg port and Conan recipe zips over the five"
	@echo "  make unreal-bundle               - K5's per-engine zips in one bundle"
	@echo "  make check-release-tree          - languages.toml agrees with the tree"
	@echo "  make check-release-tools         - check-package.sh and prove-install.sh, offline"

HELP_SECTIONS += release
