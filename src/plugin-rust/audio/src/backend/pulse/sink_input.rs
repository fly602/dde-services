// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 播放流（SinkInput）状态、事件处理和设置操作。
//!
//! 不持有 context，所有操作通过 `PulseManager::execute` 复用。
//! PulseManager 的回调通过 `on_sink_input_changed` 等函数通知本模块。

use crate::backend::SinkInputInfo;
use super::PulseManager;

/// SinkInput 运行时状态。
#[derive(Clone, Debug)]
pub struct SinkInputState {
    pub index: u32,
    pub name: String,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
}

impl From<&SinkInputInfo> for SinkInputState {
    fn from(info: &SinkInputInfo) -> Self {
        Self {
            index: info.index,
            name: info.name.clone(),
            mute: info.mute,
            volume: info.volume,
            balance: info.balance,
        }
    }
}

// ========== 事件 handler ==========

/// PulseManager 回调到达时调用，更新 SinkInput 状态。
#[allow(dead_code)]
pub fn on_sink_input_changed(_pulse: &PulseManager, _index: u32) {
    // TODO: 通过 pulse.execute 查询最新 sink input info，更新 SinkInputState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_sink_input_added(_pulse: &PulseManager, _index: u32) {
    // TODO: 创建 SinkInputState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_sink_input_removed(_pulse: &PulseManager, _index: u32) {
    // TODO: 删除 SinkInputState，发事件到 channel
}

// ========== 设置操作 ==========

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
