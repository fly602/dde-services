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

/// 查询单个声卡信息，构造 `CardState`。
pub fn query_info(
    pulse: &PulseManager,
    index: u32,
) -> Result<crate::manager::device_manager::CardState, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut state: Option<crate::manager::device_manager::CardState> = None;
        ctx.introspect().get_card_info_by_index(index, move |res| {
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
    .ok_or_else(|| format!("card {index} not found"))
}

/// 查询所有声卡信息，返回 `Vec<CardState>`。
pub fn query_list(pulse: &PulseManager) -> Result<Vec<crate::manager::device_manager::CardState>, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut list: Vec<crate::manager::device_manager::CardState> = Vec::new();
        ctx.introspect().get_card_info_list(move |res| {
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

fn state_from_info(info: &libpulse_binding::context::introspect::CardInfo) -> crate::manager::device_manager::CardState {
    use libpulse_binding::def::PortAvailable;
    use libpulse_binding::direction;

    let ports = info
        .ports
        .iter()
        .map(|p| crate::manager::device_manager::CardPortInfo {
            name: p.name.as_deref().unwrap_or("").to_owned(),
            enabled: p.available != PortAvailable::No,
            bluetooth: false,
            description: p.description.as_deref().unwrap_or("").to_owned(),
            direction: if p.direction.contains(direction::FlagSet::OUTPUT) { 0 } else { 1 },
        })
        .collect();

    crate::manager::device_manager::CardState {
        index: info.index,
        name: info.name.as_deref().unwrap_or("").to_owned(),
        active_profile: info.active_profile.as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("")
            .to_owned(),
        ports,
    }
}
