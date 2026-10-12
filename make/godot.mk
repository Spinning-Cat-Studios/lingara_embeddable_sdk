# godot/ — the GDExtension crate lingara_godot, the addon a game copies in
# (godot/addon/addons/lingara/), the Godot demo with the EK harness and the
# headless demo test, and snippets/player/gdscript/ (ADR 1.10.26ab).
#
# The crate is built on its own toolchain, as make/bevy.mk's are: gdext 0.5's
# MSRV is above the workspace's, and the recipe is a bare `cargo +toolchain`
# for the reason that file gives.
#
# `GODOT` resolves itself. Unset, it is the official Linux binary for
# GODOT_VERSION (the floor, D1), downloaded and checked against its SHA-256
# in godot/ci/godot-versions.toml by `godot-fetch`. The fetch happens the
# first time a recipe expands $(GODOT), never when this file is read, so
# `make help` downloads nothing. Any other host passes its own:
# `make test-godot GODOT=/Applications/Godot.app/Contents/MacOS/Godot`.

EK_LANGS += godot
GODOT_TOOLCHAIN ?= 1.98.1
GODOT_CARGO := cargo +$(GODOT_TOOLCHAIN)
GODOT_PROFILE := release-godot
GODOT_VERSION ?= 4.5.2
GODOT_ADDON := godot/addon/addons/lingara
GODOT_RIDS := win-x64 osx linux-x64 android-arm64 ios-arm64
GODOT_RID_PATTERN := win-x64|osx|linux-x64|android-arm64|ios-arm64
GODOT_DEMO := godot/demo
GODOT_SNIPPET := snippets/player/gdscript
GDLINT ?= gdlint

ifeq ($(GODOT),)
GODOT = $(eval GODOT := $$(shell $(MAKE) -s --no-print-directory godot-fetch))$(GODOT)
endif

# conformance-server spawns this. A sub-make, so the harness resolves GODOT
# itself and reading the variable (check-conformance-coverage does) fetches
# nothing.
CONFORMANCE_HARNESS_ek_godot = $(MAKE) -s --no-print-directory godot-ek-harness
# test-tools runs on the workspace toolchain, below gdext's MSRV.
WORKSPACE_TEST_EXCLUDES += --exclude lingara_godot

# One rid's cargo invocation and the file it writes, per K8's rids.
GODOT_TARGET_DIR = target/$(1)/$(GODOT_PROFILE)
GODOT_OSX_ARCHS ?= aarch64 x86_64
GODOT_HOST_RID := $(if $(filter Darwin,$(shell uname -s)),osx,$(if $(filter Linux,$(shell uname -s)),linux-x64,win-x64))
GODOT_HOST_ARCH := $(subst arm64,aarch64,$(shell uname -m))

.PHONY: codegen-godot check-codegen-godot godot-dist godot-addon godot-addon-zip check-godot-addon-zip check-godot-dist godot-fetch godot-demo-addon godot-import \
        test-godot godot-headless godot-demo godot-ek-harness help-godot

## The plugin's version is plugin.cfg's, written by release-manifest bump: nothing to generate.
codegen-godot:
	@:

check-codegen-godot:
	@:

## One K8 rid's library into $(GODOT_ADDON)/bin/<rid>/, built with release-godot.
godot-dist:
	@case "$(RID)" in $(GODOT_RID_PATTERN)) ;; *) echo "✗ RID must be one of: $(GODOT_RIDS)"; exit 1;; esac
	@mkdir -p $(GODOT_ADDON)/bin/$(RID)
	@$(MAKE) -s --no-print-directory godot-dist-$(RID)

.PHONY: godot-dist-win-x64 godot-dist-linux-x64 godot-dist-osx godot-dist-ios-arm64 godot-dist-android-arm64
godot-dist-win-x64:
	$(GODOT_CARGO) build -p lingara_godot --profile $(GODOT_PROFILE) --target x86_64-pc-windows-msvc
	cp $(call GODOT_TARGET_DIR,x86_64-pc-windows-msvc)/lingara_godot.dll $(GODOT_ADDON)/bin/win-x64/

godot-dist-linux-x64:
	$(GODOT_CARGO) build -p lingara_godot --profile $(GODOT_PROFILE) --target x86_64-unknown-linux-gnu
	cp $(call GODOT_TARGET_DIR,x86_64-unknown-linux-gnu)/liblingara_godot.so $(GODOT_ADDON)/bin/linux-x64/

godot-dist-osx:
	for arch in $(GODOT_OSX_ARCHS); do \
	  MACOSX_DEPLOYMENT_TARGET=11.0 $(GODOT_CARGO) build -p lingara_godot --profile $(GODOT_PROFILE) --target $$arch-apple-darwin || exit 1; done
	bash godot/scripts/package-apple.sh osx $(GODOT_ADDON)/bin/osx $(foreach a,$(GODOT_OSX_ARCHS),$(call GODOT_TARGET_DIR,$(a)-apple-darwin)/liblingara_godot.dylib)

godot-dist-ios-arm64:
	IPHONEOS_DEPLOYMENT_TARGET=13.0 $(GODOT_CARGO) build -p lingara_godot --profile $(GODOT_PROFILE) --target aarch64-apple-ios
	bash godot/scripts/package-apple.sh ios $(GODOT_ADDON)/bin/ios-arm64 $(call GODOT_TARGET_DIR,aarch64-apple-ios)/liblingara_godot.dylib

godot-dist-android-arm64:
	$(GODOT_CARGO) ndk -t arm64-v8a --platform 23 build -p lingara_godot --profile $(GODOT_PROFILE)
	cp $(call GODOT_TARGET_DIR,aarch64-linux-android)/liblingara_godot.so $(GODOT_ADDON)/bin/android-arm64/

