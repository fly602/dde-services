// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 设备管理器。
//!
//! 管理音频设备状态的内存存储。event_loop 通过各子模块的 new/update/delete
//! 处理事件后，调用 DeviceManager 的方法增删改四张表。
//! D-Bus 接口层通过 `Arc<RwLock<DeviceManager>>` 读取状态。
//!
//! 职责分工：
//! - DeviceManager — 管理四张 HashMap 表，提供增删查入口
//! - 子 device（sink.rs/source.rs/card.rs/sink_input.rs）— 处理 new/update
//!   的具体逻辑（查 pulse 构造状态），delete 有回收也在子 device 处理

use std::collections::HashMap;
use std::sync::Arc;

/// 音频端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct Port {
    pub name: String,
    pub description: String,
    pub direction: u32,
}

/// 声卡端口信息。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct CardPortInfo {
    pub name: String,
    pub enabled: bool,
    pub bluetooth: bool,
    pub description: String,
    pub direction: u32,
    /// 该端口关联的可用 profile 名称。
    pub profiles: Vec<String>,
}

impl CardPortInfo {
    /// 选择该端口最合适的 profile。
    ///
    /// 当前返回第一个可用的 profile 名称。
    /// TODO: 按 profile 优先级/蓝牙模式选择最优。
    pub fn select_profile(&self) -> Option<&str> {
        self.profiles.first().map(|s| s.as_str())
    }
}
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
#[allow(dead_code)]
pub struct CardInfo {
    pub id: u32,
    pub name: String,
    pub ports: Vec<CardPortInfo>,
}

/// Sink（输出设备）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SinkState {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub base_volume: f64,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
    pub ports: Vec<Port>,
    pub active_port: Port,
    pub card: u32,
}

/// Source（输入设备）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SourceState {
    pub index: u32,
    pub name: String,
    pub description: String,
    pub base_volume: f64,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
    pub ports: Vec<Port>,
    pub active_port: Port,
    pub card: u32,
}

/// SinkInput（播放流）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct SinkInputState {
    pub index: u32,
    pub name: String,
    pub mute: bool,
    pub volume: f64,
    pub balance: f64,
    pub support_balance: bool,
    pub fade: f64,
    pub support_fade: bool,
}

/// Card（声卡）状态。
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, zbus::zvariant::Type)]
pub struct CardState {
    pub index: u32,
    pub name: String,
    pub active_profile: String,
    pub ports: Vec<CardPortInfo>,
}
/// card 级别 profile 切换等待项。
///
/// set_profile 阻塞等待，event_loop 设备创建完成后通知。
/// pending 操作的最终结果。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PendingResult {
    /// 完成（设备重建齐全）。
    Complete,
    /// 声卡被移除，操作终止。
    CardRemoved,
    /// 失败（当前无生产者，为三态协议预留）。
    #[allow(dead_code)]
    Failed(String),
}

/// card 级别 profile 切换等待项。
///
/// set_profile 阻塞等待，event_loop 设备创建完成后通知。
///
/// `required_directions` 记录切换前该声卡存在的设备方向（bit0=输出，bit1=输入），
/// 设备重建后所有方向齐全才认为完成。
///
/// 结果是广播的（`notify_all`），多个等待者可同时收到 complete/card removed。
pub struct PendingProfile {
    result: std::sync::Mutex<Option<PendingResult>>,
    cond: std::sync::Condvar,
    /// 需要重建的方向掩码：bit0=输出(sink)，bit1=输入(source)。
    required_directions: u32,
}

/// 方向掩码：输出。
pub const DIRECTION_SINK: u32 = 1 << 0;
/// 方向掩码：输入。
pub const DIRECTION_SOURCE: u32 = 1 << 1;

impl PendingProfile {
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
    pub fn wait(&self, timeout: std::time::Duration) -> Result<PendingResult, String> {
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
        self.signal(PendingResult::Complete);
    }

    /// 广播声卡被移除。
    pub fn signal_removed(&self) {
        self.signal(PendingResult::CardRemoved);
    }
    /// 广播失败。
    #[allow(dead_code)]
    pub fn signal_failed(&self, error: String) {
        self.signal(PendingResult::Failed(error));
    }

    fn signal(&self, result: PendingResult) {
        if let Ok(mut guard) = self.result.lock() {
            if guard.is_none() {
                *guard = Some(result);
                self.cond.notify_all();
            }
        }
    }
}


// ===== PortType 常量（与 Go 版 priority_policy.go 一致） =====

