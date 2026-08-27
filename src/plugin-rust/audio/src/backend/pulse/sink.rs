// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 输出设备（Sink）设置操作与查询。
//!
//! 纯操作函数，不保存状态。状态管理在 `manager`。
//! 所有操作通过 `PulseManager::execute` 复用。

use libpulse_binding::volume::Volume;
use libpulse_binding::volume::ChannelVolumes;

use super::PulseManager;

// ========== 设置操作 ==========

/// 将 UI 音量值（0.0~1.0）转为 PulseAudio Volume。
fn volume_from_float(value: f64) -> Volume {
    Volume((value * Volume::NORMAL.0 as f64) as u32)
}

/// 构造所有声道同一音量的 ChannelVolumes。
fn cvolume_from_float(value: f64, channels: u8) -> ChannelVolumes {
    let mut cv = ChannelVolumes::default();
    cv.set_len(channels);
    cv.set(channels, volume_from_float(value));
    cv
}

/// 设置 Sink 音量。
///
/// value == 0 时静音，否则取消静音（与 Go 版行为一致）。
pub fn set_volume(
    pulse: &PulseManager,
    index: u32,
    value: f64,
    _is_play: bool,
) -> Result<(), String> {
    pulse.execute(|ctx, tx| {
        let mut intro = ctx.introspect();
        let cv = cvolume_from_float(value, 2);
        intro.set_sink_volume_by_index(index, &cv, Some(Box::new(move |ok| {
            let _ = tx.send(ok);
        })));
        true
    })?;
    Ok(())
}

/// 设置 Sink 静音。
pub fn set_mute(pulse: &PulseManager, index: u32, value: bool) -> Result<(), String> {
    pulse.execute(|ctx, tx| {
        let mut intro = ctx.introspect();
        intro.set_sink_mute_by_index(index, value, Some(Box::new(move |ok| {
            let _ = tx.send(ok);
        })));
        true
    })?;
    Ok(())
}

/// 设置 Sink 左右声道平衡。
///
/// 查询设备当前的 ChannelVolumes 和 ChannelMap，调整后再设置。
pub fn set_balance(
    pulse: &PulseManager,
    index: u32,
    value: f64,
    _is_play: bool,
) -> Result<(), String> {
    use libpulse_binding::callbacks::ListResult;
    use libpulse_binding::volume::ChannelVolumes;

    // 查询当前 volume 和 channel_map（回调 End 时 send 结果）
    let (volume, map) = pulse.execute(|ctx, tx| {
        let intro = ctx.introspect();
        let mut volume: Option<ChannelVolumes> = None;
        let mut map: Option<libpulse_binding::channelmap::Map> = None;
        intro.get_sink_info_by_index(index, move |res| {
            match res {
                ListResult::Item(info) => {
                    volume = Some(info.volume.clone());
                    map = Some(info.channel_map.clone());
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send((volume.take(), map.take()));
                }
            }
        });
        true
    })?;

    let (mut volume, map) = match (volume, map) {
        (Some(v), Some(m)) => (v, m),
        _ => return Err("get sink volume info failed".into()),
    };
    volume.set_balance(&map, value as f32);

    // 设置调整后的音量
    let result: bool = pulse.execute(|ctx, tx| {
        let mut intro = ctx.introspect();
        intro.set_sink_volume_by_index(index, &volume, Some(Box::new(move |ok| {
            let _ = tx.send(ok);
        })));
        true
    })?;
    if !result {
        return Err("set balance failed".into());
    }
    Ok(())
}

