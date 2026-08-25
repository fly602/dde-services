// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 音频后端。
//!
//! 通过 libpulse-binding 连接 PulseAudio / pipewire-pulse 兼容层。
//! 不区分底层服务器，统一走 PulseAudio 协议。
//!
//! `AudioManager` 持有 `PulseManager`，负责 Audio 级别的操作（cards、
//! default sink/source、蓝牙模式、音频服务器切换等）。
//! 对象级操作（Sink/Source/SinkInput 的音量、静音、端口等）由各自的
//! DBus 对象直接持有 `Arc<PulseManager>` 调用 `pulse` 子模块实现。

pub mod pulse;

use std::sync::Arc;

use pulse::PulseManager;

/// 音频端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Port {
    pub name: String,
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct CardPortInfo {
    pub name: String,
    pub enabled: bool,
    pub bluetooth: bool,
    pub description: String,
    pub direction: u32,
}

/// Sink（输出设备）信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SinkInfo {
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

/// Source（输入设备）信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SourceInfo {
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

/// SinkInput（播放流）信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SinkInputInfo {
    pub index: u32,
    pub name: String,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
}

/// 音频管理器，持有 PulseManager 并对外提供 Audio 级别操作接口。
///
/// DBus Audio 主接口持有 `AudioManager`。
/// Sink/Source/SinkInput/Meter 子对象不经过 AudioManager，
/// 直接持有 `Arc<PulseManager>` 调用 `pulse` 子模块。
pub struct AudioManager {
    pulse: Arc<PulseManager>,
    #[allow(dead_code)]
    events: pulse::event::EventManager,
}

impl AudioManager {
    pub fn new() -> Result<Self, String> {
        let (pulse, events) = PulseManager::new()?;
        let pulse = Arc::new(pulse);
        let events = pulse::event::EventManager::start(pulse.clone(), events);
        Ok(Self { pulse, events })
    }

    /// 获取 PulseManager 引用，供 DBus 子对象调用 pulse 子模块。
    pub fn pulse(&self) -> &Arc<PulseManager> {
        &self.pulse
    }

    // ===== Audio 级别属性（后续实现） =====

    #[allow(dead_code)]
    pub fn sinks(&self) -> Vec<u32> {
        Vec::new()
    }
    #[allow(dead_code)]
    pub fn sources(&self) -> Vec<u32> {
        Vec::new()
    }
    #[allow(dead_code)]
    pub fn sink_inputs(&self) -> Vec<u32> {
        Vec::new()
    }
    #[allow(dead_code)]
    pub fn default_sink(&self) -> Option<u32> {
        None
    }
    #[allow(dead_code)]
    pub fn default_source(&self) -> Option<u32> {
        None
    }
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

    // ===== Audio 级别方法 =====

    pub fn set_bluetooth_audio_mode(&self, _mode: &str) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn set_mono(&self, _enable: bool) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn set_port(&self, _card_id: u32, _port_name: &str, _direction: u32) -> Result<(), String> {
        // TODO: direction 决定是 sink port 还是 source port
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
