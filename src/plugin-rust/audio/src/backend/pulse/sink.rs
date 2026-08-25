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

/// 查询 Sink 信息，返回构造 registry 所需字段。
///
/// event_loop 收到 SinkAdded/SinkChanged 时调用此函数，
/// 通过 pulse.execute 查询最新 sink info，构造 SinkState 写入 registry。
#[allow(dead_code)]
pub fn query_info(_pulse: &PulseManager, _index: u32) -> Result<crate::manager::registry::SinkState, String> {
    // TODO: pulse.execute(|ctx, tx| ctx.get_sink_info_by_index(index, callback))
    //       回调中提取字段构造 SinkState
    Err("unimplemented".into())
}
