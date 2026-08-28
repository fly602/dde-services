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

/// 声卡端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct PortInfo {
    pub name: String,
    pub enabled: bool,
    pub bluetooth: bool,
    pub description: String,
    pub direction: u32,
    /// 该端口关联的可用 profile 名称。
    pub profiles: Vec<String>,
}

impl PortInfo {
    /// 选择该端口最合适的 profile。
    ///
    /// 当前返回第一个可用的 profile 名称。
    /// TODO: 按 profile 优先级/蓝牙模式选择最优。
    pub fn select_profile(&self) -> Option<&str> {
        self.profiles.first().map(|s| s.as_str())
    }
}
/// 声卡支持的 profile。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Profile {
    pub name: String,
    pub description: String,
    /// 越高越适合作为默认 profile。
    pub priority: u32,
    /// 是否可用（unavailable 的 profile 无意义）。
    pub available: bool,
}
/// 声卡（Card）状态。
///
/// `status`/`change` 是运行时状态。`change` 含 Mutex 不可序列化，
/// 用 `#[serde(skip)]` 跳过（反序列化时为 None）。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Card {
    pub index: u32,
    pub name: String,
    pub active_profile: String,
    pub ports: Vec<PortInfo>,
    /// 该声卡支持的所有 profile。
    pub profiles: Vec<Profile>,
    /// 生命周期状态。
    pub status: CardStatus,
    /// 进行中的状态变更（同步手柄）。
    #[serde(skip)]
    pub change: Option<Arc<StatusChange>>,
}

impl From<crate::backend::pulse::card::BackendCard> for Card {
    fn from(b: crate::backend::pulse::card::BackendCard) -> Self {
        Self {
            index: b.index,
            name: b.name,
            active_profile: b.active_profile,
            ports: b.ports
                .into_iter()
                .map(|p| PortInfo {
                    name: p.name,
                    enabled: p.available,
                    bluetooth: false,
                    description: p.description,
                    direction: p.direction,
                    profiles: p.profiles,
                })
                .collect(),
            profiles: b.profiles
                .into_iter()
                .map(|p| Profile {
                    name: p.name,
                    description: p.description,
                    priority: p.priority,
                    available: p.available,
                })
                .collect(),
            status: CardStatus::Ready,
            change: None,
        }
    }
}

/// 方向掩码：输出。
pub const DIRECTION_SINK: u32 = 1 << 0;
/// 方向掩码：输入。
pub const DIRECTION_SOURCE: u32 = 1 << 1;
/// Card 生命周期状态。
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum CardStatus {
    /// 正常可用。
    Ready,
    /// profile 切换中。
    Pending,
    /// 删除中。
    Removing,
}

/// 一次状态变更（如 profile 切换）的最终结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeResult {
    /// 完成（设备重建齐全）。
    Complete,
    /// 声卡被移除，操作终止。
    Removed,
    /// 失败（当前无生产者，为三态协议预留）。
    #[allow(dead_code)]
    Failed(String),
}

/// 一次状态变更的同步手柄。
///
/// 变更发起方（如 set_profile）阻塞等待，event_loop 检测到
/// 变更完成（设备重建齐全）后通知。
///
/// `required_directions` 记录变更前该声卡存在的设备方向（bit0=输出，bit1=输入），
/// 设备重建后所有方向齐全才认为完成。
///
/// 结果是广播的（`notify_all`），多个等待者可同时收到完成/移除/失败。
pub struct StatusChange {
    result: std::sync::Mutex<Option<ChangeResult>>,
    cond: std::sync::Condvar,
    /// 需要重建的方向掩码：bit0=输出(sink)，bit1=输入(source)。
    required_directions: u32,
}

impl std::fmt::Debug for StatusChange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatusChange")
            .field("required_directions", &self.required_directions)
            .field(
                "result",
                &self.result.lock().map(|r| r.as_ref().cloned()).unwrap_or(None),
            )
            .finish()
    }
}

