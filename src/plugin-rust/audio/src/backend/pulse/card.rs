// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 声卡设置操作。
//!
//! 纯操作函数，不保存状态。状态管理在 `manager::registry`。
//! 所有操作通过 `PulseManager::execute` 复用。

use super::PulseManager;

/// 设置声卡 profile。
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

/// 查询声卡信息，返回构造 registry 所需字段。
#[allow(dead_code)]
pub fn query_info(_pulse: &PulseManager, _index: u32) -> Result<crate::manager::registry::CardState, String> {
    // TODO: pulse.execute(|ctx, tx| ctx.get_card_info_by_index(index, callback))
    Err("unimplemented".into())
}
