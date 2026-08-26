// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 输出设备（Sink）设置操作。
//!
//! 纯操作函数，不保存状态。状态管理在 `manager::registry`。
//! 所有操作通过 `PulseManager::execute` 复用。

use super::PulseManager;

/// 设置 Sink 音量。
#[allow(dead_code)]
pub fn set_volume(
    _pulse: &PulseManager,
    _index: u32,
    _value: f64,
    _is_play: bool,
) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_sink_volume_by_index(index, &cvol, callback))
    Err("unimplemented".into())
}

/// 设置 Sink 静音。
#[allow(dead_code)]
pub fn set_mute(_pulse: &PulseManager, _index: u32, _value: bool) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_sink_mute_by_index(index, value, callback))
    Err("unimplemented".into())
}

/// 设置 Sink 左右声道平衡。
#[allow(dead_code)]
pub fn set_balance(
    _pulse: &PulseManager,
    _index: u32,
    _value: f64,
    _is_play: bool,
) -> Result<(), String> {
    // TODO: 计算新 cvol 后 set_sink_volume_by_index
    Err("unimplemented".into())
}

/// 设置 Sink 前后声道平衡。
#[allow(dead_code)]
pub fn set_fade(_pulse: &PulseManager, _index: u32, _value: f64) -> Result<(), String> {
    // TODO: 计算新 cvol 后 set_sink_volume_by_index
    Err("unimplemented".into())
}

/// 设置 Sink 端口。
#[allow(dead_code)]
pub fn set_port(_pulse: &PulseManager, _index: u32, _name: &str) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_sink_port_by_index(index, name, callback))
    Err("unimplemented".into())
}

/// 获取 Sink 音量计量器。
#[allow(dead_code)]
pub fn get_meter(_pulse: &PulseManager, _index: u32) -> Result<u32, String> {
    // TODO: 创建 record stream 作为 meter
    Err("unimplemented".into())
}

/// 查询单个 Sink 信息，构造 `SinkState`。
pub fn query_info(
    pulse: &PulseManager,
    index: u32,
) -> Result<crate::manager::device_manager::SinkState, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut state: Option<crate::manager::device_manager::SinkState> = None;
        ctx.introspect().get_sink_info_by_index(index, move |res| {
            match res {
                ListResult::Item(info) => {
                    state = Some(state_from_info(info));
                }
                ListResult::End => {
                    let _ = tx.send(state.take());
                }
                ListResult::Error => {
                    let _ = tx.send(None);
                }
            }
        });
        true
    })?
    .ok_or_else(|| format!("sink {index} not found"))
}

/// 查询所有 Sink 信息，返回 `Vec<SinkState>`。
pub fn query_list(pulse: &PulseManager) -> Result<Vec<crate::manager::device_manager::SinkState>, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut list: Vec<crate::manager::device_manager::SinkState> = Vec::new();
        ctx.introspect().get_sink_info_list(move |res| {
            match res {
                ListResult::Item(info) => {
                    list.push(state_from_info(info));
                }
                ListResult::End => {
                    let _ = tx.send(std::mem::take(&mut list));
                }
                ListResult::Error => {
                    let _ = tx.send(Vec::new());
                }
            }
        });
        true
    })
}

fn state_from_info(info: &libpulse_binding::context::introspect::SinkInfo) -> crate::manager::device_manager::SinkState {
    use libpulse_binding::volume::Volume;

    let ports = info
        .ports
        .iter()
        .map(|p| crate::manager::device_manager::Port {
            name: p.name.as_deref().unwrap_or("").to_owned(),
            description: p.description.as_deref().unwrap_or("").to_owned(),
            direction: 0, // sink 方向固定为输出
        })
        .collect();

    let active_port = info.active_port.as_ref().map(|p| crate::manager::device_manager::Port {
        name: p.name.as_deref().unwrap_or("").to_owned(),
        description: p.description.as_deref().unwrap_or("").to_owned(),
        direction: 0,
    }).unwrap_or_else(|| crate::manager::device_manager::Port {
        name: String::new(),
        description: String::new(),
        direction: 0,
    });

    let vol = info.volume.avg();
    let base = info.base_volume;

    crate::manager::device_manager::SinkState {
        index: info.index,
        name: info.name.as_deref().unwrap_or("").to_owned(),
        description: info.description.as_deref().unwrap_or("").to_owned(),
        base_volume: base.0 as f64 / Volume::NORMAL.0 as f64,
        mute: info.mute,
        volume: vol.0 as f64 / Volume::NORMAL.0 as f64,
        balance: info.volume.get_balance(&info.channel_map) as f64,
        support_balance: true,
        fade: info.volume.get_fade(&info.channel_map) as f64,
        support_fade: true,
        ports,
        active_port,
        card: info.card.unwrap_or(u32::MAX),
    }
}
