// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 事件循环。
//!
//! 在独立线程上消费 `PulseEvent` channel，调用各模块的 new/update/delete
//! 更新 DeviceManager，并动态注册/注销 D-Bus 子对象。

use std::sync::Arc;
use std::thread;

use parking_lot::RwLock;

use crate::backend::pulse::{PulseEvent, PulseManager};

use super::card;
use super::device_manager::DeviceManager;
use super::sink::Sink;
use super::sink_input::SinkInput;
use super::source::Source;

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
                    }
                    PulseEvent::SinkChanged { index } => {
                        let _ = Sink::update(&pulse, &device_manager, index);
                    }
                    PulseEvent::SinkRemoved { index } => {
                        Sink::delete(&device_manager, &connection, index);
                    }
                    PulseEvent::SourceAdded { index } => {
                        let _ = Source::new(&pulse, &device_manager, &connection, index);
                    }
                    PulseEvent::SourceChanged { index } => {
                        let _ = Source::update(&pulse, &device_manager, index);
                    }
                    PulseEvent::SourceRemoved { index } => {
                        Source::delete(&device_manager, &connection, index);
                    }
                    PulseEvent::SinkInputAdded { index } => {
                        let _ = SinkInput::new(&pulse, &device_manager, &connection, index);
                    }
                    PulseEvent::SinkInputChanged { index } => {
                        let _ = SinkInput::update(&pulse, &device_manager, index);
                    }
                    PulseEvent::SinkInputRemoved { index } => {
                        SinkInput::delete(&device_manager, &connection, index);
                    }
                    PulseEvent::CardAdded { index } => {
                        let _ = card::new(&pulse, &device_manager, index);
                    }
                    PulseEvent::CardChanged { index } => {
                        let _ = card::update(&pulse, &device_manager, index);
                    }
                    PulseEvent::CardRemoved { index } => {
                        card::delete(&device_manager, index);
                    }
                    PulseEvent::DefaultSinkChanged { name } => {
                        device_manager.write().set_default_sink(name.clone());
                        eprintln!("[dde-audio] default sink changed: {name}");
                        // TODO: 发 D-Bus PropertiesChanged 信号
                    }
                    PulseEvent::DefaultSourceChanged { name } => {
                        device_manager.write().set_default_source(name.clone());
                        eprintln!("[dde-audio] default source changed: {name}");
                        // TODO: 发 D-Bus PropertiesChanged 信号
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
