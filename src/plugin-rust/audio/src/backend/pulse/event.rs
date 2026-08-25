// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 事件管理器。
//!
//! 在独立线程上消费 `PulseEvent` channel，维护设备索引集合。
//! 收到增删改事件时更新内部状态并打印日志。
//! 后续可扩展为更新 DBus 属性、发送信号等。

use std::collections::HashSet;
use std::sync::Arc;
use std::thread;

use parking_lot::RwLock;

use super::{PulseEvent, PulseManager};

/// 设备状态集合，由 EventManager 维护。
#[derive(Default)]
pub struct DeviceRegistry {
    pub sinks: HashSet<u32>,
    pub sources: HashSet<u32>,
    pub sink_inputs: HashSet<u32>,
    pub cards: HashSet<u32>,
}

/// 事件管理器，消费 PulseEvent 并更新 DeviceRegistry。
pub struct EventManager {
    registry: Arc<RwLock<DeviceRegistry>>,
    handle: Option<thread::JoinHandle<()>>,
}

impl EventManager {
    /// 启动事件消费线程。
    ///
    /// `pulse` 用于事件 handler 中查询设备详情（后续实现）。
    /// `events` 是 PulseManager 创建时返回的 receiver。
    pub fn start(
        pulse: Arc<PulseManager>,
        events: crossbeam_channel::Receiver<PulseEvent>,
    ) -> Self {
        let registry = Arc::new(RwLock::new(DeviceRegistry::default()));
        let registry_clone = registry.clone();

        let handle = thread::spawn(move || {
            for event in events {
                match event {
                    PulseEvent::SinkAdded { index } => {
                        let mut reg = registry_clone.write();
                        reg.sinks.insert(index);
                        eprintln!("[dde-audio] sink added: {index}");
                    }
                    PulseEvent::SinkRemoved { index } => {
                        let mut reg = registry_clone.write();
                        reg.sinks.remove(&index);
                        eprintln!("[dde-audio] sink removed: {index}");
                    }
                    PulseEvent::SinkChanged { index } => {
                        eprintln!("[dde-audio] sink changed: {index}");
                    }
                    PulseEvent::SourceAdded { index } => {
                        let mut reg = registry_clone.write();
                        reg.sources.insert(index);
                        eprintln!("[dde-audio] source added: {index}");
                    }
                    PulseEvent::SourceRemoved { index } => {
                        let mut reg = registry_clone.write();
                        reg.sources.remove(&index);
                        eprintln!("[dde-audio] source removed: {index}");
                    }
                    PulseEvent::SourceChanged { index } => {
                        eprintln!("[dde-audio] source changed: {index}");
                    }
                    PulseEvent::SinkInputAdded { index } => {
                        let mut reg = registry_clone.write();
                        reg.sink_inputs.insert(index);
                        eprintln!("[dde-audio] sink input added: {index}");
                    }
                    PulseEvent::SinkInputRemoved { index } => {
                        let mut reg = registry_clone.write();
                        reg.sink_inputs.remove(&index);
                        eprintln!("[dde-audio] sink input removed: {index}");
                    }
                    PulseEvent::SinkInputChanged { index } => {
                        eprintln!("[dde-audio] sink input changed: {index}");
                    }
                    PulseEvent::CardAdded { index } => {
                        let mut reg = registry_clone.write();
                        reg.cards.insert(index);
                        eprintln!("[dde-audio] card added: {index}");
                    }
                    PulseEvent::CardRemoved { index } => {
                        let mut reg = registry_clone.write();
                        reg.cards.remove(&index);
                        eprintln!("[dde-audio] card removed: {index}");
                    }
                    PulseEvent::CardChanged { index } => {
                        eprintln!("[dde-audio] card changed: {index}");
                    }
                    PulseEvent::DefaultSinkChanged { name } => {
                        eprintln!("[dde-audio] default sink changed: {name}");
                    }
                    PulseEvent::DefaultSourceChanged { name } => {
                        eprintln!("[dde-audio] default source changed: {name}");
                    }
                    PulseEvent::Server => {
                        eprintln!("[dde-audio] server event");
                    }
                }
            }
            eprintln!("[dde-audio] event manager thread exited");
            // 保持 pulse 引用存活，防止 mainloop 提前析构
            let _ = &pulse;
        });

        Self {
            registry,
            handle: Some(handle),
        }
    }

    /// 获取设备注册表只读引用。
    #[allow(dead_code)]
    pub fn registry(&self) -> &Arc<RwLock<DeviceRegistry>> {
        &self.registry
    }
}

impl Drop for EventManager {
    fn drop(&mut self) {
        // channel 关闭后线程自然退出，join 等待结束
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
