// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 事件循环。
//!
//! 在独立线程上消费 `PulseEvent` channel，更新 `DeviceRegistry`。
//! 后续扩展：查询 pulse 获取最新状态、发 D-Bus 信号、触发策略逻辑。

use std::sync::Arc;
use std::thread;

use parking_lot::RwLock;

use crate::backend::pulse::{PulseEvent, PulseManager};

use super::registry::DeviceRegistry;

/// 事件循环管理器。
pub struct EventLoop {
    handle: Option<thread::JoinHandle<()>>,
}

impl EventLoop {
    /// 启动事件消费线程。
    ///
    /// `pulse` 用于事件处理中查询设备详情（后续实现）。
    /// `registry` 与 D-Bus 层共享，事件更新写入，D-Bus 读取。
    /// `events` 是 PulseManager 创建时返回的 receiver。
    pub fn start(
        pulse: Arc<PulseManager>,
        registry: Arc<RwLock<DeviceRegistry>>,
        events: crossbeam_channel::Receiver<PulseEvent>,
    ) -> Self {
        let handle = thread::spawn(move || {
            for event in events {
                match event {
                    PulseEvent::SinkAdded { index } => {
                        // TODO: pulse.execute 查询 sink info，写入 registry
                        let mut reg = registry.write();
                        eprintln!("[dde-audio] sink added: {index}");
                        // 临时占位：后续替换为真实查询
                        let _ = &mut reg;
                    }
                    PulseEvent::SinkRemoved { index } => {
                        let mut reg = registry.write();
                        reg.sinks.remove(&index);
                        eprintln!("[dde-audio] sink removed: {index}");
                    }
                    PulseEvent::SinkChanged { index } => {
                        // TODO: pulse.execute 查询最新 sink info，更新 registry
                        // TODO: 发 D-Bus PropertiesChanged 信号
                        eprintln!("[dde-audio] sink changed: {index}");
                    }
                    PulseEvent::SourceAdded { index } => {
                        // TODO: pulse.execute 查询 source info，写入 registry
                        let mut reg = registry.write();
                        eprintln!("[dde-audio] source added: {index}");
                        let _ = &mut reg;
                    }
                    PulseEvent::SourceRemoved { index } => {
                        let mut reg = registry.write();
                        reg.sources.remove(&index);
                        eprintln!("[dde-audio] source removed: {index}");
                    }
                    PulseEvent::SourceChanged { index } => {
                        // TODO: pulse.execute 查询最新 source info，更新 registry
                        // TODO: 发 D-Bus PropertiesChanged 信号
                        eprintln!("[dde-audio] source changed: {index}");
                    }
                    PulseEvent::SinkInputAdded { index } => {
                        // TODO: pulse.execute 查询 sink input info，写入 registry
                        let mut reg = registry.write();
                        eprintln!("[dde-audio] sink input added: {index}");
                        let _ = &mut reg;
                    }
                    PulseEvent::SinkInputRemoved { index } => {
                        let mut reg = registry.write();
                        reg.sink_inputs.remove(&index);
                        eprintln!("[dde-audio] sink input removed: {index}");
                    }
                    PulseEvent::SinkInputChanged { index } => {
                        // TODO: pulse.execute 查询最新 sink input info，更新 registry
                        eprintln!("[dde-audio] sink input changed: {index}");
                    }
                    PulseEvent::CardAdded { index } => {
                        // TODO: pulse.execute 查询 card info，写入 registry
                        // TODO: 可能触发 default sink/source 重选
                        let mut reg = registry.write();
                        eprintln!("[dde-audio] card added: {index}");
                        let _ = &mut reg;
                    }
                    PulseEvent::CardRemoved { index } => {
                        let mut reg = registry.write();
                        reg.cards.remove(&index);
                        eprintln!("[dde-audio] card removed: {index}");
                        // TODO: 可能触发 default sink/source 重选
                    }
                    PulseEvent::CardChanged { index } => {
                        // TODO: pulse.execute 查询最新 card info，更新 registry
                        eprintln!("[dde-audio] card changed: {index}");
                    }
                    PulseEvent::DefaultSinkChanged { name } => {
                        let mut reg = registry.write();
                        reg.default_sink = Some(name.clone());
                        eprintln!("[dde-audio] default sink changed: {name}");
                        // TODO: 发 D-Bus PropertiesChanged 信号
                    }
                    PulseEvent::DefaultSourceChanged { name } => {
                        let mut reg = registry.write();
                        reg.default_source = Some(name.clone());
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
            // 保持 pulse 引用存活，防止 mainloop 提前析构
            let _ = &pulse;
        });

        Self {
            handle: Some(handle),
        }
    }
}

impl Drop for EventLoop {
    fn drop(&mut self) {
        // channel 关闭后线程自然退出，join 等待结束
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}
