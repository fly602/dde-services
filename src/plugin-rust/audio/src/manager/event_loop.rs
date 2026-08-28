// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 事件循环。
//!
//! 在独立线程上消费 `PulseEvent` channel，调用各模块的 new/update/delete
//! 更新 DeviceManager，动态注册/注销 D-Bus 子对象，并发送属性变更信号。

use std::collections::HashMap;
use std::sync::Arc;
use std::thread;

use parking_lot::RwLock;
use zbus::zvariant::Value;

use crate::backend::pulse::{PulseEvent, PulseManager};

use super::card;
use super::device_manager::DeviceManager;
use super::sink::SinkInterface;
use super::sink_input::SinkInputInterface;
use super::source::SourceInterface;
use super::DBUS_PATH;

/// 发送 org.freedesktop.DBus.Properties.PropertiesChanged 信号。
///
/// `changed`：发生变化的属性（名 → 新值）。
/// `invalidated`：需要客户端重新读取的属性名。
fn emit_properties_changed(
    connection: &zbus::blocking::Connection,
    path: &str,
    interface_name: &str,
    changed: &HashMap<&str, Value<'_>>,
    invalidated: &[&str],
) {
    let path: zbus::zvariant::ObjectPath = match path.try_into() {
        Ok(p) => p,
        Err(_) => return,
    };
    let interface_name: zbus::names::InterfaceName = match interface_name.try_into() {
        Ok(n) => n,
        Err(_) => return,
    };

    // PropertiesChanged 信号 body: (interface_name, changed_props, invalidated_props)
    let body = (
        interface_name,
        changed,
        invalidated.to_vec(),
    );
    let _ = connection.emit_signal(
        None::<&str>,
        path,
        "org.freedesktop.DBus.Properties",
        "PropertiesChanged",
        &body,
    );
}

/// 设备增删后通知 Audio 对象列表属性变化。
fn emit_audio_list_changed(connection: &zbus::blocking::Connection, props: &[&str]) {
    emit_properties_changed(
        connection,
        DBUS_PATH,
        "org.deepin.dde.Audio2",
        &HashMap::new(),
        props,
    );
}

/// 子对象更新后通知其属性变化。
fn emit_device_changed(
    connection: &zbus::blocking::Connection,
    path: &str,
    interface_name: &str,
    props: &[&str],
) {
    emit_properties_changed(connection, path, interface_name, &HashMap::new(), props);
}

/// 设备创建后通知声卡模块检查 pending profile 完成。
fn try_complete_pending_profile(
    device_manager: &Arc<RwLock<DeviceManager>>,
    device_index: u32,
    is_sink: bool,
) {
    card::on_device_created(device_manager, device_index, is_sink);
}

/// 事件循环管理器。
pub struct EventLoop {
    handle: Option<thread::JoinHandle<()>>,
}

impl EventLoop {
    /// 启动事件消费线程。
    pub fn start(
        pulse: Arc<PulseManager>,
        device_manager: Arc<RwLock<DeviceManager>>,
        connection: zbus::blocking::Connection,
        events: crossbeam_channel::Receiver<PulseEvent>,
    ) -> Self {
        let handle = thread::spawn(move || {
            for event in events {
                match event {
                    PulseEvent::SinkAdded { index } => {
                        let _ = SinkInterface::new(&pulse, &device_manager, &connection, index);
                        try_complete_pending_profile(&device_manager, index, true);
                        emit_audio_list_changed(&connection, &["Sinks"]);
                    }
                    PulseEvent::SinkChanged { index } => {
                        let _ = SinkInterface::update(&pulse, &device_manager, index);
                        let path = SinkInterface::path(index);
                        emit_device_changed(
                            &connection,
                            &path,
                            "org.deepin.dde.Audio2.Sink",
                            &["Name", "Description", "Volume", "Mute", "BaseVolume", "Balance", "Fade", "Card", "Ports"],
                        );
                    }
                    PulseEvent::SinkRemoved { index } => {
                        SinkInterface::delete(&device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["Sinks"]);
                    }
                    PulseEvent::SourceAdded { index } => {
                        let _ = SourceInterface::new(&pulse, &device_manager, &connection, index);
                        try_complete_pending_profile(&device_manager, index, false);
                        emit_audio_list_changed(&connection, &["Sources"]);
                    }
                    PulseEvent::SourceChanged { index } => {
                        let _ = SourceInterface::update(&pulse, &device_manager, index);
                        let path = SourceInterface::path(index);
                        emit_device_changed(
                            &connection,
                            &path,
                            "org.deepin.dde.Audio2.Source",
                            &["Name", "Description", "Volume", "Mute", "BaseVolume", "Balance", "Fade", "Card", "Ports"],
                        );
                    }
                    PulseEvent::SourceRemoved { index } => {
                        SourceInterface::delete(&device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["Sources"]);
                    }
                    PulseEvent::SinkInputAdded { index } => {
                        let _ = SinkInputInterface::new(&pulse, &device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["SinkInputs"]);
                    }
                    PulseEvent::SinkInputChanged { index } => {
                        let _ = SinkInputInterface::update(&pulse, &device_manager, index);
                        let path = SinkInputInterface::path(index);
                        emit_device_changed(
                            &connection,
                            &path,
                            "org.deepin.dde.Audio2.SinkInput",
                            &["Name", "Volume", "Mute", "Balance", "Fade"],
                        );
                    }
                    PulseEvent::SinkInputRemoved { index } => {
                        SinkInputInterface::delete(&device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["SinkInputs"]);
                    }
                    PulseEvent::CardAdded { index } => {
                        let _ = card::new(&pulse, &device_manager, index);
                        emit_audio_list_changed(&connection, &["Cards", "CardsWithoutUnavailable"]);
                    }
                    PulseEvent::CardChanged { index } => {
                        let _ = card::update(&pulse, &device_manager, index);
                        emit_audio_list_changed(&connection, &["Cards", "CardsWithoutUnavailable"]);
                    }
                    PulseEvent::CardRemoved { index } => {
                        card::delete(&device_manager, index);
                        emit_audio_list_changed(&connection, &["Cards", "CardsWithoutUnavailable"]);
                    }
                    PulseEvent::DefaultSinkChanged { name } => {
                        device_manager.write().set_default_sink(name.clone());
                        emit_audio_list_changed(&connection, &["DefaultSink"]);
                    }
                    PulseEvent::DefaultSourceChanged { name } => {
                        device_manager.write().set_default_source(name.clone());
                        emit_audio_list_changed(&connection, &["DefaultSource"]);
                    }
                    PulseEvent::Server => {
                        // 服务器变化：查询最新默认 sink/source 并更新
                        if let Ok((sink, source)) = pulse.default_sink_source() {
                            device_manager.write().set_default_sink(sink.clone());
                            device_manager.write().set_default_source(source.clone());
                            if !sink.is_empty() {
                                emit_audio_list_changed(&connection, &["DefaultSink"]);
                            }
                            if !source.is_empty() {
                                emit_audio_list_changed(&connection, &["DefaultSource"]);
                            }
                        }
                    }
                }
            }
            eprintln!("[dde-audio] event loop thread exited");
            let _ = &pulse;
        });

        Self {
            handle: Some(handle),
        }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
