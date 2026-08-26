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
