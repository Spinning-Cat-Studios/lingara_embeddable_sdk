# cpp/ — the C++ channels over ffi/'s release zips (ADR 8.10.26q).
#
# A vcpkg overlay port and a Conan 2 recipe in each index's own layout, both
# templates that cpp/tools/stamp-ports.sh fills from the five
# lingara-embed-ffi zips. Nothing here builds from source: the port and the
# recipe repackage the bytes K3's ffi-dist built and K8's release checked.
#
# `test-cpp` is D6's harness: after `make ffi-dist RID=<host rid>`, one
# engine-free consumer is built three ways (the overlay port, Conan, the
# plain zip) plus a C-only consumer, then clang-tidy reads the C++ one. The
# host zip is real; the other four rids are fakes the stamper only hashes.

CMAKE ?= cmake
CLANG_TIDY ?= clang-tidy
CPP_DIR := cpp
CPP_BUILD := target/cpp-harness
# CPP_BUILD for cmd.exe, its backslashes doubled for the recipe's double quotes.
CPP_BUILD_CMD = $(subst /,\\,$(CPP_BUILD))
CPP_ZIPS := $(CPP_BUILD)/zips
CPP_STAMPED := $(CPP_BUILD)/stamped
CPP_PREFIX := $(CPP_BUILD)/zip-root
CPP_DOCTEST := $(CURDIR)/unreal/harness/third_party
CPP_PY = "$$(command -v python3 || command -v python)"
CPP_HOST_RID := $(if $(filter Windows_NT,$(OS)),win-x64,$(if $(filter Darwin,$(shell uname -s)),osx,linux-x64))
CPP_EXE := $(if $(filter Windows_NT,$(OS)),.exe,)
# The consumers build Release: a Debug build resolves the same release-only
# library (D2), which the configuration-less IMPORTED_LOCATION guarantees.
CPP_CONFIGURE = -DCMAKE_BUILD_TYPE=Release -DLINGARA_DOCTEST_DIR="$(CPP_DOCTEST)"

# The tools the vcpkg and Conan legs run, pinned (D6). VCPKG_COMMIT is any
# commit-ish of microsoft/vcpkg; a release tag is one.
VCPKG_COMMIT := 2025.06.13
CPP_VCPKG_ROOT ?= $(CURDIR)/$(CPP_BUILD)/vcpkg
VCPKG_TRIPLETS ?= $(if $(filter Windows_NT,$(OS)),x64-windows x64-windows-static-md,default)
# 2.15.0 predates Visual Studio 18 (msvc 195): on a windows-latest image that
# has no older VS, its detect fell through to MinGW's gcc 15 on PATH.
CONAN_VERSION := 2.33.0
CONAN_VENV := $(CPP_BUILD)/conan-venv
CONAN = $(CURDIR)/$(CONAN_VENV)/$(if $(filter Windows_NT,$(OS)),Scripts/conan.exe,bin/conan)
CONAN_ENV = CONAN_HOME="$(CURDIR)/$(CPP_BUILD)/conan-home"

# The release (D5): what release-cpp-ports in make/release.mk runs.
CPP_PORTS_BUILD := target/cpp-ports
CPP_URL_BASE ?= https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/releases/download/$(RELEASE_TAG)
CPP_PORTS_OUT ?= $(ZIPS)
CPP_RELEASE_VERSION = $(patsubst v%,%,$(RELEASE_TAG))

.PHONY: cpp-ports test-cpp test-cpp-tools cpp-harness-stage test-cpp-zip test-cpp-c test-cpp-vcpkg \
        test-cpp-conan check-cpp-tidy help-cpp

## The five ffi zips in ZIPS checked, then both trees stamped and zipped beside them.
cpp-ports:
	@test -n "$(ZIPS)" || { echo "Usage: make cpp-ports ZIPS=<dir of the five ffi zips> [TAG=v…]" >&2; exit 2; }
	@for rid in $(FFI_RIDS); do \
	  bash $(CPP_DIR)/tools/check-ffi-zip.sh $(ZIPS)/lingara-embed-ffi-$(RELEASE_TAG)-$$rid.zip $$rid || exit 1; done
	bash $(CPP_DIR)/tools/stamp-ports.sh --version $(CPP_RELEASE_VERSION) --zips $(ZIPS) \
	  --url-base $(CPP_URL_BASE) --out $(CPP_PORTS_BUILD)
	@rm -f $(CPP_PORTS_OUT)/lingara-embed-vcpkg-port-$(RELEASE_TAG).zip $(CPP_PORTS_OUT)/lingara-embed-conan-recipe-$(RELEASE_TAG).zip
	cd $(CPP_PORTS_BUILD)/vcpkg && $(CPP_PY) -m zipfile -c $(abspath $(CPP_PORTS_OUT))/lingara-embed-vcpkg-port-$(RELEASE_TAG).zip ports
	cd $(CPP_PORTS_BUILD)/conan && $(CPP_PY) -m zipfile -c $(abspath $(CPP_PORTS_OUT))/lingara-embed-conan-recipe-$(RELEASE_TAG).zip recipes
	@echo "✓ $(CPP_PORTS_OUT)/lingara-embed-{vcpkg-port,conan-recipe}-$(RELEASE_TAG).zip"

