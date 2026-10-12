# unreal/ — the Unreal Engine 5 plugin LingaraEmbed over ffi/'s header, its
# engine-free harness, and snippets/player/unreal/ (ADR 1.10.26ac).
#
# The line is drawn by one question: does it include an Unreal header?
# Everything that does not is built and tested here on every PR, with no
# engine (`test-unreal`, `conformance-ek-unreal`). Everything that does waits
# for `unreal-smoke`, a scripted manual lane on a machine with the engine
# installed (D10). `unreal-package` stages, copies and stamps: it needs no
# engine, so K8's release workflow runs it.

EK_LANGS += unreal
UNREAL_DIR := unreal
UNREAL_PLUGIN := $(UNREAL_DIR)/LingaraEmbed
UNREAL_THIRD_PARTY := $(UNREAL_PLUGIN)/Source/ThirdParty/LingaraEmbedLibrary
UNREAL_HEADER := $(UNREAL_THIRD_PARTY)/include/lingara_embed.h
UNREAL_HARNESS := $(UNREAL_DIR)/harness
UNREAL_BUILD := target/unreal-harness
UNREAL_SAN_BUILD := target/unreal-harness-san
UNREAL_DIST := dist/unreal
UNREAL_VERSIONS := $(UNREAL_DIR)/engine-versions.txt
# K3's host library, built debug by `ffi-host` (make/dotnet.mk) into target/debug/.
UNREAL_LIB_DIR := $(CURDIR)/target/debug
CMAKE ?= cmake
CLANG_TIDY ?= clang-tidy
# ASan and UBSan where the toolchain carries them (Linux and macOS, D10).
UNREAL_SANITIZE ?= $(if $(filter Windows_NT,$(OS)),0,1)

# The fixture's binary; conformance-server runs it from the repository root.
CONFORMANCE_HARNESS_ek_unreal = $(UNREAL_BUILD)/bin/ek_fixture$(if $(filter Windows_NT,$(OS)),.exe,)

.PHONY: codegen-unreal check-codegen-unreal unreal-harness-build test-unreal unreal-harness-tests \
        unreal-sanitize unreal-tidy check-unreal-plugin check-unreal-coverage unreal-tools-tests unreal-stage unreal-package \
        unreal-smoke help-unreal

## The plugin's copy of K3's header, committed so a checkout compiles with no Rust step.
codegen-unreal:
	cp ffi/include/lingara_embed.h $(UNREAL_HEADER)

check-codegen-unreal:
	@cmp -s ffi/include/lingara_embed.h $(UNREAL_HEADER) \
	  || { echo "✗ $(UNREAL_HEADER) differs from ffi/include/lingara_embed.h: run make codegen-unreal"; exit 1; }
	@echo "✓ the plugin's header is ffi/'s"

## The wrapper suites (C++17 and C++20) and the EK fixture, against the host library.
unreal-harness-build: ffi-host
	$(CMAKE) -S $(UNREAL_HARNESS) -B $(UNREAL_BUILD) -DCMAKE_BUILD_TYPE=Debug -DLINGARA_EMBED_LIB_DIR="$(UNREAL_LIB_DIR)"
	$(CMAKE) --build $(UNREAL_BUILD) --config Debug

unreal-harness-tests: unreal-harness-build
	$(UNREAL_BUILD)/bin/wrapper_tests_cxx17
	$(UNREAL_BUILD)/bin/wrapper_tests_cxx20

## The suites again under AddressSanitizer and UndefinedBehaviorSanitizer (AC6).
unreal-sanitize: ffi-host
	@if [ "$(UNREAL_SANITIZE)" = 1 ]; then \
	  $(CMAKE) -S $(UNREAL_HARNESS) -B $(UNREAL_SAN_BUILD) -DCMAKE_BUILD_TYPE=Debug -DLINGARA_SANITIZE=ON -DLINGARA_EMBED_LIB_DIR="$(UNREAL_LIB_DIR)" \
	  && $(CMAKE) --build $(UNREAL_SAN_BUILD) --config Debug --target wrapper_tests_cxx17 \
	  && $(UNREAL_SAN_BUILD)/bin/wrapper_tests_cxx17; \
	else echo "- sanitizers: not on this host (Windows)"; fi

