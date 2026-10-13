import os

from conan import ConanFile
from conan.errors import ConanInvalidConfiguration
from conan.tools.files import copy, get, save
from conan.tools.layout import basic_layout

required_conan_version = ">=2.2"

# (os, arch) -> the Release's platform id (ADR 1.10.26aa D7). One universal
# library serves both macOS architectures.
_RIDS = {
    ("Windows", "x86_64"): "win-x64",
    ("Macos", "armv8"): "osx",
    ("Macos", "x86_64"): "osx",
    ("Linux", "x86_64"): "linux-x64",
    ("Android", "armv8"): "android-arm64",
    ("iOS", "armv8"): "ios-arm64",
}

# CMakeDeps carries no compile features, so lingara::embed_cpp gets its
# C++17 requirement from a build module.
_CXX17_MODULE = "set_property(TARGET lingara::embed_cpp APPEND PROPERTY INTERFACE_COMPILE_FEATURES cxx_std_17)\n"


class LingaraEmbedConan(ConanFile):
    name = "lingara-embed"
    description = ("Lingara inside a game: the C ABI (lingara_embed.h) and its "
                   "header-only C++17 wrapper (lingara_embed.hpp), prebuilt")
    license = "MIT"
    url = "https://github.com/conan-io/conan-center-index"
    homepage = "https://github.com/Spinning-Cat-Studios/lingara_embeddable_sdk"
    topics = ("lingara", "language-learning", "games", "pre-built")
    # Desktop and Android are dynamic and iOS is static: the linkage is the
    # Release's, so there is no shared or fPIC option (ADR 8.10.26q D2).
    package_type = "shared-library"
    settings = "os", "arch", "compiler", "build_type"

    @property
    def _rid(self):
        return _RIDS.get((str(self.settings.os), str(self.settings.arch)))

    def configure(self):
        if self.settings.os == "iOS":
            self.package_type = "static-library"

    def layout(self):
        basic_layout(self, src_folder="src")

    def package_id(self):
        # One release build serves every compiler and configuration: the C ABI
        # carries no CRT or STL type.
        del self.info.settings.compiler
        del self.info.settings.build_type

    def validate(self):
        if self._rid is None:
            raise ConanInvalidConfiguration(
                f"{self.ref} ships no library for {self.settings.os} {self.settings.arch}")
        if self.settings.os == "iOS" and self.settings.get_safe("os.sdk") == "iphonesimulator":
            raise ConanInvalidConfiguration(f"{self.ref} ships no iOS simulator library")

    def build(self):
        # Prebuilt: the source is the Release zip for this platform.
        get(self, **self.conan_data["sources"][self.version][self._rid], destination=self.build_folder)

    def package(self):
        # The zip's own share/lingara-embed/ CMake package is left out:
        # CMakeDeps generates the config (package_info below).
        copy(self, "LICENSE", self.build_folder, os.path.join(self.package_folder, "licenses"))
        copy(self, "*.h*", os.path.join(self.build_folder, "include"), os.path.join(self.package_folder, "include"))
        copy(self, "*", os.path.join(self.build_folder, "lib"), os.path.join(self.package_folder, "lib"))
        copy(self, "*.dll", os.path.join(self.build_folder, "bin"), os.path.join(self.package_folder, "bin"))
        save(self, os.path.join(self.package_folder, self._cxx17_module), _CXX17_MODULE)

    @property
    def _cxx17_module(self):
        return os.path.join("lib", "cmake", "lingara-embed-cxx17.cmake")

    def package_info(self):
        self.cpp_info.set_property("cmake_file_name", "lingara-embed")

        embed = self.cpp_info.components["embed"]
        embed.set_property("cmake_target_name", "lingara::embed")
        # Windows links the import library lingara_embed.dll.lib.
        embed.libs = ["lingara_embed.dll" if self.settings.os == "Windows" else "lingara_embed"]
        if self.settings.os == "iOS":
            embed.system_libs = ["iconv"]

        embed_cpp = self.cpp_info.components["embed_cpp"]
        embed_cpp.set_property("cmake_target_name", "lingara::embed_cpp")
        embed_cpp.requires = ["embed"]
        embed_cpp.libdirs = []
        embed_cpp.bindirs = []
        self.cpp_info.set_property("cmake_build_modules", [self._cxx17_module])
