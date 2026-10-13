# A C-only consumer of the installed lingara-embed package (ADR 8.10.26q
# D6 step 6): lingara::embed must link with no C++ enabled, which is why
# the C++17 requirement lives on lingara::embed_cpp alone (D4).
cmake_minimum_required(VERSION 3.22)
project(lingara_c_consumer LANGUAGES C)

set(LINGARA_EXPECTED_VERSION "" CACHE STRING "The VERSION the library must report")
if(NOT LINGARA_EXPECTED_VERSION)
  message(FATAL_ERROR "LINGARA_EXPECTED_VERSION is not set: run make test-cpp-c")
endif()

find_package(lingara-embed CONFIG REQUIRED)

set(CMAKE_RUNTIME_OUTPUT_DIRECTORY ${CMAKE_BINARY_DIR}/bin)
foreach(config Debug Release RelWithDebInfo MinSizeRel)
  string(TOUPPER ${config} upper)
  set(CMAKE_RUNTIME_OUTPUT_DIRECTORY_${upper} ${CMAKE_BINARY_DIR}/bin)
endforeach()

add_executable(consumer_c main.c)
target_link_libraries(consumer_c PRIVATE lingara::embed)
target_compile_definitions(consumer_c PRIVATE LINGARA_EXPECTED_VERSION="${LINGARA_EXPECTED_VERSION}")
if(WIN32)
  # The DLL beside the binary (the plain zip puts it nowhere else).
  add_custom_command(TARGET consumer_c POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E copy_if_different $<TARGET_RUNTIME_DLLS:consumer_c> $<TARGET_FILE_DIR:consumer_c>
    COMMAND_EXPAND_LISTS)
endif()
