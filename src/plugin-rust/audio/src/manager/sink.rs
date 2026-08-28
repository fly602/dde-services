// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Sink` 接口与设备生命周期。
//!
//! - `SinkInterface::new` — 事件到达时创建 Sink 写入 DeviceManager，并注册 D-Bus 对象
//! - `SinkInterface::update` — 事件到达时更新 Sink
//! - `SinkInterface::delete` — 事件到达时回收资源并注销 D-Bus 对象
//! - D-Bus 属性从 `DeviceManager` 读取，操作委托给 `backend::pulse::sink`

use std::sync::Arc;

use parking_lot::RwLock;
use zbus::interface;

use crate::backend::pulse::PulseManager;
use crate::backend::pulse::sink as pulse_sink;

/// 音频端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub direction: u32,
}

/// Sink（输出设备）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Sink {
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

use super::device_manager::DeviceManager;

/// Sink D-Bus 对象。
///
/// 事件到达时由 `SinkInterface::new` 创建并注册到 zbus ObjectServer。
/// 属性通过 DeviceManager 读取，不保存可变状态。
pub struct SinkInterface {
    index: u32,
    pulse: Arc<PulseManager>,
    device_manager: Arc<RwLock<DeviceManager>>,
    connection: zbus::blocking::Connection,
}

impl SinkInterface {
    pub fn new_instance(
        index: u32,
        pulse: Arc<PulseManager>,
        device_manager: Arc<RwLock<DeviceManager>>,
        connection: zbus::blocking::Connection,
    ) -> Self {
        Self { index, pulse, device_manager, connection }
    }

    /// 生成 Sink 的 D-Bus 对象路径。
    pub fn path(index: u32) -> String {
        format!("/org/deepin/dde/Audio2/Sink{index}")
    }

    fn state(&self) -> Option<Sink> {
        let reg = self.device_manager.read();
        reg.sinks.get(&self.index).cloned()
    }
}

/// Sink 设备生命周期（事件处理入口，由 event_loop 调用）。
impl SinkInterface {
    /// Sink 新增：查询状态写入 DeviceManager，注册 D-Bus 对象。
    ///
    /// 返回 `(index, 是否成功)`。注册失败不阻断状态更新。
    pub fn new(
        pulse: &Arc<PulseManager>,
        device_manager: &Arc<RwLock<DeviceManager>>,
        connection: &zbus::blocking::Connection,
        index: u32,
    ) -> Result<(), String> {
        let state = pulse_sink::query_info(pulse, index)?;
        device_manager.write().add_sink(index, state);
        eprintln!("[dde-audio] sink new: {index}");
        let obj = Self::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
        connection
            .object_server()
            .at(Self::path(index), obj)
            .map_err(|e| format!("register sink {index} failed: {e}"))?;
        Ok(())
    }

    /// Sink 更新：查询最新状态写入 DeviceManager。
    ///
    /// D-Bus 属性读取时从 DeviceManager 拿最新值，无需持有实例引用。
    pub fn update(
        pulse: &Arc<PulseManager>,
        device_manager: &Arc<RwLock<DeviceManager>>,
        index: u32,
    ) -> Result<(), String> {
        let state = pulse_sink::query_info(pulse, index)?;
        device_manager.write().update_sink(index, state);
        eprintln!("[dde-audio] sink update: {index}");
        Ok(())
    }

    /// Sink 删除：回收资源，注销 D-Bus 对象。
    pub fn delete(
        device_manager: &Arc<RwLock<DeviceManager>>,
        connection: &zbus::blocking::Connection,
        index: u32,
    ) {
        device_manager.write().remove_sink(index);
        // TODO: 回收资源（如 meter stream）
        let _ = connection
            .object_server()
            .remove::<SinkInterface, _>(Self::path(index));
        eprintln!("[dde-audio] sink delete: {index}");
    }
}

#[interface(name = "org.deepin.dde.Audio2.Sink")]
impl SinkInterface {
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
        use super::meter::Meter;

        let meter = Meter::new(self.index, true, self.device_manager.clone());
        let path = Meter::path(self.index, true);
        self.connection
            .object_server()
            .at(path.clone(), meter)
            .map_err(|e| zbus::fdo::Error::Failed(format!("register meter failed: {e}")))?;
        zbus::zvariant::ObjectPath::try_from(path)
            .map(Into::into)
            .map_err(|e| zbus::fdo::Error::Failed(e.to_string()))
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
