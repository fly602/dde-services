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

use crate::backend::pulse::module::{MODULE_NULL_SINK, module_argument};
use crate::backend::pulse::PulseManager;

use device_manager::DeviceManager;

/// D-Bus 服务名。
#[allow(dead_code)]
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

        // 确保 null-sink 模块存在（端口切换时作为临时 default）
        let _ = load_module(&pulse, &device_manager, MODULE_NULL_SINK, None);
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
#[allow(dead_code)]
    pub fn pulse(&self) -> &Arc<PulseManager> {
        &self.pulse
    }

    /// 获取 DeviceManager 引用，供 D-Bus 子对象读取状态。
    pub fn device_manager(&self) -> &Arc<RwLock<DeviceManager>> {
        &self.device_manager
    }

    /// 获取 zbus Connection 引用，供 D-Bus 子对象动态注册/注销。
#[allow(dead_code)]
    pub fn connection(&self) -> &zbus::blocking::Connection {
        &self.connection
    }

    // ===== Audio 级别属性（后续实现） =====

#[allow(dead_code)]
    pub fn cards(&self) -> String {
        String::new()
    }
#[allow(dead_code)]
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
    /// 设置声卡端口。
    ///
    /// 流程：
    /// 1. 若当前声卡的 sink/source 已存在该端口 → 直接设置端口
    /// 2. 否则 select_profile 确定目标 profile
    /// 3. 目标 profile 与当前不同 → 记录 pending 并切换 profile，
    ///    等待设备重建后由 event_loop 完成端口设置
    pub fn set_port(&self, card_id: u32, port_name: &str, direction: u32) -> Result<(), String> {
        use crate::backend::pulse::card as pulse_card;
        use crate::backend::pulse::sink as pulse_sink;
        use crate::backend::pulse::source as pulse_source;

        // 从 DeviceManager 读取声卡状态
        let (active_profile, device_index, port_has_profile) = {
            let dm = self.device_manager.read();
            let card = dm.cards.get(&card_id).ok_or_else(|| {
                format!("card {card_id} not found")
            })?;

            // 查找目标端口
            let port = card.ports.iter().find(|p| p.name == port_name).ok_or_else(|| {
                format!("port {port_name} not found on card {card_id}")
            })?;

            // 查找该 card 的 sink/source 索引
            let device_index = if direction == 0 {
                dm.sinks.values().find(|s| s.card == card_id).map(|s| s.index)
            } else {
                dm.sources.values().find(|s| s.card == card_id).map(|s| s.index)
            };

            (
                card.active_profile.clone(),
                device_index,
                !port.profiles.is_empty(),
            )
        };

        // 1. 设备已存在且包含目标端口 → 直接设置
        if let Some(index) = device_index {
            let has_port = {
                let dm = self.device_manager.read();
                if direction == 0 {
                    dm.sinks
                        .get(&index)
                        .map(|s| s.ports.iter().any(|p| p.name == port_name))
                        .unwrap_or(false)
                } else {
                    dm.sources
                        .get(&index)
                        .map(|s| s.ports.iter().any(|p| p.name == port_name))
                        .unwrap_or(false)
                }
            };
            if has_port {
                eprintln!("[dde-audio] set_port: device {index} already has port {port_name}, set directly");
                return if direction == 0 {
                    pulse_sink::set_port(&self.pulse, index, port_name)
                } else {
                    pulse_source::set_port(&self.pulse, index, port_name)
                };
            }
        }

        // 2. 设备不存在该端口，需要切 profile
        if !port_has_profile {
            return Err(format!("port {port_name} has no profile on card {card_id}"));
        }

        // 确定目标 profile（从 DeviceManager 读端口 select_profile 结果）
        let target_profile = {
            let dm = self.device_manager.read();
            let card = dm.cards.get(&card_id).ok_or_else(|| {
                format!("card {card_id} not found")
            })?;
            let port = card.ports.iter().find(|p| p.name == port_name).ok_or_else(|| {
                format!("port {port_name} not found on card {card_id}")
            })?;
            port.select_profile().map(|s| s.to_owned())
        };

        let target_profile = match target_profile {
            Some(p) if !p.is_empty() => p,
            _ => return Err(format!("no available profile for card {card_id} port {port_name}")),
        };

        if active_profile != target_profile {
            // 3. profile 不同：记录 pending，切换 profile，等待设备重建
            eprintln!(
                "[dde-audio] set_port: switch card {card_id} profile {active_profile} -> {target_profile}"
            );
            self.device_manager
                .write()
                .set_pending_port(card_id, port_name.to_owned(), direction);
            pulse_card::set_card_profile(&self.pulse, card_id, &target_profile)?;
            Ok(())
        } else {
            // profile 相同但设备没有该端口（异常）：交给 pending 等设备重建
            self.device_manager
                .write()
                .set_pending_port(card_id, port_name.to_owned(), direction);
            Ok(())
        }
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
        let obj = Sink::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
        connection
            .object_server()
            .at(Sink::path(index), obj)
            .map_err(|e| format!("register sink {index} failed: {e}"))?;
    }

    // sources
    for state in source::query_list(pulse)? {
        let index = state.index;
        device_manager.write().add_source(index, state);
        let obj = Source::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
        connection
            .object_server()
            .at(Source::path(index), obj)
            .map_err(|e| format!("register source {index} failed: {e}"))?;
    }

    // sink_inputs
    for state in sink_input::query_list(pulse)? {
        let index = state.index;
        device_manager.write().add_sink_input(index, state);
        let obj = SinkInput::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
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

/// 加载模块（统一接口）。
///
/// - 模块已存在 → 标记 Complete，不重复加载
/// - 模块不存在 → 记录 Loading，按模块参数编排加载，等待设备创建事件完成
///
/// `channel` 为绑定设备名（单声道/降噪用），null-sink 传 None。
fn load_module(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    name: &str,
    channel: Option<&str>,
) -> Result<(), String> {
    use crate::backend::pulse::module::ModuleState;

    // 已加载完成则跳过
    if let ModuleState::Complete { .. } = device_manager.read().module_state(name) {
        return Ok(());
    }

    // 模块可能已被其他进程加载：查询索引并标记 Complete
    if let Some(index) = find_module_index(pulse, name)? {
        device_manager
            .write()
            .set_module_state(name, ModuleState::Complete { module_index: index });
        return Ok(());
    }

    // 不存在：记录 Loading 并加载
    let argument = module_argument(name, channel);
    device_manager.write().set_module_state(
        name,
        ModuleState::Loading { loaded_at: std::time::Instant::now() },
    );
    let index = pulse.load_module(name, &argument)?;
    device_manager
        .write()
        .set_module_state(name, ModuleState::Complete { module_index: index });
    eprintln!("[dde-audio] loaded module {name} (index {index})");
    Ok(())
}

/// 查询模块索引（存在则 Some，不存在则 None）。
fn find_module_index(
    pulse: &Arc<PulseManager>,
    name: &str,
) -> Result<Option<u32>, String> {
    use libpulse_binding::callbacks::ListResult;

    let name = name.to_owned();
    pulse.execute(|ctx, tx| {
        let intro = ctx.introspect();
        let mut found: Option<u32> = None;
        intro.get_module_info_list(move |res| {
            match res {
                ListResult::Item(info) => {
                    if info.name.as_deref() == Some(name.as_str()) {
                        found = Some(info.index);
                    }
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send(found.take());
                }
            }
        });
        true
    })
}
