// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Sink` 接口。
//!
//! D-Bus 属性从 `DeviceRegistry` 读取，操作委托给 `backend::pulse::sink`。

use std::sync::Arc;

use parking_lot::RwLock;
use zbus::interface;

use crate::backend::pulse::PulseManager;
use crate::backend::pulse::sink as pulse_sink;

use super::registry::{DeviceRegistry, SinkState};

/// Sink D-Bus 对象。
pub struct Sink {
    index: u32,
    pulse: Arc<PulseManager>,
    registry: Arc<RwLock<DeviceRegistry>>,
}

impl Sink {
    pub fn new(index: u32, pulse: Arc<PulseManager>, registry: Arc<RwLock<DeviceRegistry>>) -> Self {
        Self { index, pulse, registry }
    }

    fn state(&self) -> Option<SinkState> {
        let reg = self.registry.read();
        reg.sinks.get(&self.index).cloned()
    }
}

#[interface(name = "org.deepin.dde.Audio2.Sink")]
impl Sink {
    // ========== 属性 ==========

    #[zbus(property)]
    pub fn name(&self) -> String {
        self.state().map(|s| s.name).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn description(&self) -> String {
        self.state().map(|s| s.description).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn base_volume(&self) -> f64 {
        self.state().map(|s| s.base_volume).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn mute(&self) -> bool {
        self.state().map(|s| s.mute).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn volume(&self) -> f64 {
        self.state().map(|s| s.volume).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn balance(&self) -> f64 {
        self.state().map(|s| s.balance).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn support_balance(&self) -> bool {
        self.state().map(|s| s.support_balance).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn fade(&self) -> f64 {
        self.state().map(|s| s.fade).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn support_fade(&self) -> bool {
        self.state().map(|s| s.support_fade).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn card(&self) -> u32 {
        self.state().map(|s| s.card).unwrap_or_default()
    }

    // ========== 方法 ==========

    fn get_meter(&self) -> zbus::fdo::Result<zbus::zvariant::OwnedObjectPath> {
        // TODO: 通过 pulse::sink::get_meter 创建 Meter 并返回路径
        Err(zbus::fdo::Error::Failed("unimplemented".into()))
    }

    fn set_balance(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_sink::set_balance(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_fade(&self, value: f64) -> zbus::fdo::Result<()> {
        pulse_sink::set_fade(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_mute(&self, value: bool) -> zbus::fdo::Result<()> {
        pulse_sink::set_mute(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_port(&self, name: &str) -> zbus::fdo::Result<()> {
        pulse_sink::set_port(&self.pulse, self.index, name)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_volume(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_sink::set_volume(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }
}
