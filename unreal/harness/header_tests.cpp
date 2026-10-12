// The plugin's copy of K3's header is K3's header (ADR 1.10.26ac D3). The
// copy is committed so a source checkout compiles with no Rust step, and
// `make codegen-unreal` rewrites it; this test is the per-PR half of that
// pair, `check-codegen-unreal` the other.
#include "doctest.h"

#include <fstream>
#include <iterator>
#include <string>

namespace {

std::string read_all(const char* path) {
    std::ifstream in(path, std::ios::binary);
    return std::string(std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>());
}

}  // namespace

// 1.10.26ac AC7
TEST_CASE("the plugin header is the ffi header") {
    const std::string ffi = read_all(LINGARA_FFI_HEADER);
    const std::string plugin = read_all(LINGARA_PLUGIN_HEADER);
    REQUIRE_FALSE(ffi.empty());
    CHECK(plugin == ffi);
}
