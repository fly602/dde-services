// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 设备管理器。
//!
//! 管理音频设备状态的内存存储。event_loop 通过各子模块的 new/update/delete
//! 处理事件后，调用 DeviceManager 的方法增删改四张表。
//! D-Bus 接口层通过 `Arc<RwLock<DeviceManager>>` 读取状态。
//!
//! 职责分工：
//! - DeviceManager — 管理四张 HashMap 表，提供增删查入口
//! - 子 device（sink.rs/source.rs/card.rs/sink_input.rs）— 处理 new/update
//!   的具体逻辑（查 pulse 构造状态），delete 有回收也在子 device 处理

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

/// 设备管理器。
///
/// 持有四张 HashMap 表管理设备状态，通过 `Arc<RwLock<DeviceManager>>` 共享。
/// event_loop 写入，D-Bus 接口层读取。
#[derive(Default)]
pub struct DeviceManager {
    pub sinks: HashMap<u32, SinkState>,
    pub sources: HashMap<u32, SourceState>,
    pub sink_inputs: HashMap<u32, SinkInputState>,
    pub cards: HashMap<u32, CardState>,
    pub default_sink: Option<String>,
    pub default_source: Option<String>,
}

impl DeviceManager {
    // ===== Sink =====

    pub fn add_sink(&mut self, index: u32, state: SinkState) {
        self.sinks.insert(index, state);
    }

    pub fn update_sink(&mut self, index: u32, state: SinkState) {
        self.sinks.insert(index, state);
    }

    pub fn remove_sink(&mut self, index: u32) -> Option<SinkState> {
        self.sinks.remove(&index)
    }

    pub fn get_sink(&self, index: u32) -> Option<&SinkState> {
        self.sinks.get(&index)
    }

    // ===== Source =====

    pub fn add_source(&mut self, index: u32, state: SourceState) {
        self.sources.insert(index, state);
    }

    pub fn update_source(&mut self, index: u32, state: SourceState) {
        self.sources.insert(index, state);
    }

    pub fn remove_source(&mut self, index: u32) -> Option<SourceState> {
        self.sources.remove(&index)
    }

    pub fn get_source(&self, index: u32) -> Option<&SourceState> {
        self.sources.get(&index)
    }

    // ===== SinkInput =====

    pub fn add_sink_input(&mut self, index: u32, state: SinkInputState) {
        self.sink_inputs.insert(index, state);
    }

    pub fn update_sink_input(&mut self, index: u32, state: SinkInputState) {
        self.sink_inputs.insert(index, state);
    }

    pub fn remove_sink_input(&mut self, index: u32) -> Option<SinkInputState> {
        self.sink_inputs.remove(&index)
    }

    pub fn get_sink_input(&self, index: u32) -> Option<&SinkInputState> {
        self.sink_inputs.get(&index)
    }

    // ===== Card =====

    pub fn add_card(&mut self, index: u32, state: CardState) {
        self.cards.insert(index, state);
    }

    pub fn update_card(&mut self, index: u32, state: CardState) {
        self.cards.insert(index, state);
    }

    pub fn remove_card(&mut self, index: u32) -> Option<CardState> {
        self.cards.remove(&index)
    }

    pub fn get_card(&self, index: u32) -> Option<&CardState> {
        self.cards.get(&index)
    }

    // ===== Default =====

    pub fn set_default_sink(&mut self, name: String) {
        self.default_sink = Some(name);
    }

    pub fn set_default_source(&mut self, name: String) {
        self.default_source = Some(name);
    }
}
