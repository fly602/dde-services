// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 输入设备（Source）状态、事件处理和设置操作。
//!
//! 不持有 context，所有操作通过 `PulseManager::execute` 复用。
//! PulseManager 的回调通过 `on_source_changed` 等函数通知本模块。

use crate::backend::SourceInfo;
use super::PulseManager;

/// Source 运行时状态。
#[derive(Clone, Debug)]
pub struct SourceState {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
}

impl From<&SourceInfo> for SourceState {
    fn from(info: &SourceInfo) -> Self {
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

/// PulseManager 回调到达时调用，更新 Source 状态。
#[allow(dead_code)]
pub fn on_source_changed(_pulse: &PulseManager, _index: u32) {
    // TODO: 通过 pulse.execute 查询最新 source info，更新 SourceState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_source_added(_pulse: &PulseManager, _index: u32) {
    // TODO: 创建 SourceState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_source_removed(_pulse: &PulseManager, _index: u32) {
    // TODO: 删除 SourceState，发事件到 channel
}

// ========== 设置操作 ==========

/// 设置 Source 音量。
#[allow(dead_code)]
pub fn set_volume(
    _pulse: &PulseManager,
    _index: u32,
    _value: f64,
    _is_play: bool,
) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_source_volume_by_index(index, &cvol, callback))
    Err("unimplemented".into())
}

/// 设置 Source 静音。
#[allow(dead_code)]
pub fn set_mute(_pulse: &PulseManager, _index: u32, _value: bool) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_source_mute_by_index(index, value, callback))
    Err("unimplemented".into())
}

/// 设置 Source 左右声道平衡。
#[allow(dead_code)]
pub fn set_balance(
    _pulse: &PulseManager,
    _index: u32,
    _value: f64,
    _is_play: bool,
) -> Result<(), String> {
    // TODO: 计算新 cvol 后 set_source_volume_by_index
    Err("unimplemented".into())
}

/// 设置 Source 前后声道平衡。
#[allow(dead_code)]
pub fn set_fade(_pulse: &PulseManager, _index: u32, _value: f64) -> Result<(), String> {
    // TODO: 计算新 cvol 后 set_source_volume_by_index
    Err("unimplemented".into())
}

/// 设置 Source 端口。
#[allow(dead_code)]
pub fn set_port(_pulse: &PulseManager, _index: u32, _name: &str) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_source_port_by_index(index, name, callback))
    Err("unimplemented".into())
}

/// 获取 Source 音量计量器。
#[allow(dead_code)]
pub fn get_meter(_pulse: &PulseManager, _index: u32) -> Result<u32, String> {
    // TODO: 创建 record stream 作为 meter
    Err("unimplemented".into())
}