const PORT_TYPE_BLUETOOTH: u32 = 0;
const PORT_TYPE_HEADSET: u32 = 1;
const PORT_TYPE_USB: u32 = 2;
const PORT_TYPE_BUILTIN: u32 = 3;
const PORT_TYPE_HDMI: u32 = 4;
const PORT_TYPE_LINE_IO: u32 = 5;
#[allow(dead_code)]
const PORT_TYPE_MULTI_CHANNEL: u32 = 6;
const PORT_TYPE_UNKNOWN: u32 = 7;

/// 判断端口名称/声卡名是否包含关键字（不区分大小写）。
fn contains_keyword(card_name: &str, port_name: &str, keyword: &str) -> bool {
    card_name.to_lowercase().contains(keyword)
        || port_name.to_lowercase().contains(keyword)
}

/// 图标端口类型（与 Go 版 GetIconPortType 一致）。
///
/// 顺序：LineIO > Builtin > Headset > Hdmi > Bluetooth > Usb > Unknown
fn get_icon_port_type(card_name: &str, port_name: &str) -> u32 {
    // 每个 (类型, 关键字列表)
    let map: &[(u32, &[&str])] = &[
        (PORT_TYPE_LINE_IO, &["linein", "lineout"]),
        (PORT_TYPE_BUILTIN, &["speaker", "input-mic"]),
        (PORT_TYPE_HEADSET, &["rear-mic", "front-mic", "headset", "headphone"]),
        (PORT_TYPE_HDMI, &["hdmi"]),
        (PORT_TYPE_BLUETOOTH, &["bluez", "bluetooth"]),
        (PORT_TYPE_USB, &["usb"]),
    ];
    for &(t, keywords) in map {
        for &k in keywords {
            if contains_keyword(card_name, port_name, k) {
                return t;
            }
        }
    }
    PORT_TYPE_UNKNOWN
}
/// Cards 属性 JSON 序列化结构，字段名与 Go 版兼容。
#[derive(serde::Serialize)]
struct CardExport<'a> {
    #[serde(rename = "Id")]
    id: u32,
    #[serde(rename = "Name")]
    name: &'a str,
    #[serde(rename = "Ports")]
    ports: Vec<CardPortExport>,
}

/// Cards 属性端口序列化结构。
#[derive(serde::Serialize)]
struct CardPortExport {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "Enabled")]
    enabled: bool,
    #[serde(rename = "Bluetooth")]
    bluetooth: bool,
    #[serde(rename = "Description")]
    description: String,
    #[serde(rename = "Direction")]
    direction: u32,
    #[serde(rename = "PortType")]
    port_type: u32,
}

/// 设备管理器。
///
/// 持有四张 HashMap 表管理设备状态，通过 `Arc<RwLock<DeviceManager>>` 共享。
/// event_loop 写入，D-Bus 接口层读取。
#[derive(Default)]
pub struct DeviceManager {
    pub sinks: HashMap<u32, SinkState>,
    pub sources: HashMap<u32, SourceState>,
    pub sink_inputs: HashMap<u32, SinkInputState>,
    pub cards: HashMap<u32, CardState>,
    pub default_sink: Option<String>,
    pub default_source: Option<String>,
    /// 正在切换 profile 的声卡：card_id → 等待项。
    pub pending_profiles: HashMap<u32, Arc<PendingProfile>>,
    /// PulseAudio 模块状态：module 名 → 状态。
    pub modules: HashMap<String, crate::backend::pulse::module::ModuleState>,
}
impl DeviceManager {
    // ===== Sink =====

    pub fn add_sink(&mut self, index: u32, state: SinkState) {
        self.sinks.insert(index, state);
    }

    pub fn update_sink(&mut self, index: u32, state: SinkState) {
        self.sinks.insert(index, state);
    }

    pub fn remove_sink(&mut self, index: u32) -> Option<SinkState> {
        self.sinks.remove(&index)
    }

#[allow(dead_code)]
    pub fn get_sink(&self, index: u32) -> Option<&SinkState> {
        self.sinks.get(&index)
    }

    // ===== Source =====

    pub fn add_source(&mut self, index: u32, state: SourceState) {
        self.sources.insert(index, state);
    }

    pub fn update_source(&mut self, index: u32, state: SourceState) {
        self.sources.insert(index, state);
    }

    pub fn remove_source(&mut self, index: u32) -> Option<SourceState> {
        self.sources.remove(&index)
    }

#[allow(dead_code)]
    pub fn get_source(&self, index: u32) -> Option<&SourceState> {
        self.sources.get(&index)
    }

    // ===== SinkInput =====

