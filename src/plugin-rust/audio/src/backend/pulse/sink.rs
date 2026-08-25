// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 输出设备（Sink）状态、事件处理和设置操作。
//!
//! 不持有 context，所有操作通过 `PulseManager::execute` 复用。
//! PulseManager 的回调通过 `on_sink_changed` 等函数通知本模块。

use crate::backend::SinkInfo;
use super::PulseManager;

/// Sink 运行时状态。
#[derive(Clone, Debug)]
pub struct SinkState {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
}

impl From<&SinkInfo> for SinkState {
    fn from(info: &SinkInfo) -> Self {
        Self {
            index: info.index,
            name: info.name.clone(),
            description: info.description.clone(),
            mute: info.mute,
            volume: info.volume,
            balance: info.balance,
        }
    }
}

// ========== 事件 handler ==========

/// PulseManager 回调到达时调用，更新 Sink 状态。
#[allow(dead_code)]
pub fn on_sink_changed(_pulse: &PulseManager, _index: u32) {
    // TODO: 通过 pulse.execute 查询最新 sink info，更新 SinkState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_sink_added(_pulse: &PulseManager, _index: u32) {
    // TODO: 创建 SinkState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_sink_removed(_pulse: &PulseManager, _index: u32) {
    // TODO: 删除 SinkState，发事件到 channel
}

// ========== 设置操作 ==========

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
