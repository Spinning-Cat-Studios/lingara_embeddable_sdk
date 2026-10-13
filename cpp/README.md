# lingara-embed for C and C++

The C ABI (`lingara_embed.h`) and its header-only C++17 wrapper
(`lingara_embed.hpp`), prebuilt for five platforms and installable four ways.
Unreal Engine users want the plugin under `unreal/` instead.

Every way gives the same CMake package:

```cmake
find_package(lingara-embed CONFIG REQUIRED)
target_link_libraries(game PRIVATE lingara::embed_cpp)   # C++17: lingara_embed.hpp
# or, from C:
target_link_libraries(game PRIVATE lingara::embed)       # lingara_embed.h
```

`lingara::embed` needs no C++ at all; `lingara::embed_cpp` adds the wrapper and
requires C++17.

| Platform | Library | vcpkg triplets | Conan settings |
|---|---|---|---|
| Windows x64 | `lingara_embed.dll` (dynamic) | `x64-windows`, `x64-windows-static-md` | `os=Windows arch=x86_64` |
| macOS (universal) | `liblingara_embed.dylib` (dynamic) | `arm64-osx`, `x64-osx` | `os=Macos arch=armv8\|x86_64` |
| Linux x64 (glibc ≥ 2.28) | `liblingara_embed.so` (dynamic) | `x64-linux` | `os=Linux arch=x86_64` |
| Android arm64 (API ≥ 23) | `liblingara_embed.so` (dynamic) | `arm64-android` | `os=Android arch=armv8` |
| iOS arm64, device only | `liblingara_embed.a` (static) | `arm64-ios` | `os=iOS arch=armv8` |

The linkage is the library's, not the triplet's: a static triplet gets the
DLL on Windows (`x64-windows-static` is refused; use `x64-windows-static-md`),
and there is no iOS simulator library. One release build serves Debug and
Release alike, because no CRT or STL type crosses the C ABI.

**Tokens.** A game mints native tokens on its server **without `origin`**:
an origin-bound token is for the web widget, and a native client sends none.

## 1. vcpkg (overlay port)

Each GitHub Release carries `lingara-embed-vcpkg-port-v<version>.zip`. Unzip it
and point vcpkg at its `ports/`:

```sh
unzip lingara-embed-vcpkg-port-v<version>.zip -d lingara-ports
vcpkg install lingara-embed --overlay-ports=lingara-ports/ports
# or, in manifest mode: "dependencies": ["lingara-embed"] and VCPKG_OVERLAY_PORTS=lingara-ports/ports
```

## 2. Conan 2 (local-recipes-index)

Each Release carries `lingara-embed-conan-recipe-v<version>.zip`, a tree in
ConanCenter's layout. Conan ≥ 2.2 serves it as a remote:

```sh
unzip lingara-embed-conan-recipe-v<version>.zip -d lingara-conan
conan remote add lingara-embed ./lingara-conan --type=local-recipes-index
```

then `requires = "lingara-embed/<version>"` with the `CMakeDeps` and
`CMakeToolchain` generators.

## 3. CPM or FetchContent

The ffi zip carries its own CMake package, so either fetches it directly. Take
the hash from the Release's `checksums.txt`:

```cmake
include(FetchContent)
FetchContent_Declare(lingara_embed
  URL https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk/releases/download/v<version>/lingara-embed-ffi-v<version>-<platform>.zip
  URL_HASH SHA256=<from checksums.txt>)
FetchContent_MakeAvailable(lingara_embed)   # populates only: the zip has no CMakeLists.txt
find_package(lingara-embed CONFIG REQUIRED PATHS ${lingara_embed_SOURCE_DIR} NO_DEFAULT_PATH)
```

With CPM: `CPMAddPackage(NAME lingara_embed URL … DOWNLOAD_ONLY YES)`, then the
same `find_package`.

## 4. The plain zip

Unzip `lingara-embed-ffi-v<version>-<platform>.zip` anywhere and pass that
directory as `CMAKE_PREFIX_PATH`. It holds `include/`, `lib/` (`bin/` for the
DLL), `share/lingara-embed/` and `LICENSE`.

## The curated indexes

`lingara-embed` is not in microsoft/vcpkg or ConanCenter yet, and neither
index lets a name be reserved: it is claimed by the first accepted submission.

- **vcpkg:** a submission is due once the project has six months of public
  releases.
- **ConanCenter:** it asks that recipes build from source, and it has no Rust
  toolchain. Whether it admits a prebuilt recipe over a Rust-built C library
  is an open question there; until it says yes, the local-recipes-index above
  **is** the Conan channel.

Either way the `find_package` name and the targets stay the same.

## Maintainers

`make test-cpp` (after `make ffi-dist RID=<host>`) builds one consumer
through the stamped overlay port, Conan and the plain zip, a C-only consumer,
and runs clang-tidy; `make release-cpp-ports ZIPS=<dir>` stamps and zips both
trees at release (ADR 8.10.26q).
