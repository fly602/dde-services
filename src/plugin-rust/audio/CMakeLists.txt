# SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
#
# SPDX-License-Identifier: LGPL-3.0-or-later

find_program(CARGO_EXECUTABLE cargo REQUIRED)

set(RUST_TARGET_DIR "${CMAKE_BINARY_DIR}/cargo")
set(RUST_PLUGIN "${RUST_TARGET_DIR}/debug/libplugin_rust_audio.so")

add_custom_command(
    OUTPUT "${RUST_PLUGIN}"
    COMMAND
        ${CMAKE_COMMAND} -E env
        CARGO_TARGET_DIR=${RUST_TARGET_DIR}
        ${CARGO_EXECUTABLE} build
            --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
    DEPENDS
        "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/lib.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/abi.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/backend/mod.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/backend/pulse/mod.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/backend/pulse/card.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/backend/pulse/sink.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/backend/pulse/source.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/backend/pulse/sink_input.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/dbus/mod.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/dbus/audio.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/dbus/sink.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/dbus/source.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/dbus/sink_input.rs"
        "${CMAKE_CURRENT_SOURCE_DIR}/src/dbus/meter.rs"
    WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}"
    VERBATIM
)

add_custom_target(plugin-rust-audio ALL DEPENDS "${RUST_PLUGIN}")

# 将构建产物和配置文件复制到构建目录，便于 Debug 模式直接运行
add_custom_command(
    TARGET plugin-rust-audio POST_BUILD
    COMMAND ${CMAKE_COMMAND} -E make_directory "${CMAKE_BINARY_DIR}/lib"
    COMMAND ${CMAKE_COMMAND} -E make_directory "${CMAKE_BINARY_DIR}/share"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${RUST_PLUGIN}"
        "${CMAKE_BINARY_DIR}/lib/libplugin_rust_audio.so"
    COMMAND ${CMAKE_COMMAND} -E copy_if_different
        "${CMAKE_CURRENT_SOURCE_DIR}/plugin-rust-audio.json"
        "${CMAKE_BINARY_DIR}/share/plugin-rust-audio.json"
    VERBATIM
)

# 安装：so 到 deepin-service-manager lib 目录，json 到 user 配置目录
install(FILES "${RUST_PLUGIN}"
        DESTINATION ${CMAKE_INSTALL_LIBDIR}/deepin-service-manager)

install(FILES plugin-rust-audio.json
        DESTINATION share/deepin-service-manager/user)
