# kotlin/ — the JVM player kit (ADR 1.10.26ae): the UniFFI wrapper
# (kotlin/uniffi, crate lingara-embed-uniffi), com.getlingara:lingara-embed-kotlin,
# and both natives artefacts, which this file builds.
#
# kotlin/ is the Gradle project :kotlin in the repo-root build the JVM kits
# share; java/ is make/java.mk's. The Kotlin suite runs against the host
# library through jna.library.path, as the Lingara Android app's JVM tests do, so every
# test target builds it first (kotlin-natives-host).
#
# `natives-desktop RID=<rid>` and `natives-android` build D5's rows into
# dist/jvm/<rid>/ under JNA's own resource prefixes, with the named
# release-jvm profile. K8's natives matrix (1.10.26ag D3) runs these same
# targets, so a release native is the local native. win-x64 needs MSVC and
# linux-x64 needs cargo-zigbuild, so on a macOS host both are CI's.
#
# The committed bindings are bindgen's output over the host library, with
# --no-format, so a regen is byte-stable and `check-codegen-kotlin` fails on
# any byte of difference: a doc comment in kotlin/uniffi is a checksum input.
#
# conformance-ek-kotlin is never .PHONY, so it takes the recipe of the
# conformance-ek-% pattern rule in make/conformance.mk.

EK_LANGS += kotlin

GRADLE ?= ./gradlew
GRADLE_FLAGS ?= -q --console=plain
UNIFFI_PKG := lingara-embed-uniffi
UNIFFI_BINDGEN := target/debug/lingara-embed-uniffi-bindgen
JVM_PROFILE ?= release-jvm
JVM_DIST := dist/jvm
JVM_RIDS := win-x64 osx linux-x64 android-arm64
JVM_LIB := lingara_embed_uniffi
KOTLIN_BINDINGS := kotlin/src/main/kotlin
KOTLIN_GENERATED := $(KOTLIN_BINDINGS)/com/getlingara/embed/lingara_embed_uniffi.kt
# The host library bindgen reads, by the host's own naming.
HOST_UNIFFI_LIB = $(if $(filter Windows_NT,$(OS)),target/debug/$(JVM_LIB).dll,$(if $(filter Darwin,$(shell uname -s)),target/debug/lib$(JVM_LIB).dylib,target/debug/lib$(JVM_LIB).so))
CONFORMANCE_HARNESS_ek_kotlin := kotlin/conformance/build/install/conformance/bin/conformance

.PHONY: kotlin-natives-host natives-desktop natives-android codegen-kotlin check-codegen-kotlin test-kotlin \
        check-jvm-shape jvm-example kotlin-harness help-kotlin

## The wrapper, its bindgen and the conformance server for this host.
kotlin-natives-host:
	$(CARGO) build -q -p $(UNIFFI_PKG) -p lingara-embed-uniffi-bindgen -p lingara-embed-conformance-server

## One desktop platform into $(JVM_DIST)/<rid>/ (D5): win-x64, osx (both slices) or linux-x64.
natives-desktop:
	@case " win-x64 osx linux-x64 " in *" $(RID) "*) ;; *) echo "RID must be one of: win-x64 osx linux-x64"; exit 2;; esac
	@rm -rf $(JVM_DIST)/$(RID)
	$(MAKE) --no-print-directory natives-desktop-$(RID)
	@find $(JVM_DIST)/$(RID) -type f | xargs ls -l

natives-desktop-win-x64:
	CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static" \
	  $(CARGO) build -p $(UNIFFI_PKG) --profile $(JVM_PROFILE) --target x86_64-pc-windows-msvc
	mkdir -p $(JVM_DIST)/win-x64/win32-x86-64
	cp target/x86_64-pc-windows-msvc/$(JVM_PROFILE)/$(JVM_LIB).dll $(JVM_DIST)/win-x64/win32-x86-64/

# Two thin libraries, not a lipo: JNA's resource prefix is per architecture.
natives-desktop-osx:
	MACOSX_DEPLOYMENT_TARGET=11.0 $(CARGO) build -p $(UNIFFI_PKG) --profile $(JVM_PROFILE) --target aarch64-apple-darwin
	MACOSX_DEPLOYMENT_TARGET=11.0 $(CARGO) build -p $(UNIFFI_PKG) --profile $(JVM_PROFILE) --target x86_64-apple-darwin
	mkdir -p $(JVM_DIST)/osx/darwin-aarch64 $(JVM_DIST)/osx/darwin-x86-64
	cp target/aarch64-apple-darwin/$(JVM_PROFILE)/lib$(JVM_LIB).dylib $(JVM_DIST)/osx/darwin-aarch64/
	cp target/x86_64-apple-darwin/$(JVM_PROFILE)/lib$(JVM_LIB).dylib $(JVM_DIST)/osx/darwin-x86-64/

