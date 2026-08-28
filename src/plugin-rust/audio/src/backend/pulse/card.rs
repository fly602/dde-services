// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

use super::PulseManager;

/// 设置声卡 profile。
#[allow(dead_code)]
pub fn set_card_profile(
    pulse: &PulseManager,
    card_id: u32,
    profile_name: &str,
) -> Result<(), String> {
    let profile_name = profile_name.to_owned();
    let ok: bool = pulse.execute(|ctx, tx| {
        let mut intro = ctx.introspect();
        intro.set_card_profile_by_index(card_id, &profile_name, Some(Box::new(move |ok| {
            let _ = tx.send(ok);
        })));
        true
    })?;
    if !ok {
        return Err(format!("set card profile failed for card {card_id}"));
    }
    Ok(())
}

/// 设置端口启用/禁用。
///
/// libpulse 无直接 API，通过查询声卡端口所属 profile 并切换实现。
/// 端口启用状态（enabled 持久化）由上层 DeviceManager 管理（TODO）。
#[allow(dead_code)]
pub fn set_port_enabled(
    pulse: &PulseManager,
    card_id: u32,
    port_name: &str,
    _enabled: bool,
) -> Result<(), String> {
    use libpulse_binding::callbacks::ListResult;

    let port_name = port_name.to_owned();

    // 查询声卡，找到端口对应的 profile 名称
    let profile_name: Option<String> = pulse.execute(|ctx, tx| {
        let intro = ctx.introspect();
        let mut profile: Option<String> = None;
        let port = port_name.clone();
        intro.get_card_info_by_index(card_id, move |res| {
            match res {
                ListResult::Item(info) => {
                    for p in &info.ports {
                        if p.name.as_deref() == Some(port.as_str()) {
                            if let Some(prof) = p.profiles.first() {
                                profile = prof.name.as_deref().map(|n| n.to_owned());
                            }
                            break;
                        }
                    }
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send(profile.take());
                }
            }
        });
        true
    })?;

    match profile_name {
        Some(name) => set_card_profile(pulse, card_id, &name),
        None => Err(format!("port {port_name} not found on card {card_id}")),
    }
}

/// 查询端口是否启用。
#[allow(dead_code)]
pub fn is_port_enabled(
    pulse: &PulseManager,
    card_id: u32,
    port_name: &str,
) -> Result<bool, String> {
    use libpulse_binding::callbacks::ListResult;

    let port_name = port_name.to_owned();

    pulse.execute(|ctx, tx| {
        let intro = ctx.introspect();
        let mut enabled = false;
        intro.get_card_info_by_index(card_id, move |res| {
            match res {
                ListResult::Item(info) => {
                    for port in &info.ports {
                        if port.name.as_deref() == Some(port_name.as_str()) {
                            enabled = port.available != libpulse_binding::def::PortAvailable::No;
                            break;
                        }
                    }
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send(enabled);
                }
            }
        });
        true
    })
}

/// 查询单个声卡信息，构造 `Card`。
pub fn query_info(
    pulse: &PulseManager,
    index: u32,
) -> Result<crate::manager::card::Card, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut state: Option<crate::manager::card::Card> = None;
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

/// 查询所有声卡信息，返回 `Vec<Card>`。
pub fn query_list(pulse: &PulseManager) -> Result<Vec<crate::manager::card::Card>, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut list: Vec<crate::manager::card::Card> = Vec::new();
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

fn state_from_info(info: &libpulse_binding::context::introspect::CardInfo) -> crate::manager::card::Card {
    use libpulse_binding::def::PortAvailable;
    use libpulse_binding::direction;

    let ports = info
        .ports
        .iter()
        .map(|p| crate::manager::card::CardPortInfo {
            name: p.name.as_deref().unwrap_or("").to_owned(),
            enabled: p.available != PortAvailable::No,
            bluetooth: false,
            description: p.description.as_deref().unwrap_or("").to_owned(),
            direction: if p.direction.contains(direction::FlagSet::OUTPUT) { 0 } else { 1 },
            profiles: p.profiles.iter()
                .filter_map(|prof| prof.name.as_deref().map(|n| n.to_owned()))
                .collect(),
        })
        .collect();

    let profiles = info
        .profiles
        .iter()
        .map(|p| crate::manager::card::CardProfile {
            name: p.name.as_deref().unwrap_or("").to_owned(),
            description: p.description.as_deref().unwrap_or("").to_owned(),
            priority: p.priority,
            available: p.available,
        })
        .collect();

    crate::manager::card::Card {
        index: info.index,
        name: info.name.as_deref().unwrap_or("").to_owned(),
        active_profile: info.active_profile.as_ref()
            .and_then(|p| p.name.as_deref())
            .unwrap_or("")
            .to_owned(),
        ports,
        profiles,
    }
}
