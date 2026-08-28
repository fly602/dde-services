// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 声卡（Card）业务逻辑。
//!
//! 处理 Card 的 new/update/delete，更新 DeviceManager。
//! Card 无独立 D-Bus 对象（信息通过 Audio 主接口的 cards 属性暴露），
//! 因此不需要注册/注销，只需更新状态。

use std::sync::Arc;

use parking_lot::RwLock;

use crate::backend::pulse::card as pulse_card;
use crate::backend::pulse::PulseManager;

use super::device_manager::DeviceManager;

/// Card 新增：查询状态写入 DeviceManager。
pub fn new(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    index: u32,
) -> Result<(), String> {
    let state = pulse_card::query_info(pulse, index)?;
    device_manager.write().add_card(index, state);
    eprintln!("[dde-audio] card new: {index}");
    Ok(())
}

/// Card 更新：查询最新状态写入 DeviceManager。
pub fn update(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    index: u32,
) -> Result<(), String> {
    let state = pulse_card::query_info(pulse, index)?;
    device_manager.write().update_card(index, state);
    eprintln!("[dde-audio] card update: {index}");
    Ok(())
}

/// Card 删除：从 DeviceManager 移除。
pub fn delete(device_manager: &Arc<RwLock<DeviceManager>>, index: u32) {
    device_manager.write().remove_card(index);
    // TODO: 可能触发 default sink/source 重选
    eprintln!("[dde-audio] card delete: {index}");
}

/// 设备创建后检查 pending profile 切换是否完成。
///
/// 若设备所属声卡正在切换 profile，检查所需方向（切换前的 sink/source）
/// 是否都已重建，齐全则通知等待线程并清除 pending。
pub fn on_device_created(
    device_manager: &Arc<RwLock<DeviceManager>>,
    device_index: u32,
    is_sink: bool,
) {
    use super::device_manager::DIRECTION_SINK;
    use super::device_manager::DIRECTION_SOURCE;

    let card_id = {
        let dm = device_manager.read();
        if is_sink {
            dm.sinks.get(&device_index).map(|s| s.card)
        } else {
            dm.sources.get(&device_index).map(|s| s.card)
        }
    };

    let card_id = match card_id {
        Some(cid) => cid,
        None => return,
    };

    // 无 pending profile 则忽略
    if device_manager.read().get_pending_profile(card_id).is_none() {
        return;
    }

    // 检查所需方向是否都已重建
    let all_ready = {
        let dm = device_manager.read();
        let wait = dm.get_pending_profile(card_id);
        match wait {
            Some(wait) => {
                let required = wait.required_directions();
                let sink_ok = required & DIRECTION_SINK == 0
                    || dm.sinks.values().any(|s| s.card == card_id);
                let source_ok = required & DIRECTION_SOURCE == 0
                    || dm.sources.values().any(|s| s.card == card_id);
                sink_ok && source_ok
            }
            None => false,
        }
    };

    if all_ready {
        eprintln!("[dde-audio] complete pending profile: card {card_id}");
        let wait = device_manager.write().take_pending_profile(card_id);
        if let Some(wait) = wait {
            wait.signal();
        }
    }
}
