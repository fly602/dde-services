// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Meter` 接口。
//!
//! 音量计量器对象。`Volume` 属性从 DeviceManager 读取对应设备当前音量，
//! `Tick` 方法保持 meter 存活。真实实时峰值监控后续实现。

use std::sync::Arc;

use parking_lot::RwLock;
use zbus::interface;

use super::device_manager::DeviceManager;

/// Meter D-Bus 对象。
pub struct Meter {
    /// 设备索引（sink 或 source 的 index）。
    device_index: u32,
    /// 是否为 sink（否则为 source）。
    is_sink: bool,
    device_manager: Arc<RwLock<DeviceManager>>,
}

impl Meter {
    /// 构造 Meter 对象。
    pub fn new(
        device_index: u32,
        is_sink: bool,
        device_manager: Arc<RwLock<DeviceManager>>,
    ) -> Self {
        Self {
            device_index,
            is_sink,
            device_manager,
        }
    }

    /// 生成 Meter 的 D-Bus 对象路径。
    pub fn path(device_index: u32, is_sink: bool) -> String {
        let kind = if is_sink { "Sink" } else { "Source" };
        format!("/org/deepin/dde/Audio2/Meter{kind}{device_index}")
    }

    /// 当前设备音量。
    fn current_volume(&self) -> f64 {
        let reg = self.device_manager.read();
        if self.is_sink {
            reg.sinks
                .get(&self.device_index)
                .map(|s| s.volume)
                .unwrap_or_default()
        } else {
            reg.sources
                .get(&self.device_index)
                .map(|s| s.volume)
                .unwrap_or_default()
        }
    }
}

#[interface(name = "org.deepin.dde.Audio2.Meter")]
impl Meter {
    /// 当前音量。
    #[zbus(property)]
    pub fn volume(&self) -> f64 {
        self.current_volume()
    }

    /// 音量计量 tick 方法，保持 meter 存活。
    fn tick(&self) -> zbus::fdo::Result<()> {
        // TODO: 更新存活时间戳
        Ok(())
    }
}