/// 设置 Sink 前后声道平衡。
pub fn set_fade(pulse: &PulseManager, index: u32, value: f64) -> Result<(), String> {
    use libpulse_binding::callbacks::ListResult;
    use libpulse_binding::volume::ChannelVolumes;

    let (volume, map) = pulse.execute(|ctx, tx| {
        let intro = ctx.introspect();
        let mut volume: Option<ChannelVolumes> = None;
        let mut map: Option<libpulse_binding::channelmap::Map> = None;
        intro.get_sink_info_by_index(index, move |res| {
            match res {
                ListResult::Item(info) => {
                    volume = Some(info.volume.clone());
                    map = Some(info.channel_map.clone());
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send((volume.take(), map.take()));
                }
            }
        });
        true
    })?;

    let (mut volume, map) = match (volume, map) {
        (Some(v), Some(m)) => (v, m),
        _ => return Err("get sink volume info failed".into()),
    };
    volume.set_fade(&map, value as f32);

    let result: bool = pulse.execute(|ctx, tx| {
        let mut intro = ctx.introspect();
        intro.set_sink_volume_by_index(index, &volume, Some(Box::new(move |ok| {
            let _ = tx.send(ok);
        })));
        true
    })?;
    if !result {
        return Err("set fade failed".into());
    }
    Ok(())
}

/// 设置 Sink 端口。
pub fn set_port(pulse: &PulseManager, index: u32, name: &str) -> Result<(), String> {
    let name = name.to_owned();
    pulse.execute(|ctx, tx| {
        let mut intro = ctx.introspect();
        intro.set_sink_port_by_index(index, &name, Some(Box::new(move |ok| {
            let _ = tx.send(ok);
        })));
        true
    })?;
    Ok(())
}

/// 获取 Sink 音量计量器。
#[allow(dead_code)]
pub fn get_meter(_pulse: &PulseManager, _index: u32) -> Result<u32, String> {
    // TODO: 创建 record stream 作为 meter
    Err("unimplemented".into())
}

// ========== 查询 ==========

/// 查询单个 Sink 信息，构造 `SinkState`。
pub fn query_info(
    pulse: &PulseManager,
    index: u32,
) -> Result<crate::manager::device_manager::SinkState, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut state: Option<crate::manager::device_manager::SinkState> = None;
        ctx.introspect().get_sink_info_by_index(index, move |res| {
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
    .ok_or_else(|| format!("sink {index} not found"))
}

/// 查询所有 Sink 信息，返回 `Vec<SinkState>`。
pub fn query_list(pulse: &PulseManager) -> Result<Vec<crate::manager::device_manager::SinkState>, String> {
    use libpulse_binding::callbacks::ListResult;

    pulse.execute(|ctx, tx| {
        let mut list: Vec<crate::manager::device_manager::SinkState> = Vec::new();
        ctx.introspect().get_sink_info_list(move |res| {
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

fn state_from_info(info: &libpulse_binding::context::introspect::SinkInfo) -> crate::manager::device_manager::SinkState {
    let ports = info
        .ports
        .iter()
        .map(|p| crate::manager::device_manager::Port {
            name: p.name.as_deref().unwrap_or("").to_owned(),
            description: p.description.as_deref().unwrap_or("").to_owned(),
            direction: 0, // sink 方向固定为输出
        })
        .collect();

    let active_port = info.active_port.as_ref().map(|p| crate::manager::device_manager::Port {
        name: p.name.as_deref().unwrap_or("").to_owned(),
        description: p.description.as_deref().unwrap_or("").to_owned(),
        direction: 0,
    }).unwrap_or_else(|| crate::manager::device_manager::Port {
        name: String::new(),
        description: String::new(),
        direction: 0,
    });

    let vol = info.volume.avg();
    let base = info.base_volume;

    crate::manager::device_manager::SinkState {
        index: info.index,
        name: info.name.as_deref().unwrap_or("").to_owned(),
        description: info.description.as_deref().unwrap_or("").to_owned(),
        base_volume: base.0 as f64 / Volume::NORMAL.0 as f64,
        mute: info.mute,
        volume: vol.0 as f64 / Volume::NORMAL.0 as f64,
        balance: info.volume.get_balance(&info.channel_map) as f64,
        support_balance: true,
        fade: info.volume.get_fade(&info.channel_map) as f64,
        support_fade: true,
        ports,
        active_port,
        card: info.card.unwrap_or(u32::MAX),
    }
}
