// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Meter` 接口。
//!
//! 音量计量器对象。生命周期仿 Go 版 `audio1/meter.go`：
//! - 创建时置 alive 并启动清理线程（tryQuit）
//! - `Tick` 方法续命（alive = true）
//! - 清理线程 10 秒轮询，超时未续命则注销对象并释放资源
//!
//! Source 的 `Volume` 来自 backend `SourceMeter` 的实时峰值；
//! Sink 无真实监测（Go 版 Sink.GetMeter 亦为 TODO），Volume 回退读设备静态音量。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use parking_lot::RwLock;
use zbus::interface;

use crate::backend::pulse::meter::SourceMeter;

use super::device_manager::DeviceManager;

/// 续命超时：10 秒内未调用 Tick 则销毁。
const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Meter D-Bus 对象。
///
/// zbus `at()` 注册的是值类型，因此内部可变状态（alive）
/// 用 Arc 原子共享；Clone 得到共享同一份状态的新实例，
/// 清理线程持有一个克隆，D-Bus 注册持有另一个。
#[derive(Clone)]
pub struct Meter {
    /// 登记 id（如 "source3"）。
    id: String,
    /// 设备索引（sink 或 source 的 index）。
    device_index: u32,
    /// 是否为 sink（否则为 source）。
    is_sink: bool,
    /// 续命标记，与清理线程共享。
    alive: Arc<AtomicBool>,
    /// 真实峰值计量（source 有，sink 无）。
    backend: Option<Arc<SourceMeter>>,
    device_manager: Arc<RwLock<DeviceManager>>,
    /// D-Bus 连接，清理时注销自身对象。
    connection: zbus::blocking::Connection,
}

impl Meter {
    /// 构造 Meter，启动清理线程。
    ///
    /// `backend` 为 source 的真实峰值计量；sink 传 None。
    pub fn new(
        id: String,
        device_index: u32,
        is_sink: bool,
        backend: Option<Arc<SourceMeter>>,
        device_manager: Arc<RwLock<DeviceManager>>,
        connection: zbus::blocking::Connection,
    ) -> Arc<Self> {
        let alive = Arc::new(AtomicBool::new(true));
        let meter = Arc::new(Self {
            id,
            device_index,
            is_sink,
            alive: alive.clone(),
            backend,
            device_manager,
            connection,
        });
        // 启动清理线程（仿 Go tryQuit）
        let meter_weak = Arc::downgrade(&meter);
        thread::spawn(move || {
            loop {
                thread::sleep(POLL_INTERVAL);
                if !alive.load(Ordering::Relaxed) {
                    break;
                }
                alive.store(false, Ordering::Relaxed);
            }
            // 超时未续命：注销 D-Bus 对象并从 DeviceManager 移除
            // （Arc 引用降为 0 时 backend SourceMeter 自动释放）
            if let Some(meter) = meter_weak.upgrade() {
                let id = meter.id.clone();
                let path = Meter::path(meter.device_index, meter.is_sink);
                let _ = meter
                    .connection
                    .object_server()
                    .remove::<Meter, _>(path);
                let mut dm = meter.device_manager.write();
                // 仅当仍是同一个实例时移除，避免误删新 meter
                if dm.meters.get(&id).map(|m| Arc::ptr_eq(m, &meter)) == Some(true) {
                    dm.meters.remove(&id);
                }
            }
        });
        meter
    }

    /// 生成 Meter 的 D-Bus 对象路径。
    pub fn path(device_index: u32, is_sink: bool) -> String {
        let kind = if is_sink { "Sink" } else { "Source" };
        format!("/org/deepin/dde/Audio2/Meter{kind}{device_index}")
    }

    /// 当前音量。
    ///
    /// source：backend 实时峰值；sink：读 DeviceManager 静态音量（无真实监测）。
    fn current_volume(&self) -> f64 {
        if let Some(backend) = &self.backend {
            return backend.peak() as f64;
        }
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

    /// 音量计量 tick 方法，续命。
    fn tick(&self) -> zbus::fdo::Result<()> {
        self.alive.store(true, Ordering::Relaxed);
        Ok(())
    }
}