impl StatusChange {
    /// 创建等待项，指定需要重建的方向。
    pub fn new(required_directions: u32) -> Arc<Self> {
        Arc::new(Self {
            result: std::sync::Mutex::new(None),
            cond: std::sync::Condvar::new(),
            required_directions,
        })
    }

    /// 需要重建的方向掩码。
    pub fn required_directions(&self) -> u32 {
        self.required_directions
    }

    /// 阻塞等待结果，超时返回错误。
    pub fn wait(&self, timeout: std::time::Duration) -> Result<ChangeResult, String> {
        let mut result = self
            .result
            .lock()
            .map_err(|e| format!("mutex poisoned: {e}"))?;
        while result.is_none() {
            let (guard, timeout_result) = self
                .cond
                .wait_timeout(result, timeout)
                .map_err(|e| format!("mutex poisoned: {e}"))?;
            result = guard;
            if timeout_result.timed_out() {
                return Err("profile switch timed out".into());
            }
        }
        Ok(result.clone().unwrap())
    }

    /// 广播完成。
    pub fn signal_complete(&self) {
        self.signal(ChangeResult::Complete);
    }

    /// 广播声卡被移除。
    pub fn signal_removed(&self) {
        self.signal(ChangeResult::Removed);
    }

    /// 广播失败。
    #[allow(dead_code)]
    pub fn signal_failed(&self, error: String) {
        self.signal(ChangeResult::Failed(error));
    }

    fn signal(&self, result: ChangeResult) {
        if let Ok(mut guard) = self.result.lock() {
            if guard.is_none() {
                *guard = Some(result);
                self.cond.notify_all();
            }
        }
    }
}


/// Card 新增：查询状态写入 DeviceManager。
pub fn new(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    index: u32,
) -> Result<(), String> {
    let state: Card = pulse_card::query_info(pulse, index)?.into();
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
    let state: Card = pulse_card::query_info(pulse, index)?.into();
    device_manager.write().update_card(index, state);
    eprintln!("[dde-audio] card update: {index}");
    Ok(())
}

/// Card 删除：从 DeviceManager 移除。
///
/// 若有进行中的 profile 切换操作，广播 Removed 让等待线程结束。
pub fn delete(device_manager: &Arc<RwLock<DeviceManager>>, index: u32) {
    let removed = device_manager.write().remove_card(index);
    if let Some(card) = removed {
        if let Some(op) = card.change {
            eprintln!("[dde-audio] card removed during profile switch: card {index}");
            op.signal_removed();
        }
    }
    // TODO: 可能触发 default sink/source 重选
    eprintln!("[dde-audio] card delete: {index}");
}

/// 设备创建后检查 profile 切换是否完成。
///
/// 若设备所属声卡处于 Pending 状态，检查所需方向（切换前的 sink/source）
/// 是否都已重建，齐全则回 Ready 并通知等待线程。
pub fn on_device_created(
    device_manager: &Arc<RwLock<DeviceManager>>,
    device_index: u32,
    is_sink: bool,
) {
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

    // 非 Pending 状态则忽略
    let all_ready = {
        let dm = device_manager.read();
        let card = match dm.cards.get(&card_id) {
            Some(c) => c,
            None => return,
        };
        if card.status != CardStatus::Pending {
            return;
        }
        let op = match &card.change {
            Some(op) => op,
            None => return,
        };
        let required = op.required_directions();
        let sink_ok = required & DIRECTION_SINK == 0
            || dm.sinks.values().any(|s| s.card == card_id);
        let source_ok = required & DIRECTION_SOURCE == 0
            || dm.sources.values().any(|s| s.card == card_id);
        sink_ok && source_ok
    };

    if all_ready {
        eprintln!("[dde-audio] complete pending profile: card {card_id}");
        let op = device_manager.write().cards.get_mut(&card_id).and_then(|c| c.change.take());
        if let Some(op) = op {
            op.signal_complete();
        }
        if let Some(card) = device_manager.write().cards.get_mut(&card_id) {
            card.status = CardStatus::Ready;
        }
    }
}
