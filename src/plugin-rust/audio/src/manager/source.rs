// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Source` 接口与设备生命周期。
//!
//! - `Source::new` — 事件到达时创建 SourceState 写入 DeviceManager，并注册 D-Bus 对象
//! - `Source::update` — 事件到达时更新 SourceState
//! - `Source::delete` — 事件到达时回收资源并注销 D-Bus 对象
//! - D-Bus 属性从 `DeviceManager` 读取，操作委托给 `backend::pulse::source`

use std::sync::Arc;

use parking_lot::RwLock;
use zbus::interface;

use crate::backend::pulse::PulseManager;
use crate::backend::pulse::source as pulse_source;

use super::device_manager::{DeviceManager, SourceState};

/// Source D-Bus 对象。
pub struct Source {
    index: u32,
    pulse: Arc<PulseManager>,
    device_manager: Arc<RwLock<DeviceManager>>,
}

impl Source {
    /// 构造 Source D-Bus 对象实例（关联函数，由注册逻辑调用）。
    pub fn new_instance(
        index: u32,
        pulse: Arc<PulseManager>,
        device_manager: Arc<RwLock<DeviceManager>>,
    ) -> Self {
        Self { index, pulse, device_manager }
    }

    /// 生成 Source 的 D-Bus 对象路径。
    pub fn path(index: u32) -> String {
        format!("/org/deepin/dde/Audio2/Source{index}")
    }

    fn state(&self) -> Option<SourceState> {
        let reg = self.device_manager.read();
        reg.sources.get(&self.index).cloned()
    }
}

/// Source 设备生命周期（事件处理入口，由 event_loop 调用）。
impl Source {
    /// Source 新增：查询状态写入 DeviceManager，注册 D-Bus 对象。
    pub fn new(
        pulse: &Arc<PulseManager>,
        device_manager: &Arc<RwLock<DeviceManager>>,
        connection: &zbus::blocking::Connection,
        index: u32,
    ) -> Result<(), String> {
        let state = pulse_source::query_info(pulse, index)?;
        device_manager.write().add_source(index, state);
        eprintln!("[dde-audio] source new: {index}");

        let obj = Self::new_instance(index, pulse.clone(), device_manager.clone());
        connection
            .object_server()
            .at(Self::path(index), obj)
            .map_err(|e| format!("register source {index} failed: {e}"))?;
        Ok(())
    }

    /// Source 更新：查询最新状态写入 DeviceManager。
    pub fn update(
        pulse: &Arc<PulseManager>,
        device_manager: &Arc<RwLock<DeviceManager>>,
        index: u32,
    ) -> Result<(), String> {
        let state = pulse_source::query_info(pulse, index)?;
        device_manager.write().update_source(index, state);
        eprintln!("[dde-audio] source update: {index}");
        Ok(())
    }

    /// Source 删除：回收资源，注销 D-Bus 对象。
    pub fn delete(
        device_manager: &Arc<RwLock<DeviceManager>>,
        connection: &zbus::blocking::Connection,
        index: u32,
    ) {
        device_manager.write().remove_source(index);
        // TODO: 回收资源
        let _ = connection
            .object_server()
            .remove::<Source, _>(Self::path(index));
        eprintln!("[dde-audio] source delete: {index}");
    }
}

#[interface(name = "org.deepin.dde.Audio2.Source")]
impl Source {
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
        // TODO: 通过 pulse::source::get_meter 创建 Meter 并返回路径
        Err(zbus::fdo::Error::Failed("unimplemented".into()))
    }

    fn set_balance(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_source::set_balance(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_fade(&self, value: f64) -> zbus::fdo::Result<()> {
        pulse_source::set_fade(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_mute(&self, value: bool) -> zbus::fdo::Result<()> {
        pulse_source::set_mute(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_port(&self, name: &str) -> zbus::fdo::Result<()> {
        pulse_source::set_port(&self.pulse, self.index, name)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_volume(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_source::set_volume(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }
}
