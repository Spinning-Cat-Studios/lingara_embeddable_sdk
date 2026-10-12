# ffi/ — lingara-embed-ffi, the C ABI over lingara-embed (ADR 1.10.26aa).
#
# Not a binding, so it joins neither EK_LANGS nor ES_LANGS: dotnet (D8) and
# unreal (K5) consume it, and make/dotnet.mk's codegen targets run the header
# and schema targets here. The committed header and schema are written by the
# crate's own tests (LINGARA_EMBED_BLESS=1), so no cbindgen CLI is needed.
#
# `ffi-dist RID=<rid>` builds one of D7's five platforms into dist/ffi/<rid>/
# with the named ffi-release profile; K8's release matrix runs it per host.

FFI_PKG := lingara-embed-ffi
FFI_PROFILE ?= ffi-release
FFI_DIST := dist/ffi
FFI_HEADER := ffi/include/lingara_embed.h
FFI_RIDS := win-x64 osx linux-x64 android-arm64 ios-arm64
# Target-scoped flags, so the host's own build cache is never invalidated.
FFI_ANDROID_RUSTFLAGS := -C link-arg=-Wl,-z,max-page-size=16384

.PHONY: ffi-header check-ffi-header ffi-schema check-ffi-schema ffi-dist ffi-deps-check test-ffi help-ffi

## Rewrite include/lingara_embed.h from cbindgen over ffi/src.
ffi-header:
	LINGARA_EMBED_BLESS=1 $(CARGO) test -q -p $(FFI_PKG) --lib header_tests

## The committed header is cbindgen's output (AC10).
check-ffi-header:
	$(CARGO) test -q -p $(FFI_PKG) --lib header_tests

## Rewrite schema/messages.schema.json from schemars over K1's types.
ffi-schema:
	LINGARA_EMBED_BLESS=1 $(CARGO) test -q -p $(FFI_PKG) --lib schema_tests

## The committed schema is the core's message set (AC11).
check-ffi-schema:
	$(CARGO) test -q -p $(FFI_PKG) --lib schema_tests

## The crate's tests, then the staticlib and the C smoke over the header
## alone (AC12). The smoke is --ignored in a plain `cargo test`; here it
## gets its two variables and fails, never skips, without them.
FFI_HOST_LIB = $(if $(filter Windows_NT,$(OS)),target/debug/lingara_embed.lib,target/debug/liblingara_embed.a)
test-ffi:
	$(CARGO) clippy -p $(FFI_PKG) --all-targets -- -D warnings
	$(CARGO) test -p $(FFI_PKG)
	@# --color never: under CARGO_TERM_COLOR=always (CI) rustc's note ends in
	@# an ANSI reset glued to the last library, which no linker can find.
	@libs=$$($(CARGO) rustc -q --color never -p $(FFI_PKG) --lib --crate-type staticlib -- --print native-static-libs 2>&1 \
	    | sed -n 's/.*native-static-libs: //p' | tail -1); \
	  echo "native-static-libs: $$libs"; \
	  LINGARA_EMBED_STATICLIB="$(CURDIR)/$(FFI_HOST_LIB)" LINGARA_EMBED_NATIVE_LIBS="$$libs" \
	    $(CARGO) test -p $(FFI_PKG) --test c_smoke_tests -- --ignored

## One platform into dist/ffi/<rid>/: the libraries and the header (D7).
ffi-dist:
	@case " $(FFI_RIDS) " in *" $(RID) "*) ;; *) echo "RID must be one of: $(FFI_RIDS)"; exit 2;; esac
	@rm -rf $(FFI_DIST)/$(RID) && mkdir -p $(FFI_DIST)/$(RID)
	$(MAKE) --no-print-directory ffi-dist-$(RID)
	cp $(FFI_HEADER) $(FFI_DIST)/$(RID)/
	@ls -l $(FFI_DIST)/$(RID)