# glibc 2.28 floor, from zig's sysroot, as K3's ffi-dist-linux-x64.
natives-desktop-linux-x64:
	$(CARGO) zigbuild -p $(UNIFFI_PKG) --profile $(JVM_PROFILE) --target x86_64-unknown-linux-gnu.2.28
	mkdir -p $(JVM_DIST)/linux-x64/linux-x86-64
	cp target/x86_64-unknown-linux-gnu/$(JVM_PROFILE)/lib$(JVM_LIB).so $(JVM_DIST)/linux-x64/linux-x86-64/

## The arm64 library into $(JVM_DIST)/android-arm64/arm64-v8a/: API 23, 16 KB pages (D5).
natives-android:
	@rm -rf $(JVM_DIST)/android-arm64 && mkdir -p $(JVM_DIST)/android-arm64/arm64-v8a
	CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="-C link-arg=-Wl,-z,max-page-size=16384" \
	  $(CARGO) ndk -t arm64-v8a --platform 23 build -p $(UNIFFI_PKG) --profile $(JVM_PROFILE)
	cp target/aarch64-linux-android/$(JVM_PROFILE)/lib$(JVM_LIB).so $(JVM_DIST)/android-arm64/arm64-v8a/
	@ls -l $(JVM_DIST)/android-arm64/arm64-v8a

## Rewrite the committed Kotlin bindings from the host library (D2).
codegen-kotlin: kotlin-natives-host
	$(UNIFFI_BINDGEN) generate --library $(HOST_UNIFFI_LIB) --language kotlin --no-format \
	  --config kotlin/uniffi/uniffi.toml --out-dir $(KOTLIN_BINDINGS)

## The committed bindings equal a fresh regen, byte for byte (AC13).
check-codegen-kotlin: kotlin-natives-host
	@tmp=$$(mktemp -d) && \
	  $(UNIFFI_BINDGEN) generate --library $(HOST_UNIFFI_LIB) --language kotlin --no-format \
	    --config kotlin/uniffi/uniffi.toml --out-dir $$tmp && \
	  if cmp -s $$tmp/com/getlingara/embed/lingara_embed_uniffi.kt $(KOTLIN_GENERATED); then \
	    echo "✓ $(KOTLIN_GENERATED) is current"; rm -rf $$tmp; \
	  else diff -u $(KOTLIN_GENERATED) $$tmp/com/getlingara/embed/lingara_embed_uniffi.kt | head -40; \
	    echo "✗ $(KOTLIN_GENERATED) is stale: run make codegen-kotlin"; rm -rf $$tmp; exit 1; fi

## clippy and the wrapper's tests, then ktlint and the JUnit suite against
## the host library; the harness and the snippets compile and pass the same lint.
test-kotlin: kotlin-natives-host
	$(CARGO) clippy -p $(UNIFFI_PKG) -p lingara-embed-uniffi-bindgen --all-targets -- -D warnings
	$(CARGO) test -p $(UNIFFI_PKG)
	$(GRADLE) $(GRADLE_FLAGS) :kotlin:check :kotlin:conformance:check :snippets:player:kotlin:check

## The four artefacts as a game downloads them (AC8, AC10, AC11): needs all
## five libraries in $(JVM_DIST) and an Android SDK, so it is CI's jvm job.
check-jvm-shape:
	@for d in win-x64/win32-x86-64 osx/darwin-aarch64 osx/darwin-x86-64 linux-x64/linux-x86-64 android-arm64/arm64-v8a; do \
	  [ -d $(JVM_DIST)/$$d ] || { echo "✗ no $(JVM_DIST)/$$d: build it with make natives-desktop / natives-android"; exit 1; }; done
	$(GRADLE) $(GRADLE_FLAGS) -Dmaven.repo.local=$(CURDIR)/build/m2 :kotlin:shape:test

## The libGDX example's three modules, through the root wrapper (D8).
jvm-example:
	$(GRADLE) $(GRADLE_FLAGS) -p kotlin/examples/libgdx build

kotlin-harness: kotlin-natives-host
	$(GRADLE) $(GRADLE_FLAGS) :kotlin:conformance:installDist

## Every EK case through LingaraEmbed. No recipe, and not .PHONY (see the header).
conformance-ek-kotlin: kotlin-harness

help-kotlin:
	@echo ""
	@echo "kotlin (com.getlingara:lingara-embed-kotlin and both natives):"
	@echo "  make test-kotlin                 - clippy, the wrapper's tests, ktlint and JUnit on the host library"
	@echo "  make codegen-kotlin              - Rewrite the committed UniFFI bindings"
	@echo "  make check-codegen-kotlin        - The committed bindings equal a fresh regen"
	@echo "  make natives-desktop RID=<rid>   - One desktop platform into $(JVM_DIST)/<rid>/: win-x64 osx linux-x64"
	@echo "  make natives-android             - The arm64 library into $(JVM_DIST)/android-arm64/"
	@echo "  make check-jvm-shape             - The four artefacts' POMs and libraries (all five natives built)"
	@echo "  make jvm-example                 - Build the libGDX example"
	@echo "  make conformance-ek-kotlin       - Every EK case through the Kotlin kit"

HELP_SECTIONS += kotlin
