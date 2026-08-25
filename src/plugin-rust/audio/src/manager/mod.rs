// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 音频管理器模块。
//!
//! 整合 D-Bus 接口定义、内存状态管理、事件处理。
//! - [`AudioManager`] — 持有 PulseManager、DeviceRegistry、EventLoop
//! - [`registry`] — 设备状态内存存储
//! - [`event_loop`] — 消费 PulseEvent 更新 registry
//! - [`audio`] / [`sink`] / [`source`] / [`sink_input`] / [`meter`] — D-Bus 接口

pub mod audio;
pub mod event_loop;
pub mod meter;
pub mod registry;
pub mod sink;
pub mod sink_input;
pub mod source;

use std::sync::Arc;

use parking_lot::RwLock;

use crate::backend::pulse::PulseManager;

use registry::DeviceRegistry;

/// D-Bus 服务名。
pub const DBUS_SERVICE_NAME: &str = "org.deepin.dde.Audio2";

/// D-Bus 主对象路径。
pub const DBUS_PATH: &str = "/org/deepin/dde/Audio2";

/// 音频管理器。
///
/// 持有 PulseManager（libpulse 连接）、DeviceRegistry（内存状态）、
/// EventLoop（事件消费线程）。
///
/// D-Bus Audio 主接口通过 `Arc<AudioManager>` 调用 Audio 级操作。
/// Sink/Source/SinkInput 子接口通过 `Arc<RwLock<DeviceRegistry>>` 读取状态，
/// 通过 `Arc<PulseManager>` 调用底层操作。
pub struct AudioManager {
    pulse: Arc<PulseManager>,
    registry: Arc<RwLock<DeviceRegistry>>,
    #[allow(dead_code)]
    event_loop: event_loop::EventLoop,
}

impl AudioManager {
    pub fn new() -> Result<Self, String> {
        let (pulse, events) = PulseManager::new()?;
        let pulse = Arc::new(pulse);
        let registry = Arc::new(RwLock::new(DeviceRegistry::default()));
        let event_loop = event_loop::EventLoop::start(pulse.clone(), registry.clone(), events);

        Ok(Self {
            pulse,
            registry,
            event_loop,
        })
    }

    /// 获取 PulseManager 引用，供 D-Bus 子对象调用底层操作。
    pub fn pulse(&self) -> &Arc<PulseManager> {
        &self.pulse
    }

    /// 获取 DeviceRegistry 引用，供 D-Bus 子对象读取状态。
    pub fn registry(&self) -> &Arc<RwLock<DeviceRegistry>> {
        &self.registry
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