ffi-dist-win-x64:
	CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS="-C target-feature=+crt-static" \
	  $(CARGO) build -p $(FFI_PKG) --profile $(FFI_PROFILE) --target x86_64-pc-windows-msvc
	cp target/x86_64-pc-windows-msvc/$(FFI_PROFILE)/lingara_embed.dll \
	   target/x86_64-pc-windows-msvc/$(FFI_PROFILE)/lingara_embed.dll.lib \
	   target/x86_64-pc-windows-msvc/$(FFI_PROFILE)/lingara_embed.lib $(FFI_DIST)/win-x64/

ffi-dist-osx:
	MACOSX_DEPLOYMENT_TARGET=11.0 $(CARGO) build -p $(FFI_PKG) --profile $(FFI_PROFILE) --target aarch64-apple-darwin
	MACOSX_DEPLOYMENT_TARGET=11.0 $(CARGO) build -p $(FFI_PKG) --profile $(FFI_PROFILE) --target x86_64-apple-darwin
	lipo -create -output $(FFI_DIST)/osx/liblingara_embed.dylib \
	  target/aarch64-apple-darwin/$(FFI_PROFILE)/liblingara_embed.dylib \
	  target/x86_64-apple-darwin/$(FFI_PROFILE)/liblingara_embed.dylib
	install_name_tool -id @rpath/liblingara_embed.dylib $(FFI_DIST)/osx/liblingara_embed.dylib

# glibc 2.28 floor, from zig's sysroot; ffi-deps-check verifies it on the artefact.
ffi-dist-linux-x64:
	$(CARGO) zigbuild -p $(FFI_PKG) --profile $(FFI_PROFILE) --target x86_64-unknown-linux-gnu.2.28
	cp target/x86_64-unknown-linux-gnu/$(FFI_PROFILE)/liblingara_embed.so $(FFI_DIST)/linux-x64/

ffi-dist-android-arm64:
	CARGO_TARGET_AARCH64_LINUX_ANDROID_RUSTFLAGS="$(FFI_ANDROID_RUSTFLAGS)" \
	  $(CARGO) ndk -t arm64-v8a --platform 23 build -p $(FFI_PKG) --profile $(FFI_PROFILE)
	cp target/aarch64-linux-android/$(FFI_PROFILE)/liblingara_embed.so $(FFI_DIST)/android-arm64/

# Static: an iOS app may not load a loose dylib.
ffi-dist-ios-arm64:
	IPHONEOS_DEPLOYMENT_TARGET=13.0 $(CARGO) build -p $(FFI_PKG) --profile $(FFI_PROFILE) --target aarch64-apple-ios
	cp target/aarch64-apple-ios/$(FFI_PROFILE)/liblingara_embed.a $(FFI_DIST)/ios-arm64/

## Each dist/ffi/<rid>/ present imports only its allowlisted libraries, and
## linux-x64 needs no GLIBC_ above 2.28 (AC28). RID=<rid> checks one.
ffi-deps-check:
	bash tools/ffi-deps/check_test.sh
	@for rid in $(or $(RID),$(FFI_RIDS)); do \
	  if [ -d $(FFI_DIST)/$$rid ]; then bash tools/ffi-deps/check.sh $$rid $(FFI_DIST)/$$rid || exit 1; \
	  elif [ -n "$(RID)" ]; then echo "✗ no $(FFI_DIST)/$$rid: run make ffi-dist RID=$$rid"; exit 1; \
	  else echo "- $$rid: not built here"; fi; \
	done

help-ffi:
	@echo ""
	@echo "ffi (lingara-embed-ffi, the C ABI):"
	@echo "  make test-ffi                - clippy, the crate's tests, and the C smoke over the header"
	@echo "  make ffi-header / ffi-schema - Rewrite the committed header / message schema"
	@echo "  make check-ffi-header        - The header is cbindgen's output"
	@echo "  make check-ffi-schema        - The schema is the core's message set"
	@echo "  make ffi-dist RID=<rid>      - One platform into $(FFI_DIST)/<rid>/: $(FFI_RIDS)"
	@echo "  make ffi-deps-check [RID=]   - Every built artefact imports only its allowlist"

HELP_SECTIONS += ffi
