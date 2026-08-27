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
use super::sink::Sink;
use super::sink_input::SinkInput;
use super::source::Source;
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
                        let _ = Sink::new(&pulse, &device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["Sinks"]);
                    }
                    PulseEvent::SinkChanged { index } => {
                        let _ = Sink::update(&pulse, &device_manager, index);
                        let path = Sink::path(index);
                        emit_device_changed(
                            &connection,
                            &path,
                            "org.deepin.dde.Audio2.Sink",
                            &["Name", "Description", "Volume", "Mute", "BaseVolume", "Balance", "Fade", "Card", "Ports"],
                        );
                    }
                    PulseEvent::SinkRemoved { index } => {
                        Sink::delete(&device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["Sinks"]);
                    }
                    PulseEvent::SourceAdded { index } => {
                        let _ = Source::new(&pulse, &device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["Sources"]);
                    }
                    PulseEvent::SourceChanged { index } => {
                        let _ = Source::update(&pulse, &device_manager, index);
                        let path = Source::path(index);
                        emit_device_changed(
                            &connection,
                            &path,
                            "org.deepin.dde.Audio2.Source",
                            &["Name", "Description", "Volume", "Mute", "BaseVolume", "Balance", "Fade", "Card", "Ports"],
                        );
                    }
                    PulseEvent::SourceRemoved { index } => {
                        Source::delete(&device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["Sources"]);
                    }
                    PulseEvent::SinkInputAdded { index } => {
                        let _ = SinkInput::new(&pulse, &device_manager, &connection, index);
                        emit_audio_list_changed(&connection, &["SinkInputs"]);
                    }
                    PulseEvent::SinkInputChanged { index } => {
                        let _ = SinkInput::update(&pulse, &device_manager, index);
                        let path = SinkInput::path(index);
                        emit_device_changed(
                            &connection,
                            &path,
                            "org.deepin.dde.Audio2.SinkInput",
                            &["Name", "Volume", "Mute", "Balance", "Fade"],
                        );
                    }
                    PulseEvent::SinkInputRemoved { index } => {
                        SinkInput::delete(&device_manager, &connection, index);
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
                        // TODO: 查询 server info（默认 sink/source 可能变了）
                        eprintln!("[dde-audio] server event");
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
