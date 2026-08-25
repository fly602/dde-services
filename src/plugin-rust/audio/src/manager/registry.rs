// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 设备内存状态注册表。
//!
//! event_loop 消费 PulseEvent 后更新此注册表，
//! D-Bus 接口层读取此注册表返回属性值。
//! 使用 `RwLock` 保护：event_loop 写，D-Bus 层读。

use std::collections::HashMap;

/// 音频端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub direction: u32,
}

/// 声卡端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct CardPortInfo {
    pub name: String,
    pub enabled: bool,
    pub bluetooth: bool,
    pub description: String,
    pub direction: u32,
}

/// 声卡信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct CardInfo {
    pub id: u32,
    pub name: String,
    pub ports: Vec<CardPortInfo>,
}

/// Sink（输出设备）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SinkState {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub base_volume: f64,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
    pub ports: Vec<Port>,
    pub active_port: Port,
    pub card: u32,
}

/// Source（输入设备）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SourceState {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub base_volume: f64,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
    pub ports: Vec<Port>,
    pub active_port: Port,
    pub card: u32,
}

/// SinkInput（播放流）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SinkInputState {
    pub index: u32,
    pub name: String,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
}

/// Card（声卡）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct CardState {
    pub index: u32,
    pub name: String,
    pub active_profile: String,
    pub ports: Vec<CardPortInfo>,
}

/// 设备状态注册表。
///
/// 由 event_loop 线程写入，D-Bus 接口层读取。
/// 通过 `Arc<RwLock<DeviceRegistry>>` 共享。
#[derive(Default)]
pub struct DeviceRegistry {
    pub sinks: HashMap<u32, SinkState>,
    pub sources: HashMap<u32, SourceState>,
    pub sink_inputs: HashMap<u32, SinkInputState>,
    pub cards: HashMap<u32, CardState>,
    pub default_sink: Option<String>,
    pub default_source: Option<String>,
}