## clang-tidy at K3's numbers (50 lines, 5 parameters, complexity 10) over the wrapper and harness.
unreal-tidy: unreal-harness-build
	$(CLANG_TIDY) --quiet -p $(UNREAL_BUILD) $(if $(filter Darwin,$(shell uname -s)),--extra-arg=-isysroot$(shell xcrun --show-sdk-path),) \
	  $(UNREAL_HARNESS)/wrapper_tests.cpp $(UNREAL_HARNESS)/header_tests.cpp $(UNREAL_HARNESS)/fixture/*.cpp

## The descriptor, the settings header, lib/'s tracked files and the snippet (AC11).
check-unreal-plugin:
	bash $(UNREAL_DIR)/tools/check_plugin.sh

## Every event tag has a literal in the decoder, every command tag in the encoder (AC10).
check-unreal-coverage:
	bash $(UNREAL_DIR)/tools/check_coverage.sh

## The three tools' own tests (AC10, AC11, AC16) and the smoke script's dry run.
unreal-tools-tests:
	bash $(UNREAL_DIR)/tools/check_coverage_test.sh
	bash $(UNREAL_DIR)/tools/check_plugin_test.sh
	bash $(UNREAL_DIR)/tools/package_test.sh
	bash $(UNREAL_DIR)/tools/smoke.sh --dry-run /no-engine 5.6 > /dev/null

## Everything engine-free: the harness, the sanitizers, the plugin and coverage checks and their tests.
test-unreal: unreal-harness-tests unreal-sanitize check-codegen-unreal check-unreal-plugin check-unreal-coverage unreal-tools-tests

## Every EK case through the C++ fixture. The recipe is make/conformance.mk's.
conformance-ek-unreal: unreal-harness-build

## dist/ffi/<rid>/ into the plugin's lib/<Platform>/, for every rid that has been built.
unreal-stage:
	bash $(UNREAL_DIR)/tools/package.sh stage $(FFI_DIST) $(UNREAL_THIRD_PARTY)/lib

## One zip per line of engine-versions.txt (or UE=5.<m>), at dist/unreal/ (D9).
unreal-package:
	bash $(UNREAL_DIR)/tools/package.sh package $(FFI_DIST) $(UNREAL_DIST) $(UNREAL_VERSIONS) $(UE)

## The scripted manual engine smoke, on a machine with the engine (D10).
unreal-smoke:
	@test -n "$(UE_ROOT)" -a -n "$(UE)" || { echo "✗ usage: make unreal-smoke UE_ROOT=<engine> UE=5.<m>"; exit 2; }
	bash $(UNREAL_DIR)/tools/smoke.sh "$(UE_ROOT)" "$(UE)"

help-unreal:
	@echo ""
	@echo "unreal (LingaraEmbed, the UE5 plugin; no engine needed unless named):"
	@echo "  make test-unreal              - Wrapper suites (+sanitizers), header, plugin and coverage checks"
	@echo "  make unreal-tidy              - clang-tidy over the wrapper and the harness"
	@echo "  make conformance-ek-unreal    - Every EK case through the C++ fixture"
	@echo "  make codegen-unreal           - Copy ffi/'s header into the plugin"
	@echo "  make unreal-stage             - dist/ffi/<rid>/ into the plugin's lib/"
	@echo "  make unreal-package [UE=5.m]  - One zip per supported engine version into $(UNREAL_DIST)/"
	@echo "  make unreal-smoke UE_ROOT= UE= - The manual engine smoke (needs Unreal Engine)"

HELP_SECTIONS += unreal