## D6 step 0: stamp-ports.sh and check-ffi-zip.sh against fixtures, offline (AC1, AC2).
test-cpp-tools:
	bash $(CPP_DIR)/tools/stamp_ports_test.sh
	bash $(CPP_DIR)/tools/check_ffi_zip_test.sh
	bash $(CPP_DIR)/tools/release_ports_test.sh

# D6 steps 1-2: the host's real zip, checked; four fake ones; both trees
# stamped against file:// URLs; the host zip unzipped for the plain legs.
cpp-harness-stage:
	@rm -rf $(CPP_ZIPS) $(CPP_STAMPED) $(CPP_PREFIX) && mkdir -p $(CPP_ZIPS) $(CPP_PREFIX)
	$(MAKE) --no-print-directory release-ffi-zip RID=$(CPP_HOST_RID) FFI_ZIP_OUT=$(CPP_ZIPS)
	bash $(CPP_DIR)/tools/check-ffi-zip.sh $(CPP_ZIPS)/lingara-embed-ffi-$(RELEASE_TAG)-$(CPP_HOST_RID).zip $(CPP_HOST_RID)
	@for rid in $(filter-out $(CPP_HOST_RID),$(FFI_RIDS)); do \
	  echo "fake $$rid" > $(CPP_ZIPS)/fake-$$rid.txt && \
	  (cd $(CPP_ZIPS) && $(CPP_PY) -m zipfile -c lingara-embed-ffi-$(RELEASE_TAG)-$$rid.zip fake-$$rid.txt) || exit 1; done
	@# A Windows path becomes file:///C:/…; a POSIX one file:///….
	@dir=$$(cd $(CPP_ZIPS) && { pwd -W 2>/dev/null || pwd; }); case "$$dir" in /*) url="file://$$dir";; *) url="file:///$$dir";; esac; \
	  bash $(CPP_DIR)/tools/stamp-ports.sh --version $(RELEASE_VERSION) --zips $(CPP_ZIPS) --url-base "$$url" --out $(CPP_STAMPED)
	$(CPP_PY) -m zipfile -e $(CPP_ZIPS)/lingara-embed-ffi-$(RELEASE_TAG)-$(CPP_HOST_RID).zip $(CPP_PREFIX)

## D6 step 5: the consumer with CMAKE_PREFIX_PATH at the unzipped host zip (AC6).
test-cpp-zip: cpp-harness-stage
	$(CMAKE) -S $(CPP_DIR)/harness/consumer -B $(CPP_BUILD)/zip $(CPP_CONFIGURE) -DCMAKE_PREFIX_PATH="$(CURDIR)/$(CPP_PREFIX)"
	$(CMAKE) --build $(CPP_BUILD)/zip --config Release
	$(CPP_BUILD)/zip/bin/consumer$(CPP_EXE)

## D6 step 6: a C-only project links lingara::embed and reports VERSION (AC7).
test-cpp-c: cpp-harness-stage
	$(CMAKE) -S $(CPP_DIR)/harness/consumer-c -B $(CPP_BUILD)/c -DCMAKE_BUILD_TYPE=Release \
	  -DCMAKE_PREFIX_PATH="$(CURDIR)/$(CPP_PREFIX)" -DLINGARA_EXPECTED_VERSION=$(RELEASE_VERSION)
	$(CMAKE) --build $(CPP_BUILD)/c --config Release
	$(CPP_BUILD)/c/bin/consumer_c$(CPP_EXE)

$(CPP_VCPKG_ROOT)/.vcpkg-root:
	git clone --quiet https://github.com/microsoft/vcpkg.git $(CPP_VCPKG_ROOT)
	git -C $(CPP_VCPKG_ROOT) checkout --quiet $(VCPKG_COMMIT)
	cd $(CPP_VCPKG_ROOT) && $(if $(filter Windows_NT,$(OS)),cmd //c bootstrap-vcpkg.bat -disableMetrics,sh bootstrap-vcpkg.sh -disableMetrics)

## D6 step 3: the pinned vcpkg installs the stamped overlay port in manifest
## mode, per triplet (x64-windows-static-md too on Windows), and the smoke runs (AC3).
test-cpp-vcpkg: cpp-harness-stage $(CPP_VCPKG_ROOT)/.vcpkg-root
	@for triplet in $(VCPKG_TRIPLETS); do \
	  build=$(CPP_BUILD)/vcpkg-$$triplet; \
	  $(CMAKE) -S $(CPP_DIR)/harness/consumer -B $$build $(CPP_CONFIGURE) \
	    -DCMAKE_TOOLCHAIN_FILE="$(CPP_VCPKG_ROOT)/scripts/buildsystems/vcpkg.cmake" \
	    -DVCPKG_OVERLAY_PORTS="$(CURDIR)/$(CPP_STAMPED)/vcpkg/ports" \
	    $$([ "$$triplet" = default ] || echo "-DVCPKG_TARGET_TRIPLET=$$triplet") || exit 1; \
	  $(CMAKE) --build $$build --config Release || exit 1; \
	  echo "→ vcpkg $$triplet"; $$build/bin/consumer$(CPP_EXE) || exit 1; \
	done

$(CONAN):
	$(CPP_PY) -m venv $(CONAN_VENV)
	$(CONAN_VENV)/$(if $(filter Windows_NT,$(OS)),Scripts,bin)/python -m pip install --quiet conan==$(CONAN_VERSION)

## D6 step 4: conan create runs the recipe's test_package (AC4), then the
## consumer builds from conanfile.txt with CMakeDeps and the smoke runs (AC5).
test-cpp-conan: cpp-harness-stage $(CONAN)
	$(CONAN_ENV) $(CONAN) profile detect --force
	@# test_package builds in a temp dir: under the recipe tree, MSBuild's
	@# tracking logs pass Windows' 260-character MAX_PATH (FTK1011).
	$(CONAN_ENV) $(CONAN) create $(CPP_STAMPED)/conan/recipes/lingara-embed/all --version $(RELEASE_VERSION) --build=missing \
	  -c 'tools.cmake.cmake_layout:test_folder=$$TMP'
	@# user_presets off: CMakeUserPresets.json would land in the source tree.
	$(CONAN_ENV) $(CONAN) install $(CPP_DIR)/harness/consumer/conanfile.txt --output-folder $(CPP_BUILD)/conan-deps \
	  -s build_type=Release -c tools.cmake.cmaketoolchain:user_presets=
	$(CMAKE) -S $(CPP_DIR)/harness/consumer -B $(CPP_BUILD)/conan $(CPP_CONFIGURE) \
	  -DCMAKE_TOOLCHAIN_FILE="$(CURDIR)/$(CPP_BUILD)/conan-deps/conan_toolchain.cmake"
	$(CMAKE) --build $(CPP_BUILD)/conan --config Release
	@# conanrun puts the library on the loader's path, as a Conan user's run does.
	@# cmd reads a `/` as a switch, so its paths are backslashes throughout.
	$(if $(filter Windows_NT,$(OS)),cmd //c "$(CPP_BUILD_CMD)\\conan-deps\\conanrun.bat && $(CPP_BUILD_CMD)\\conan\\bin\\consumer.exe",\
	  . $(CPP_BUILD)/conan-deps/conanrun.sh && $(CPP_BUILD)/conan/bin/consumer)

## D6 step 7: clang-tidy at K5's numbers over the C++ consumer (the plain-zip build's database).
# A configure of its own: the Visual Studio generators write no
# compile_commands.json, so Windows takes Ninja (the dev env's) for this one.
check-cpp-tidy: test-cpp-zip
	$(CMAKE) -S $(CPP_DIR)/harness/consumer -B $(CPP_BUILD)/tidy $(CPP_CONFIGURE) -DCMAKE_PREFIX_PATH="$(CURDIR)/$(CPP_PREFIX)" \
	  $(if $(filter Windows_NT,$(OS)),-G Ninja -DCMAKE_CXX_COMPILER=cl,)
	$(CLANG_TIDY) --quiet --config-file=unreal/harness/.clang-tidy -p $(CPP_BUILD)/tidy \
	  $(if $(filter Darwin,$(shell uname -s)),--extra-arg=-isysroot$(shell xcrun --show-sdk-path),) \
	  $(CPP_DIR)/harness/consumer/main.cpp

## D6 steps 0-7, after make ffi-dist RID=<host rid>.
test-cpp: test-cpp-tools test-cpp-zip test-cpp-c test-cpp-vcpkg test-cpp-conan check-cpp-tidy

help-cpp:
	@echo ""
	@echo "cpp (the vcpkg port and Conan recipe over the ffi zips):"
	@echo "  make test-cpp          - Every leg below, after make ffi-dist RID=$(CPP_HOST_RID)"
	@echo "  make test-cpp-tools    - stamp-ports.sh and check-ffi-zip.sh against fixtures"
	@echo "  make test-cpp-zip      - The consumer over the unzipped host zip"
	@echo "  make test-cpp-c        - A C-only consumer of lingara::embed"
	@echo "  make test-cpp-vcpkg    - The stamped overlay port, vcpkg $(VCPKG_COMMIT)"
	@echo "  make test-cpp-conan    - The stamped recipe's test_package and CMakeDeps, Conan $(CONAN_VERSION)"
	@echo "  make check-cpp-tidy    - clang-tidy over the C++ consumer"
	@echo "  make cpp-ports ZIPS=<d> - Stamp and zip both trees over a release's five ffi zips"

HELP_SECTIONS += cpp
