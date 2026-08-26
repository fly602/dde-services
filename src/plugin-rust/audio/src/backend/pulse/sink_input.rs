// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 播放流（SinkInput）设置操作。
//!
//! 纯操作函数，不保存状态。状态管理在 `manager::registry`。
//! 所有操作通过 `PulseManager::execute` 复用。

use super::PulseManager;

/// 设置 SinkInput 音量。
#[allow(dead_code)]
pub fn set_volume(
    _pulse: &PulseManager,
    _index: u32,
    _value: f64,
    _is_play: bool,
) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_sink_input_volume(index, &cvol, callback))
    Err("unimplemented".into())
}

/// 设置 SinkInput 静音。
#[allow(dead_code)]
pub fn set_mute(_pulse: &PulseManager, _index: u32, _value: bool) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_sink_input_mute(index, value, callback))
    Err("unimplemented".into())
}

/// 设置 SinkInput 左右声道平衡。
#[allow(dead_code)]
pub fn set_balance(
    _pulse: &PulseManager,
    _index: u32,
    _value: f64,
    _is_play: bool,
) -> Result<(), String> {
    // TODO: 计算新 cvol 后 set_sink_input_volume
    Err("unimplemented".into())
}

/// 设置 SinkInput 前后声道平衡。
#[allow(dead_code)]
pub fn set_fade(_pulse: &PulseManager, _index: u32, _value: f64) -> Result<(), String> {
    // TODO: 计算新 cvol 后 set_sink_input_volume
    Err("unimplemented".into())
}

/// 查询单个 SinkInput 信息，构造 `SinkInputState`。
pub fn query_info(
    pulse: &PulseManager,
    index: u32,
) -> Result<crate::manager::device_manager::SinkInputState, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut state: Option<crate::manager::device_manager::SinkInputState> = None;
        ctx.introspect().get_sink_input_info(index, move |res| {
            match res {
                ListResult::Item(info) => {
                    state = Some(state_from_info(info));
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send(state.take());
                }
            }
        });
        true
    })?
    .ok_or_else(|| format!("sink input {index} not found"))
}

/// 查询所有 SinkInput 信息，返回 `Vec<SinkInputState>`。
pub fn query_list(pulse: &PulseManager) -> Result<Vec<crate::manager::device_manager::SinkInputState>, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut list: Vec<crate::manager::device_manager::SinkInputState> = Vec::new();
        ctx.introspect().get_sink_input_info_list(move |res| {
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

fn state_from_info(info: &libpulse_binding::context::introspect::SinkInputInfo) -> crate::manager::device_manager::SinkInputState {
    use libpulse_binding::volume::Volume;

    let vol = info.volume.avg();

    crate::manager::device_manager::SinkInputState {
        index: info.index,
        name: info.name.as_deref().unwrap_or("").to_owned(),
        mute: info.mute,
        volume: vol.0 as f64 / Volume::NORMAL.0 as f64,
        balance: info.volume.get_balance(&info.channel_map) as f64,
        support_balance: true,
        fade: info.volume.get_fade(&info.channel_map) as f64,
        support_fade: true,
    }
}
