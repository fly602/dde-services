// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 声卡状态、事件处理和设置操作。
//!
//! 不持有 context，所有操作通过 `PulseManager::execute` 复用。
//! PulseManager 的回调通过 `on_card_changed` 等函数通知本模块。

use super::PulseManager;

/// 声卡运行时状态。
#[derive(Clone, Debug)]
pub struct CardState {
    pub index: u32,
    pub name: String,
    pub active_profile: String,
}

// ========== 事件 handler ==========

/// PulseManager 回调到达时调用，更新声卡状态。
#[allow(dead_code)]
pub fn on_card_changed(_pulse: &PulseManager, _index: u32) {
    // TODO: 通过 pulse.execute 查询最新 card info，更新 CardState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_card_added(_pulse: &PulseManager, _index: u32) {
    // TODO: 创建 CardState，发事件到 channel
}

#[allow(dead_code)]
pub fn on_card_removed(_pulse: &PulseManager, _index: u32) {
    // TODO: 删除 CardState，发事件到 channel
}

// ========== 设置操作 ==========

/// 设置声卡 profile。 vb
#[allow(dead_code)]
pub fn set_card_profile(
    _pulse: &PulseManager,
    _card_id: u32,
    _profile_name: &str,
) -> Result<(), String> {
    // TODO: pulse.execute(|ctx, tx| ctx.set_card_profile_by_index(...))
    Err("unimplemented".into())
}

/// 设置端口启用/禁用。
#[allow(dead_code)]
pub fn set_port_enabled(
    _pulse: &PulseManager,
    _card_id: u32,
    _port_name: &str,
    _enabled: bool,
) -> Result<(), String> {
    // TODO: 通过 card ext-port 或 profile 切换实现
    Err("unimplemented".into())
}

/// 查询端口是否启用。
#[allow(dead_code)]
pub fn is_port_enabled(
    _pulse: &PulseManager,
    _card_id: u32,
    _port_name: &str,
) -> Result<bool, String> {
    // TODO: 查询 card port 状态
    Err("unimplemented".into())
}
