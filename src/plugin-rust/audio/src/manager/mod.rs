// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 音频管理器模块。
//!
//! 整合 D-Bus 接口定义、内存状态管理、事件处理。
//! - [`AudioManager`] — 持有 PulseManager、DeviceManager、EventLoop
//! - [`device_manager`] — 设备状态内存存储（四张 HashMap 表）
//! - [`event_loop`] — 消费 PulseEvent 更新 DeviceManager
//! - [`audio`] / [`sink`] / [`source`] / [`sink_input`] / [`card`] / [`meter`] — D-Bus 接口与业务逻辑

pub mod audio;
pub mod card;
pub mod device_manager;
pub mod event_loop;
pub mod meter;
pub mod sink;
pub mod sink_input;
pub mod source;

use std::sync::Arc;

use parking_lot::RwLock;

use crate::backend::pulse::PulseManager;

use device_manager::DeviceManager;

/// D-Bus 服务名。
pub const DBUS_SERVICE_NAME: &str = "org.deepin.dde.Audio2";

/// D-Bus 主对象路径。
pub const DBUS_PATH: &str = "/org/deepin/dde/Audio2";

/// 音频管理器。
///
/// 持有 PulseManager（libpulse 连接）、DeviceManager（内存状态）、
/// EventLoop（事件消费线程）。
///
/// D-Bus Audio 主接口通过 `Arc<AudioManager>` 调用 Audio 级操作。
/// Sink/Source/SinkInput 子接口通过 `Arc<RwLock<DeviceManager>>` 读取状态，
/// 通过 `Arc<PulseManager>` 调用底层操作。
pub struct AudioManager {
    pulse: Arc<PulseManager>,
    device_manager: Arc<RwLock<DeviceManager>>,
    connection: zbus::blocking::Connection,
    #[allow(dead_code)]
    event_loop: event_loop::EventLoop,
}

impl AudioManager {
    pub fn new(connection: zbus::blocking::Connection) -> Result<Self, String> {
        let (pulse, events) = PulseManager::new()?;
        let pulse = Arc::new(pulse);
        let device_manager = Arc::new(RwLock::new(DeviceManager::default()));

        // 启动前先查询当前音频状态，填充 DeviceManager 并注册 D-Bus 子对象
        init_devices(&pulse, &device_manager, &connection)?;

        let event_loop = event_loop::EventLoop::start(
            pulse.clone(),
            device_manager.clone(),
            connection.clone(),
            events,
        );

        Ok(Self {
            pulse,
            device_manager,
            connection,
            event_loop,
        })
    }

    /// 获取 PulseManager 引用，供 D-Bus 子对象调用底层操作。
    pub fn pulse(&self) -> &Arc<PulseManager> {
        &self.pulse
    }

    /// 获取 DeviceManager 引用，供 D-Bus 子对象读取状态。
    pub fn device_manager(&self) -> &Arc<RwLock<DeviceManager>> {
        &self.device_manager
    }

    /// 获取 zbus Connection 引用，供 D-Bus 子对象动态注册/注销。
    pub fn connection(&self) -> &zbus::blocking::Connection {
        &self.connection
    }

    // ===== Audio 级别属性（后续实现） =====

    pub fn cards(&self) -> String {
        String::new()
    }
    pub fn cards_without_unavailable(&self) -> String {
        String::new()
    }
    pub fn bluetooth_audio_mode(&self) -> String {
        String::new()
    }
    pub fn bluetooth_audio_mode_opts(&self) -> Vec<String> {
        Vec::new()
    }
    pub fn current_audio_server(&self) -> String {
        "pulseaudio".to_owned()
    }
    pub fn audio_server_state(&self) -> bool {
        true
    }
    pub fn max_ui_volume(&self) -> f64 {
        1.0
    }
    pub fn mono(&self) -> bool {
        false
    }

    // ===== Audio 级别方法（后续实现） =====

    pub fn set_bluetooth_audio_mode(&self, _mode: &str) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn set_mono(&self, _enable: bool) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn set_port(&self, _card_id: u32, _port_name: &str, _direction: u32) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn set_port_enabled(
        &self,
        _card_id: u32,
        _port_name: &str,
        _enabled: bool,
    ) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn is_port_enabled(&self, _card_id: u32, _port_name: &str) -> Result<bool, String> {
        Err("unimplemented".into())
    }
    pub fn set_current_audio_server(&self, _server_name: &str) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn stop_audio_service(&self) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn reset(&self) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn no_restart_pulse_audio(&self) -> Result<(), String> {
        Err("unimplemented".into())
    }
}

/// 启动初始化：查询当前所有设备状态，填充 DeviceManager 并注册 D-Bus 子对象。
///
/// 顺序：cards → sinks → sources → sink_inputs，再查询默认 sink/source。
fn init_devices(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    connection: &zbus::blocking::Connection,
) -> Result<(), String> {
    use crate::backend::pulse::{card, sink, sink_input, source};
    use crate::manager::sink::Sink;
    use crate::manager::sink_input::SinkInput;
    use crate::manager::source::Source;

    // cards（无 D-Bus 对象，只填状态）
    for state in card::query_list(pulse)? {
        device_manager.write().add_card(state.index, state);
    }

    // sinks（注册 D-Bus 对象）
    for state in sink::query_list(pulse)? {
        let index = state.index;
        device_manager.write().add_sink(index, state);
        let obj = Sink::new_instance(index, pulse.clone(), device_manager.clone());
        connection
            .object_server()
            .at(Sink::path(index), obj)
            .map_err(|e| format!("register sink {index} failed: {e}"))?;
    }

    // sources
    for state in source::query_list(pulse)? {
        let index = state.index;
        device_manager.write().add_source(index, state);
        let obj = Source::new_instance(index, pulse.clone(), device_manager.clone());
        connection
            .object_server()
            .at(Source::path(index), obj)
            .map_err(|e| format!("register source {index} failed: {e}"))?;
    }

    // sink_inputs
    for state in sink_input::query_list(pulse)? {
        let index = state.index;
        device_manager.write().add_sink_input(index, state);
        let obj = SinkInput::new_instance(index, pulse.clone(), device_manager.clone());
        connection
            .object_server()
            .at(SinkInput::path(index), obj)
            .map_err(|e| format!("register sink input {index} failed: {e}"))?;
    }

    // 默认 sink/source
    if let Ok((default_sink, default_source)) = pulse.default_sink_source() {
        device_manager.write().set_default_sink(default_sink);
        device_manager.write().set_default_source(default_source);
    }

    eprintln!("[dde-audio] device init done");
    Ok(())
}