## A built rid's architecture: osx universal (both slices), the mobile pair arm64.
check-godot-dist:
	@case "$(RID)" in \
	  osx) info=$$(lipo -info $(GODOT_ADDON)/bin/osx/liblingara_godot.framework/liblingara_godot); \
	       case "$$info" in *x86_64*arm64*|*arm64*x86_64*) ;; *) echo "✗ osx is not universal: $$info"; exit 1;; esac;; \
	  ios-arm64) lipo -info $(GODOT_ADDON)/bin/ios-arm64/lingara_godot.xcframework/ios-arm64/lingara_godot.framework/lingara_godot | grep -q arm64 \
	       || { echo "✗ ios-arm64 is not arm64"; exit 1; };; \
	  android-arm64) file $(GODOT_ADDON)/bin/android-arm64/liblingara_godot.so | grep -q "ARM aarch64" \
	       || { echo "✗ android-arm64 is not aarch64"; exit 1; };; \
	  *) echo "✗ RID must be osx, ios-arm64 or android-arm64"; exit 1;; esac
	@echo "✓ $(RID): the right architecture"

## The inner loop: godot-dist for this host's rid (on macOS, this machine's arch only).
godot-addon:
	@$(MAKE) -s --no-print-directory godot-dist RID=$(GODOT_HOST_RID) $(if $(filter osx,$(GODOT_HOST_RID)),GODOT_OSX_ARCHS=$(GODOT_HOST_ARCH))

## The release zip K8 reads. Refuses while any rid is missing; no godot-addon
## prerequisite, so a host build never lands in a release zip.
godot-addon-zip:
	bash godot/ci/make-addon-zip.sh godot/addon "$(GODOT_RIDS)"

## The zip's two rules over a fixture bin/: only addons/lingara/**, and a missing rid refused.
check-godot-addon-zip:
	bash godot/ci/addon_zip_test.sh

## The Linux binary for GODOT_VERSION, SHA-256 checked; prints its path only.
godot-fetch:
	@bash godot/ci/fetch-godot.sh godot/ci/godot-versions.toml $(GODOT_VERSION) godot/.cache

## The addon copied into the demo and the snippet project (copies, gitignored).
godot-demo-addon: godot-addon
	for project in $(GODOT_DEMO) $(GODOT_SNIPPET); do \
	  rm -rf $$project/addons/lingara && mkdir -p $$project/addons && cp -R $(GODOT_ADDON) $$project/addons/ || exit 1; done

## The first import, which writes .godot/extension_list.cfg, so the extension loads.
# Each runs at most twice. Godot 4.5.2's editor can crash on exit after a
# project's first import (signal 11 in EditorHelp's extension-docs pass during
# Main::cleanup, measured on the macOS build 2026-10-10; 4.7.2 does not). The
# import itself has finished by then, so a second, clean run is the check: a
# failure that is not that one fails twice.
godot-import: godot-demo-addon
	for project in $(GODOT_DEMO) $(GODOT_SNIPPET); do \
	  $(GODOT) --headless --path $$project --import || $(GODOT) --headless --path $$project --import || exit 1; done

## The crate's tests, gdlint, the snippet's regions, and --check-only over the snippet.
test-godot: godot-import
	$(GODOT_CARGO) test -p lingara_godot
	@# gdlint reads gdlintrc from its cwd upward, never beside the files, so
	@# it runs in godot/ to see godot/.gdlintrc (from the root it lints at
	@# gdtoolkit's defaults).
	cd godot && $(GDLINT) $(CURDIR)/$(GODOT_ADDON) $(CURDIR)/$(GODOT_DEMO)/*.gd $(CURDIR)/$(GODOT_DEMO)/tests \
	  $(CURDIR)/$(GODOT_SNIPPET)/lingara_snippet.gd
	$(GODOT) --headless --path $(GODOT_DEMO) --script res://tests/snippet_regions_test.gd
	$(GODOT) --headless --path $(GODOT_SNIPPET) --check-only --script res://lingara_snippet.gd

## Every EK case through LingaraEmbed. The recipe is make/conformance.mk's.
conformance-ek-godot: godot-import

godot-ek-harness:
	$(GODOT) --headless --path $(GODOT_DEMO) --script res://tests/ek_runner.gd

## The demo headless against `conformance-server serve` and the --fake token server.
godot-headless: godot-import
	$(CARGO) build -q -p lingara-embed-conformance-server -p lingara_demo_token_server
	GODOT="$(GODOT)" bash godot/ci/run-headless-demo.sh

## The demo in the editor, for a person: start the token server first (godot/demo/README.md).
godot-demo: godot-demo-addon
	$(GODOT) --editor --path $(GODOT_DEMO)

help-godot:
	@echo ""
	@echo "godot (lingara_godot on Rust $(GODOT_TOOLCHAIN); Godot $(GODOT_VERSION) unless GODOT=<binary>):"
	@echo "  make godot-dist RID=<rid>     - One rid's library: $(GODOT_RIDS)"
	@echo "  make check-godot-dist RID=<r> - osx universal; ios-arm64 and android-arm64 arm64"
	@echo "  make godot-addon              - The host's library, the inner loop"
	@echo "  make godot-addon-zip          - The release zip (every rid present)"
	@echo "  make check-godot-addon-zip    - The zip's rules, over a fixture"
	@echo "  make test-godot               - Crate tests, gdlint, the snippet's regions and parse"
	@echo "  make conformance-ek-godot     - Every EK case through LingaraEmbed"
	@echo "  make godot-headless           - The demo's headless test"
	@echo "  make godot-demo               - The demo in the editor"

HELP_SECTIONS += godot
