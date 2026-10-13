# An engine-free C++ consumer of the installed lingara-embed package
# (ADR 8.10.26q D6). The same project is built three ways by make/cpp.mk:
# through the vcpkg overlay port, through Conan's CMakeDeps, and with
# CMAKE_PREFIX_PATH at an unzipped ffi zip. It names no path into the repo
# but doctest's, which make passes as LINGARA_DOCTEST_DIR.
cmake_minimum_required(VERSION 3.22)
project(lingara_cpp_consumer LANGUAGES CXX)

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(LINGARA_DOCTEST_DIR "" CACHE PATH "Directory holding doctest.h")
if(NOT LINGARA_DOCTEST_DIR)
  message(FATAL_ERROR "LINGARA_DOCTEST_DIR is not set: run make test-cpp")
endif()

find_package(lingara-embed CONFIG REQUIRED)

# One output directory for every configuration, so make finds the binary
# under single- and multi-config generators alike.
set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
foreach(config Debug Release RelWithDebInfo MinSizeRel)
  string(TOUPPER ${config} upper)
  set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_${upper} ${CMAKE_BINARY_DIR}/bin)
endforeach()

add_executable(consumer main.cpp)
target_link_libraries(consumer PRIVATE lingara::embed_cpp)
target_include_directories(consumer SYSTEM PRIVATE ${LINGARA_DOCTEST_DIR})
target_compile_definitions(consumer PRIVATE DOCTEST_CONFIG_IMPLEMENT_WITH_MAIN)
if(WIN32)
  # The DLL beside the binary. The list is empty when the package manager
  # puts the DLL on PATH instead (Conan's conanrun), and copy_if_different
  # refuses an empty list, so that case runs `true`.
  set(dlls $<TARGET_RUNTIME_DLLS:consumer>)
  add_custom_command(TARGET consumer POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E $<IF:$<BOOL:${dlls}>,copy_if_different,true> ${dlls} $<TARGET_FILE_DIR:consumer>
    COMMAND_EXPAND_LISTS)
endif()
