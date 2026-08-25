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

/// 查询 SinkInput 信息，返回构造 registry 所需字段。
#[allow(dead_code)]
pub fn query_info(_pulse: &PulseManager, _index: u32) -> Result<crate::manager::registry::SinkInputState, String> {
    // TODO: pulse.execute(|ctx, tx| ctx.get_sink_input_info(index, callback))
    Err("unimplemented".into())
}
