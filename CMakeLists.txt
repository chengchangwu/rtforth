# CMakeLists.txt

cmake_minimum_required(VERSION 3.10)
project(rtforth VERSION 0.9.0)

enable_testing()

if (CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(CARGO_CMD cargo build)
else ()
    set(CARGO_CMD cargo build --release)
endif ()
set(CARGO_TEST_CMD cargo test)
set(CARGO_BENCH_CMD cargo bench)

add_custom_target(native
    COMMENT "Building native version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD} --example rf
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(x86
    COMMENT "Building x86 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD} --example rf --target=i686-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(x86-64
    COMMENT "Building x86-64 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD} --example rf --target=x86_64-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(arm
    COMMENT "Building arm version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD} --example rf --target=armv7-unknown-linux-gnueabihf
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(arm64
    COMMENT "Building arm64 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_CMD} --example rf --target=aarch64-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(test-native
    COMMENT "Test native version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_TEST_CMD}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(test-x86
    COMMENT "Test x86 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_TEST_CMD} --target=i686-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(test-x86-64
    COMMENT "Test x86-64 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_TEST_CMD} --target=x86_64-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(bench-native
    COMMENT "Benchmark native version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_BENCH_CMD}
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(bench-x86
    COMMENT "Benchmark x86 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_BENCH_CMD} --target=i686-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

add_custom_target(bench-x86-64
    COMMENT "Benchmark x86-64 version of rtforth"
    COMMAND ${CMAKE_COMMAND} -E env CARGO_TARGET_DIR=${CMAKE_CURRENT_BINARY_DIR} ${CARGO_BENCH_CMD} --target=x86_64-unknown-linux-gnu
    WORKING_DIRECTORY ${CMAKE_CURRENT_SOURCE_DIR})

