// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! dde audio Rust 插件入口。
//!
//! 编译为 `cdylib`，由 deepin-service-manager 的 Rust 加载后端通过
//! `DSMRustStartV1` / `DSMRustStopV1` ABI 加载和卸载。
//!
//! 插件使用 zbus 在 session/system bus 上注册 `org.deepin.dde.Audio2` 服务，
//! 持有 blocking Connection 以保持服务存活。

mod abi;
mod backend;
mod manager;

use core::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};

use zbus::blocking::{Connection, connection};

use abi::{PluginContextV1, SESSION_BUS, SYSTEM_BUS, check_abi, service_name};
use manager::audio::Audio;
use manager::{AudioManager, DBUS_PATH};

/// 插件运行时状态，在 Stop 时释放。
struct PluginState {
    connection: Option<Connection>,
}

/// 启动插件：创建 D-Bus 连接、申请服务名、注册 Audio 对象。
fn start_plugin(context: &PluginContextV1) -> Result<PluginState, i32> {
    if !check_abi(context) {
        eprintln!("[dde-audio] unsupported Rust plugin ABI");
        return Err(-1);
    }

    let name = service_name(context).ok_or_else(|| {
        eprintln!("[dde-audio] invalid service name");
        -1
    })?;

    let builder = match context.bus_type {
        SYSTEM_BUS => connection::Builder::system(),
        SESSION_BUS => connection::Builder::session(),
        value => {
            eprintln!("[dde-audio] unsupported D-Bus type: {value}");
            return Err(-1);
        }
    }
    .map_err(|e| {
        eprintln!("[dde-audio] failed to create D-Bus builder: {e}");
        -1
    })?;

    let connection = builder
        .name(name)
        .map_err(|e| {
            eprintln!("[dde-audio] failed to configure D-Bus name: {e}");
            -1
        })?
        .build()
        .map_err(|e| {
            eprintln!("[dde-audio] failed to build D-Bus connection: {e}");
            -1
        })?;

    let manager = AudioManager::new(connection.clone()).map_err(|e| {
        eprintln!("[dde-audio] failed to create audio manager: {e}");
        -1
    })?;
    let manager = std::sync::Arc::new(manager);

    connection
        .object_server()
        .at(DBUS_PATH, Audio::new(manager.clone(), manager.device_manager().clone()))
        .map_err(|e| {
            eprintln!("[dde-audio] failed to register Audio object: {e}");
            -1
        })?;

    Ok(PluginState {
        connection: Some(connection),
    })
}

/// 框架入口：启动插件。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DSMRustStartV1(
    context: *const PluginContextV1,
    plugin_handle: *mut *mut c_void,
) -> i32 {
    if context.is_null() || plugin_handle.is_null() {
        return -1;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let state = start_plugin(unsafe { &*context })?;
        let handle = Box::into_raw(Box::new(state)).cast::<c_void>();
        unsafe {
            *plugin_handle = handle;
        }
        Ok::<(), i32>(())
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(code)) => code,
        Err(_) => -1,
    }
}

/// 框架入口：停止插件，释放 D-Bus 连接。
#[unsafe(no_mangle)]
pub unsafe extern "C" fn DSMRustStopV1(plugin_handle: *mut c_void) -> i32 {
    if plugin_handle.is_null() {
        return -1;
    }

    match catch_unwind(AssertUnwindSafe(|| {
        let mut state = unsafe { Box::from_raw(plugin_handle.cast::<PluginState>()) };
        if let Some(connection) = state.connection.take() {
            connection.close().map_err(|e| {
                eprintln!("[dde-audio] failed to close D-Bus connection: {e}");
                -1
            })?;
        }
        Ok::<(), i32>(())
    })) {
        Ok(Ok(())) => 0,
        Ok(Err(code)) => code,
        Err(_) => -1,
    }
}

#[cfg(test)]
mod tests {
    use super::backend::pulse::{card, sink, sink_input, source, PulseManager};

    /// 连接 pulse daemon，查询所有设备列表。
    /// 需要系统有 PulseAudio 或 pipewire-pulse 运行。
    #[test]
    fn query_all_devices() {
        let (pulse, _rx) = PulseManager::new().expect("connect to pulse daemon");

        let cards = card::query_list(&pulse).expect("query cards");
        let sinks = sink::query_list(&pulse).expect("query sinks");
        let sources = source::query_list(&pulse).expect("query sources");
        let sink_inputs = sink_input::query_list(&pulse).expect("query sink inputs");

        eprintln!("cards: {}", cards.len());
        for c in &cards {
            eprintln!("  card {}: {} profile={}", c.index, c.name, c.active_profile);
        }
        eprintln!("sinks: {}", sinks.len());
        for s in &sinks {
            eprintln!(
                "  sink {}: {} desc={} vol={} mute={} card={}",
                s.index, s.name, s.description, s.volume, s.mute, s.card
            );
        }
        eprintln!("sources: {}", sources.len());
        for s in &sources {
            eprintln!(
                "  source {}: {} desc={} vol={} mute={} card={}",
                s.index, s.name, s.description, s.volume, s.mute, s.card
            );
        }
        eprintln!("sink inputs: {}", sink_inputs.len());
        for si in &sink_inputs {
            eprintln!(
                "  sink input {}: {} vol={} mute={}",
                si.index, si.name, si.volume, si.mute
            );
        }

        // 至少应该有 card 和 sink（物理音频设备）
        assert!(!cards.is_empty(), "should have at least one card");
    }

    /// 验证默认 sink/source 查询。
    #[test]
    fn query_default_sink_source() {
        let (pulse, _rx) = PulseManager::new().expect("connect to pulse daemon");
        let (sink, source) = pulse.default_sink_source().expect("query default sink/source");
        eprintln!("default sink: {sink}");
        eprintln!("default source: {source}");
    }

    /// 验证 source 音量计量：创建 meter，采集后读峰值。
    #[test]
    fn source_meter_peak() {
        let (pulse, _rx) = PulseManager::new().expect("connect to pulse daemon");
        let (_, source_name) = pulse.default_sink_source().expect("query default source");
        assert!(!source_name.is_empty(), "should have a default source");

        let source_index = source::query_list(&pulse)
            .expect("query sources")
            .into_iter()
            .find(|s| s.name == source_name)
            .map(|s| s.index)
            .expect("default source should exist");

        let meter = pulse
            .create_source_meter(source_index)
            .expect("create source meter");

        // 短暂采集（meter 回调在 mainloop 线程，25Hz 采样）
        std::thread::sleep(std::time::Duration::from_millis(500));
        let peak = meter.peak();
        eprintln!("source {source_index} ({source_name}) peak: {peak}");
        // 峰值应落在合法范围 [0, 1]
        assert!((0.0..=1.0).contains(&peak), "peak out of range: {peak}");
    }
}
