// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! DBus 接口定义。
//!
//! 与 Go 版 `org.deepin.dde.Audio2` 对应：
//! - `Audio` → `/org/deepin/dde/Audio2`（接口 `org.deepin.dde.Audio2`）
//! - `Sink`  → `/org/deepin/dde/Audio2/Sink{N}`（接口 `org.deepin.dde.Audio2.Sink`）
//! - `Source` → `/org/deepin/dde/Audio2/Source{N}`（接口 `org.deepin.dde.Audio2.Source`）
//! - `SinkInput` → `/org/deepin/dde/Audio2/SinkInput{N}`（接口 `org.deepin.dde.Audio2.SinkInput`）
//! - `Meter` → `/org/deepin/dde/Audio2/Meter{N}`（接口 `org.deepin.dde.Audio2.Meter`）

pub mod audio;
pub mod meter;
pub mod sink;
pub mod sink_input;
pub mod source;

/// DBus 服务名。
pub const DBUS_SERVICE_NAME: &str = "org.deepin.dde.Audio2";

/// DBus 主对象路径。
pub const DBUS_PATH: &str = "/org/deepin/dde/Audio2";
