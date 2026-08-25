// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2` 主接口。
//!
//! 属性和方法对应 Go 版 `Audio` 结构体的导出成员和 `exported_methods_auto.go`。

use std::sync::Arc;

use zbus::interface;

use crate::backend::AudioManager;

/// Audio DBus 对象。
///
/// 持有 `Arc<AudioManager>`，所有方法委托给 AudioManager。
pub struct Audio {
    manager: Arc<AudioManager>,
}

impl Audio {
    pub fn new(manager: AudioManager) -> Self {
        Self {
            manager: Arc::new(manager),
        }
    }
}

#[interface(name = "org.deepin.dde.Audio2")]
impl Audio {
    // ========== 属性 ==========

    /// 当前所有 SinkInput 的对象路径列表。
    #[zbus(property)]
    pub fn sink_inputs(&self) -> Vec<zbus::zvariant::OwnedObjectPath> {
        Vec::new()
    }

    /// 当前所有 Sink 的对象路径列表。
    #[zbus(property)]
    pub fn sinks(&self) -> Vec<zbus::zvariant::OwnedObjectPath> {
        Vec::new()
    }

    /// 当前所有 Source 的对象路径列表。
    #[zbus(property)]
    pub fn sources(&self) -> Vec<zbus::zvariant::OwnedObjectPath> {
        Vec::new()
    }

    /// 默认输出设备路径。
    #[zbus(property)]
    pub fn default_sink(&self) -> zbus::zvariant::OwnedObjectPath {
        zbus::zvariant::OwnedObjectPath::default()
    }

    /// 默认输入设备路径。
    #[zbus(property)]
    pub fn default_source(&self) -> zbus::zvariant::OwnedObjectPath {
        zbus::zvariant::OwnedObjectPath::default()
    }

    /// 声卡信息（JSON 字符串）。
    #[zbus(property)]
    pub fn cards(&self) -> String {
        self.manager.cards()
    }

    /// 声卡信息（不含不可用设备，JSON 字符串）。
    #[zbus(property)]
    pub fn cards_without_unavailable(&self) -> String {
        self.manager.cards_without_unavailable()
    }

    /// 蓝牙音频模式。
    #[zbus(property)]
    pub fn bluetooth_audio_mode(&self) -> String {
        self.manager.bluetooth_audio_mode()
    }

    /// 可用的蓝牙音频模式列表。
    #[zbus(property)]
    pub fn bluetooth_audio_mode_opts(&self) -> Vec<String> {
        self.manager.bluetooth_audio_mode_opts()
    }

    /// 当前音频服务器名称。
    #[zbus(property)]
    pub fn current_audio_server(&self) -> String {
        self.manager.current_audio_server()
    }

    /// 音频服务器状态。
    #[zbus(property)]
    pub fn audio_server_state(&self) -> bool {
        self.manager.audio_server_state()
    }

    /// 最大 UI 音量。
    #[zbus(property)]
    pub fn max_ui_volume(&self) -> f64 {
        self.manager.max_ui_volume()
    }

    /// 单声道模式。
    #[zbus(property)]
    pub fn mono(&self) -> bool {
        self.manager.mono()
    }

    // ========== 方法 ==========

    /// 检查端口是否启用。
    fn is_port_enabled(&self, card_id: u32, port_name: &str) -> zbus::fdo::Result<bool> {
        self.manager
            .is_port_enabled(card_id, port_name)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 设置不再重启 PulseAudio。
    fn no_restart_pulse_audio(&self) -> zbus::fdo::Result<()> {
        self.manager
            .no_restart_pulse_audio()
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 重置音频配置。
    fn reset(&self) -> zbus::fdo::Result<()> {
        self.manager.reset().map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 设置蓝牙音频模式。
    fn set_bluetooth_audio_mode(&self, mode: &str) -> zbus::fdo::Result<()> {
        self.manager
            .set_bluetooth_audio_mode(mode)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 设置声卡端口。
    fn set_port(&self, card_id: u32, port_name: &str, direction: u32) -> zbus::fdo::Result<()> {
        self.manager
            .set_port(card_id, port_name, direction)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 启用/禁用端口。
    fn set_port_enabled(&self, card_id: u32, port_name: &str, enabled: bool) -> zbus::fdo::Result<()> {
        self.manager
            .set_port_enabled(card_id, port_name, enabled)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 设置当前音频服务器。
    fn set_current_audio_server(&self, server_name: &str) -> zbus::fdo::Result<()> {
        self.manager
            .set_current_audio_server(server_name)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 设置单声道模式。
    fn set_mono(&self, enable: bool) -> zbus::fdo::Result<()> {
        self.manager
            .set_mono(enable)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    /// 停止音频服务。
    fn stop_audio_service(&self) -> zbus::fdo::Result<()> {
        self.manager
            .stop_audio_service()
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }
}