    pub fn add_sink_input(&mut self, index: u32, state: SinkInputState) {
        self.sink_inputs.insert(index, state);
    }

    pub fn update_sink_input(&mut self, index: u32, state: SinkInputState) {
        self.sink_inputs.insert(index, state);
    }

    pub fn remove_sink_input(&mut self, index: u32) -> Option<SinkInputState> {
        self.sink_inputs.remove(&index)
    }

#[allow(dead_code)]
    pub fn get_sink_input(&self, index: u32) -> Option<&SinkInputState> {
        self.sink_inputs.get(&index)
    }

    // ===== Card =====

    pub fn add_card(&mut self, index: u32, state: CardState) {
        self.cards.insert(index, state);
    }

    pub fn update_card(&mut self, index: u32, state: CardState) {
        self.cards.insert(index, state);
    }

    pub fn remove_card(&mut self, index: u32) -> Option<CardState> {
        self.cards.remove(&index)
    }

#[allow(dead_code)]
    pub fn get_card(&self, index: u32) -> Option<&CardState> {
        self.cards.get(&index)
    }

    // ===== Default =====

    pub fn set_default_sink(&mut self, name: String) {
        self.default_sink = Some(name);
    }

    pub fn set_default_source(&mut self, name: String) {
        self.default_source = Some(name);
    }

    // ===== 查询辅助 =====

    /// 按名称查找 Sink 索引。
    pub fn find_sink_index_by_name(&self, name: &str) -> Option<u32> {
        self.sinks.values().find(|s| s.name == name).map(|s| s.index)
    }

    /// 按名称查找 Source 索引。
    pub fn find_source_index_by_name(&self, name: &str) -> Option<u32> {
        self.sources.values().find(|s| s.name == name).map(|s| s.index)
    }

    /// 序列化声卡列表为 JSON 字符串。
    ///
    /// 格式与 Go 版 Cards 属性兼容：
    /// `[{"Id":52,"Name":"...","Ports":[...]}]`
    pub fn cards_json(&self) -> String {
        let list: Vec<CardExport> = self
            .cards
            .values()
            .map(|c| card_to_export(c, false))
            .collect();
        serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())
    }

    /// 序列化声卡列表为 JSON（不含不可用端口）。
    pub fn cards_without_unavailable_json(&self) -> String {
        let list: Vec<CardExport> = self
            .cards
            .values()
            .map(|c| card_to_export(c, true))
            .collect();
        serde_json::to_string(&list).unwrap_or_else(|_| "[]".into())
    }


    // ===== Module =====

    /// 获取模块状态。
    pub fn module_state(&self, name: &str) -> crate::backend::pulse::module::ModuleState {
        self.modules
            .get(name)
            .cloned()
            .unwrap_or_default()
    }

    /// 更新模块状态。
    pub fn set_module_state(
        &mut self,
        name: &str,
        state: crate::backend::pulse::module::ModuleState,
    ) {
        self.modules.insert(name.to_owned(), state);
    }

    /// 移除模块状态。
    #[allow(dead_code)]
    pub fn remove_module(&mut self, name: &str) {
        self.modules.remove(name);
    }

    // ===== Pending Profile =====

    /// 记录声卡 profile 切换等待项。
    pub fn set_pending_profile(&mut self, card_id: u32, wait: Arc<PendingProfile>) {
        self.pending_profiles.insert(card_id, wait);
    }

    /// 获取声卡 profile 切换等待项。
    pub fn get_pending_profile(&self, card_id: u32) -> Option<Arc<PendingProfile>> {
        self.pending_profiles.get(&card_id).cloned()
    }

    /// 移除声卡 profile 切换等待项。
    pub fn take_pending_profile(&mut self, card_id: u32) -> Option<Arc<PendingProfile>> {
        self.pending_profiles.remove(&card_id)
    }
}

/// 将 CardState 转换为 CardExport。
/// `filter_unavailable` 为 true 时过滤 enabled=false 的端口。
fn card_to_export(card: &CardState, filter_unavailable: bool) -> CardExport<'_> {
    let ports: Vec<CardPortExport> = card
        .ports
        .iter()
        .filter(|p| !filter_unavailable || p.enabled)
        .map(|p| CardPortExport {
            name: p.name.clone(),
            enabled: p.enabled,
            bluetooth: p.bluetooth,
            description: p.description.clone(),
            direction: p.direction,
            port_type: get_icon_port_type(&card.name, &p.name),
        })
        .collect();
    CardExport {
        id: card.index,
        name: &card.name,
        ports,
    }
}
