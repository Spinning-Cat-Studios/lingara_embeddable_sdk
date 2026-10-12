# The Unreal plugin's engine-free harness (ADR 1.10.26ac D10).
#
# Everything here builds without Unreal: lingara_embed.hpp, its doctest
# suites at C++17 and C++20, and the EK fixture that drives the wrapper
# against conformance-server. make/unreal.mk configures it into
# target/unreal-harness/ with LINGARA_EMBED_LIB_DIR pointing at K3's host
# library (`cargo build -p lingara-embed-ffi`, target/debug/).
cmake_minimum_required(VERSION 3.22)
project(lingara_unreal_harness LANGUAGES CXX)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(LINGARA_EMBED_LIB_DIR "" CACHE PATH "Directory holding K3's host library")
option(LINGARA_SANITIZE "AddressSanitizer and UndefinedBehaviorSanitizer" OFF)

if(NOT LINGARA_EMBED_LIB_DIR)
  message(FATAL_ERROR "LINGARA_EMBED_LIB_DIR is not set: run make test-unreal")
endif()

set(REPO_ROOT ${CMAKE_CURRENT_SOURCE_DIR}/../..)
set(PLUGIN_INCLUDE ${REPO_ROOT}/unreal/LingaraEmbed/Source/ThirdParty/LingaraEmbedLibrary/include)
set(THIRD_PARTY ${CMAKE_CURRENT_SOURCE_DIR}/third_party)

# One output directory for every configuration, so the fixture's path is
# the same under single- and multi-config generators.
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
foreach(config Debug Release RelWithDebInfo MinSizeRel)
  string(TOUPPER ${config} upper)
  set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_${upper} ${CMAKE_BINARY_DIR}/bin)
endforeach()

# K3's dynamic library: the import library on Windows (never the static
# lingara_embed.lib, which carries its own CRT), the dylib or .so elsewhere.
if(WIN32)
  set(LINGARA_EMBED_LINK ${LINGARA_EMBED_LIB_DIR}/lingara_embed.dll.lib)
  set(LINGARA_EMBED_RUNTIME ${LINGARA_EMBED_LIB_DIR}/lingara_embed.dll)
elseif(APPLE)
  set(LINGARA_EMBED_LINK ${LINGARA_EMBED_LIB_DIR}/liblingara_embed.dylib)
else()
  set(LINGARA_EMBED_LINK ${LINGARA_EMBED_LIB_DIR}/liblingara_embed.so)
endif()
set(CMAKE_BUILD_RPATH ${LINGARA_EMBED_LIB_DIR})

# The engine's own flags: no exceptions, no RTTI, warnings as errors.
function(lingara_strict target standard)
  set_target_properties(${target} PROPERTIES CXX_STANDARD ${standard} CXX_STANDARD_REQUIRED ON CXX_EXTENSIONS OFF)
  target_include_directories(${target} PRIVATE ${PLUGIN_INCLUDE})
  target_include_directories(${target} SYSTEM PRIVATE ${THIRD_PARTY})
  target_link_libraries(${target} PRIVATE ${LINGARA_EMBED_LINK})
  if(MSVC)
    target_compile_options(${target} PRIVATE /W4 /WX /GR- /EHs-c- /external:W0)
    # _CRT_SECURE_NO_WARNINGS: C4996 deprecates std::getenv, which the
    # fixture reads once before any thread starts; under /WX it is an error.
    target_compile_definitions(${target} PRIVATE _HAS_EXCEPTIONS=0 JSON_NOEXCEPTION _CRT_SECURE_NO_WARNINGS)
  else()
    target_compile_options(${target} PRIVATE -fno-exceptions -fno-rtti -Wall -Wextra -Werror)
    target_compile_definitions(${target} PRIVATE JSON_NOEXCEPTION)
  endif()
  if(LINGARA_SANITIZE)
    target_compile_options(${target} PRIVATE -fsanitize=address,undefined -fno-omit-frame-pointer -fno-sanitize-recover=all)
    target_link_options(${target} PRIVATE -fsanitize=address,undefined)
  endif()
  if(WIN32)
    add_custom_command(TARGET ${target} POST_BUILD
      COMMAND ${CMAKE_COMMAND} -E copy_if_different ${LINGARA_EMBED_RUNTIME} $<TARGET_FILE_DIR:${target}>)
  endif()
endfunction()

set(SUITE_SOURCES test_main.cpp wrapper_tests.cpp header_tests.cpp)
foreach(standard 17 20)
  add_executable(wrapper_tests_cxx${standard} ${SUITE_SOURCES})
  lingara_strict(wrapper_tests_cxx${standard} ${standard})
  # With exceptions off, a failed REQUIRE aborts the binary: still a failure.
  target_compile_definitions(wrapper_tests_cxx${standard} PRIVATE
    DOCTEST_CONFIG_NO_EXCEPTIONS_BUT_WITH_ALL_ASSERTS
    LINGARA_FFI_HEADER="${REPO_ROOT}/ffi/include/lingara_embed.h"
    LINGARA_PLUGIN_HEADER="${PLUGIN_INCLUDE}/lingara_embed.h")
endforeach()

# The EK fixture: R1's case protocol over loopback HTTP (cpp-httplib, plain
# HTTP to 127.0.0.1 only, so no TLS dependency).
add_library(vendored_httplib STATIC ${THIRD_PARTY}/httplib.cpp)
set_target_properties(vendored_httplib PROPERTIES CXX_STANDARD 17 CXX_STANDARD_REQUIRED ON)
target_include_directories(vendored_httplib SYSTEM PUBLIC ${THIRD_PARTY})
target_compile_definitions(vendored_httplib PUBLIC CPPHTTPLIB_NO_EXCEPTIONS)
find_package(Threads REQUIRED)
target_link_libraries(vendored_httplib PUBLIC Threads::Threads)
if(WIN32)
  target_compile_definitions(vendored_httplib PUBLIC _WIN32_WINNT=0x0A00)
  target_link_libraries(vendored_httplib PUBLIC ws2_32)
endif()
if(MSVC)
  target_compile_options(vendored_httplib PRIVATE /W0 /GR- /EHs-c-)
  target_compile_definitions(vendored_httplib PRIVATE _HAS_EXCEPTIONS=0)
else()
  target_compile_options(vendored_httplib PRIVATE -w -fno-exceptions -fno-rtti)
endif()
if(LINGARA_SANITIZE)
  target_compile_options(vendored_httplib PRIVATE -fsanitize=address,undefined)
endif()

add_executable(ek_fixture fixture/main.cpp fixture/host_steps.cpp fixture/matching.cpp)
lingara_strict(ek_fixture 17)
target_link_libraries(ek_fixture PRIVATE vendored_httplib)

enable_testing()
add_test(NAME wrapper_tests_cxx17 COMMAND wrapper_tests_cxx17)
add_test(NAME wrapper_tests_cxx20 COMMAND wrapper_tests_cxx20)
