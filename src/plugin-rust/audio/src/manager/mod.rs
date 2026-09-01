// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 音频管理器模块。
//!
//! 整合 D-Bus 接口定义、内存状态管理、事件处理。
//! - [`AudioManager`] — 持有 PulseManager、DeviceManager、EventLoop
//! - [`device_manager`] — 设备状态内存存储（四张 HashMap 表）
//! - [`event_loop`] — 消费 PulseEvent 更新 DeviceManager
//! - [`audio`] / [`sink`] / [`source`] / [`sink_input`] / [`card`] / [`meter`] — D-Bus 接口与业务逻辑

pub mod audio;
pub mod card;
pub mod config;
pub mod coordinator;
pub mod device_manager;
pub mod device_type;
pub mod dconfig;
pub mod event_loop;
pub mod meter;
pub mod port_priority;
pub mod sink;
pub mod sink_input;
pub mod source;

use std::sync::Arc;

use parking_lot::RwLock;

use crate::backend::pulse::module::{MODULE_NULL_SINK, module_argument};
use crate::backend::pulse::PulseManager;

use config::AudioConfig;
use device_manager::DeviceManager;
use port_priority::{Direction, PortKey};

/// D-Bus 服务名。
#[allow(dead_code)]
pub const DBUS_SERVICE_NAME: &str = "org.deepin.dde.Audio1";

/// D-Bus 主对象路径。
pub const DBUS_PATH: &str = "/org/deepin/dde/Audio1";

/// 音频管理器。
///
/// 持有 PulseManager（libpulse 连接）、DeviceManager（内存状态）、
/// EventLoop（事件消费线程）。
///
/// D-Bus Audio 主接口通过 `Arc<AudioManager>` 调用 Audio 级操作。
/// Sink/Source/SinkInput 子接口通过 `Arc<RwLock<DeviceManager>>` 读取状态，
/// 通过 `Arc<PulseManager>` 调用底层操作。
pub struct AudioManager {
    pulse: Arc<PulseManager>,
    device_manager: Arc<RwLock<DeviceManager>>,
    connection: zbus::blocking::Connection,
    #[allow(dead_code)]
    event_loop: event_loop::EventLoop,
    /// 端口设置/优先级切换协调器。
    coordinator: coordinator::SwitchCoordinator,
    /// 配置持久化。
    config: AudioConfig,
    /// 事件循环回调用的自身弱引用槽（lib.rs 在 Arc 创建后写入）。
    #[allow(dead_code)]
    manager_slot: std::sync::Arc<parking_lot::RwLock<Option<std::sync::Weak<AudioManager>>>>,
    /// dconfig 变更监听线程（保活句柄）。
    #[allow(dead_code)]
    dconfig_watch: Option<std::thread::JoinHandle<()>>,
    /// 单声道开关状态。
    mono: std::sync::atomic::AtomicBool,
}

impl AudioManager {
    pub fn new(
        connection: zbus::blocking::Connection,
        manager_slot: std::sync::Arc<parking_lot::RwLock<Option<std::sync::Weak<AudioManager>>>>,
    ) -> Result<Self, String> {
        let (pulse, events) = PulseManager::new()?;
        let pulse = Arc::new(pulse);
        let device_manager = Arc::new(RwLock::new(DeviceManager::default()));

        // 加载持久化配置（禁用端口/用户偏好）
        let config = AudioConfig::new();
        config.load();

        // 启动前先查询当前音频状态，填充 DeviceManager 并注册 D-Bus 子对象
        init_devices(&pulse, &device_manager, &connection)?;

        // 应用持久化配置（需 DeviceManager 已有 cards 做卡名→id 映射）
        config.apply_to(&mut *device_manager.write());

        // 确保 null-sink 模块存在（端口切换时作为临时 default）
        let _ = load_module(&pulse, &device_manager, MODULE_NULL_SINK, None, None);

        // 声卡事件回调：升级自身弱引用，触发自动端口切换
        let slot = manager_slot.clone();
        let on_card_event: Option<Arc<dyn Fn() + Send + Sync>> = Some(Arc::new(move || {
            if let Some(weak) = slot.read().as_ref() {
                if let Some(mgr) = weak.upgrade() {
                    let _ = mgr.auto_switch_ports();
                }
            }
        }));

        let event_loop = event_loop::EventLoop::start(
            pulse.clone(),
            device_manager.clone(),
            connection.clone(),
            events,
            on_card_event,
        );

        // dconfig 类型优先级：
        // 1) 启动一次性读取应用到内存策略
        // 2) 订阅 valueChanged，变更时经 weak slot 重应用
        let mut dconfig_watch = None;
        if let Ok(dconn) = zbus::blocking::Connection::system() {
            match dconfig::load_type_order(&dconn) {
                Ok(cfg) => {
                    let mut dm = device_manager.write();
                    dm.output_priority.set_type_order(cfg.output.clone());
                    dm.input_priority.set_type_order(cfg.input.clone());
                    dm.refresh_priority();
                }
                Err(e) => eprintln!("[dde-audio] dconfig load failed (using defaults): {e}"),
            }
            // 监听回调：经 manager 弱引用升级后应用（new 返回后 slot 才写入）
            let slot_watch = manager_slot.clone();
            let apply = Arc::new(move |cfg: dconfig::TypeOrderConfig| {
                let weak = { slot_watch.read().as_ref().cloned() };
                if let Some(weak) = weak {
                    if let Some(mgr) = weak.upgrade() {
                        mgr.apply_type_order(&cfg);
                    }
                }
            });
            match dconfig::subscribe(dconn, apply) {
                Ok(h) => dconfig_watch = Some(h),
                Err(e) => eprintln!("[dde-audio] dconfig subscribe failed: {e}"),
            }
        } else {
            eprintln!("[dde-audio] dconfig system bus unreachable");
        }

        // 端口操作协调器：executor 线程串行执行任务；handler 通过自身
        // 弱引用升级分发给实际执行函数。AudioManager 构造完成后 slot
        // 才写入，executor 到那时才会收到任务，因此 upgrade 必然成功。
        let exec_slot = manager_slot.clone();
        let handler: Arc<dyn Fn(coordinator::TaskOp, &std::sync::atomic::AtomicBool) -> Result<(), String> + Send + Sync> =
            Arc::new(move |op, cancel| {
                let weak = { exec_slot.read().as_ref().cloned() };
                match weak.and_then(|w| w.upgrade()) {
                    Some(mgr) => mgr.execute_task(op, cancel),
                    None => Err("audio manager gone".into()),
                }
            });

        Ok(Self {
            pulse,
            device_manager,
            connection,
            event_loop,
            coordinator: coordinator::SwitchCoordinator::new(handler),
            config,
            manager_slot,
            dconfig_watch,
            mono: std::sync::atomic::AtomicBool::new(false),
        })
    }

    /// 应用 dconfig 提供的类型优先级顺序到内存策略（不落本地文件）。
    ///
    /// 供启动时一次性读取与 `valueChanged` 监听回调共用。
    pub fn apply_type_order(&self, cfg: &dconfig::TypeOrderConfig) {
        let mut dm = self.device_manager.write();
        dm.output_priority
            .set_type_order(cfg.output.clone());
        dm.input_priority
            .set_type_order(cfg.input.clone());
        dm.refresh_priority();
    }

    /// coordinator executor 线程执行任务的入口（由 handler 分发）。
    ///
    /// 仅在 executor 线程调用，天然串行；`cancel` 为该任务取消令牌，
    /// 任务内部等待点轮询它，被更高优先级任务取代时提前结束。
    fn execute_task(
        &self,
        op: coordinator::TaskOp,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        use coordinator::TaskOp;
        match op {
            TaskOp::SetPort { card_id, port_name, direction, auto } => {
                self.set_port_inner(card_id, &port_name, direction, auto, cancel)
            }
            TaskOp::SetPortEnabled { card_id, port_name, enabled } => {
                self.device_manager
                    .write()
                    .set_port_enabled(card_id, &port_name, enabled);
                if let Some(card) = self.device_manager.read().cards.get(&card_id) {
                    self.config.set_port_enabled(&card.name, &port_name, enabled);
                }
                eprintln!(
                    "[dde-audio] set port enabled: card {card_id} port {port_name} enabled={enabled}"
                );
                Ok(())
            }
            TaskOp::AutoSwitch => {
                // 选输出/输入优选端口并设置（auto=true）
                self.auto_switch_targets(cancel)
            }
        }
    }

    /// 获取 PulseManager 引用，供 D-Bus 子对象调用底层操作。
#[allow(dead_code)]
    pub fn pulse(&self) -> &Arc<PulseManager> {
        &self.pulse
    }

    /// 获取 DeviceManager 引用，供 D-Bus 子对象读取状态。
    pub fn device_manager(&self) -> &Arc<RwLock<DeviceManager>> {
        &self.device_manager
    }

    /// 获取 zbus Connection 引用，供 D-Bus 子对象动态注册/注销。
#[allow(dead_code)]
    pub fn connection(&self) -> &zbus::blocking::Connection {
        &self.connection
    }

    // ===== Audio 级别属性（后续实现） =====

#[allow(dead_code)]
    pub fn cards(&self) -> String {
        String::new()
    }
#[allow(dead_code)]
    pub fn cards_without_unavailable(&self) -> String {
        String::new()
    }
    pub fn bluetooth_audio_mode(&self) -> String {
        String::new()
    }
    pub fn bluetooth_audio_mode_opts(&self) -> Vec<String> {
        Vec::new()
    }
    pub fn current_audio_server(&self) -> String {
        "pulseaudio".to_owned()
    }
    pub fn audio_server_state(&self) -> bool {
        true
    }
    pub fn max_ui_volume(&self) -> f64 {
        1.0
    }
    /// 单声道是否开启。
    pub fn mono(&self) -> bool {
        use std::sync::atomic::Ordering;
        self.mono.load(Ordering::SeqCst)
    }

    // ===== Audio 级别方法（后续实现） =====

    pub fn set_bluetooth_audio_mode(&self, _mode: &str) -> Result<(), String> {
        Err("unimplemented".into())
    }
    /// 开启/关闭单声道。
    ///
    /// 开启：加载 `module-remap-sink` 创建 mono-sink（master 绑定当前
    /// 物理默认 sink），并设为默认输出；关闭：卸载 mono 模块，恢复默认。
    pub fn set_mono(&self, enable: bool) -> Result<(), String> {
        use crate::backend::pulse::module::{MODULE_REMAP_SINK, MONO_SINK_NAME};
        use std::sync::atomic::Ordering;

        if self.mono.load(Ordering::SeqCst) == enable {
            return Ok(());
        }

        if enable {
            // 取当前默认 sink 作为 mono 的 master（物理设备）
            let (default_sink, _) = self.pulse.default_sink_source()?;
            if default_sink.is_empty() || default_sink == MONO_SINK_NAME {
                return Err("no physical default sink for mono".into());
            }
            // 加载 remap-sink 模块（参数在 module_argument 中编排）
            load_module(
                &self.pulse,
                &self.device_manager,
                MODULE_REMAP_SINK,
                Some(&default_sink),
                None,
            )?;
            // mono-sink 设为默认输出
            self.pulse.set_default_sink(MONO_SINK_NAME)?;
            eprintln!("[dde-audio] mono enabled (master {default_sink})");
        } else {
            // 卸载 mono 模块：先查索引再卸载
            if let Some(index) = find_module_index(&self.pulse, MODULE_REMAP_SINK)? {
                self.pulse.unload_module(index)?;
                // 恢复默认 sink 为物理设备
                let (default_sink, _) = self.pulse.default_sink_source()?;
                if default_sink != MONO_SINK_NAME && !default_sink.is_empty() {
                    self.pulse.set_default_sink(&default_sink)?;
                }
            }
            eprintln!("[dde-audio] mono disabled");
        }

        self.mono.store(enable, Ordering::SeqCst);
        Ok(())
    }
    /// 设置声卡端口。
    ///
    /// 手动设置声卡端口（用户触发）。经 coordinator 提交任务（最高
    /// 优先级，取代进行中的自动/其它手动操作），同步等待结果。
    pub fn set_port(&self, card_id: u32, port_name: &str, direction: u32) -> Result<(), String> {
        use coordinator::{SwitchKind, TaskOp};
        let task = self.coordinator.new_task(
            SwitchKind::ManualSetPort,
            TaskOp::SetPort {
                card_id,
                port_name: port_name.to_owned(),
                direction,
                auto: false,
            },
        );
        self.coordinator.submit(&task);
        match task.wait(std::time::Duration::from_secs(15))? {
            coordinator::TaskResult::Ok => Ok(()),
            coordinator::TaskResult::Err(e) => Err(e),
            coordinator::TaskResult::Cancelled => {
                Err("port operation cancelled by a newer request".into())
            }
        }
    }

    /// 设置声卡端口（手动/自动共享核心）。
    ///
    /// `auto=true` 为优先级自动切换调用（不记录 user_prefer）；
    /// `auto=false` 为手动调用（记录用户偏好 R6）。
    /// 由 coordinator executor 线程串行调用；`cancel` 为该任务取消令牌，
    /// 等待点轮询它，被更高优先级任务取代时提前结束。
    fn set_port_inner(
        &self,
        card_id: u32,
        port_name: &str,
        direction: u32,
        auto: bool,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        // 手动：记录用户偏好（R6）
        // 注意：先取卡名（读锁内克隆），释放后再写锁 —— read() 内嵌套 write()
        // 会触发 parking_lot 非重入死锁（SetPort 曾因卡死 D-Bus 线程）。
        if !auto {
            let card_name = self
                .device_manager
                .read()
                .cards
                .get(&card_id)
                .map(|c| c.name.clone());
            if let Some(card_name) = card_name {
                let direction = Direction::from(direction);
                if direction == Direction::Output {
                    self.device_manager
                        .write()
                        .output_priority
                        .set_user_prefer(&card_name, port_name);
                } else {
                    self.device_manager
                        .write()
                        .input_priority
                        .set_user_prefer(&card_name, port_name);
                }
                // 持久化用户首选端口（跨重启保留）
                self.config.set_user_prefer(
                    direction,
                    PortKey {
                        card_name,
                        port_name: port_name.to_owned(),
                    },
                );
            }
        }
        // 实际端口设置逻辑已迁入 card 模块（R1 等待 / profile 切换 / 设端口）
        card::set_port(&self.pulse, &self.device_manager, card_id, port_name, direction, cancel)
    }

    /// 自动切换端口：按优先级选输出/输入优选端口并设置。
    ///
    /// 触发点：声卡增删改后、声卡 Pending→Ready 完成时（由 event_loop 回调触发）。
    /// 通过协调器 PriorityAuto 与手动 set_port 互斥（R3/R4）。
    /// 触发自动端口切换。
    ///
    /// event_loop 回调调用：按优先级选输出/输入优选端口并设置
    /// （auto=true）。经 coordinator 提交 AutoSwitch 任务（最低优先级，
    /// 遇手动/启用等待；同类新抢旧），由 executor 串行执行。
    pub fn auto_switch_ports(&self) -> Result<(), String> {
        use coordinator::{SwitchKind, TaskOp};
        let task = self.coordinator.new_task(SwitchKind::PriorityAuto, TaskOp::AutoSwitch);
        self.coordinator.submit(&task);
        Ok(())
    }

    /// 在 executor 线程内执行自动切换：选输出/输入优选端口并设置。
    ///
    /// `cancel` 为所属 PriorityAuto 任务的取消令牌，被更高优先级任务
    /// 取代时提前结束。
    fn auto_switch_targets(
        &self,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), String> {
        // 输出：选优选端口对应的 sink 并设置
        let out_target = {
            let dm = self.device_manager.read();
            dm.output_priority.prefer_port(|key| {
                let card_id = dm.cards.iter()
                    .find(|(_, c)| c.name == key.card_name)
                    .map(|(i, _)| *i);
                match card_id {
                    Some(cid) => dm.sinks.values().any(|s| s.card == cid),
                    None => false,
                }
            }).map(|p| (p.card_id, p.port_name.clone(), 1u32))
        };
        if let Some((card_id, port_name, dir)) = out_target {
            let _ = self.set_port_inner(card_id, &port_name, dir, true, cancel);
        }

        // 输入：同理选 source
        let in_target = {
            let dm = self.device_manager.read();
            dm.input_priority.prefer_port(|key| {
                let card_id = dm.cards.iter()
                    .find(|(_, c)| c.name == key.card_name)
                    .map(|(i, _)| *i);
                match card_id {
                    Some(cid) => dm.sources.values().any(|s| s.card == cid),
                    None => false,
                }
            }).map(|p| (p.card_id, p.port_name.clone(), 2u32))
        };
        if let Some((card_id, port_name, dir)) = in_target {
            let _ = self.set_port_inner(card_id, &port_name, dir, true, cancel);
        }
        Ok(())
    }

    /// 设置端口启用/禁用。
    ///
    /// 经 coordinator 提交 SetPortEnabled 任务（与端口切换互斥：
    /// 遇到手动 SetPort 等待，遇到自动切换可抢占），同步等待结果。
    pub fn set_port_enabled(
        &self,
        card_id: u32,
        port_name: &str,
        enabled: bool,
    ) -> Result<(), String> {
        use coordinator::{SwitchKind, TaskOp};
        let task = self.coordinator.new_task(
            SwitchKind::SetPortEnabled,
            TaskOp::SetPortEnabled {
                card_id,
                port_name: port_name.to_owned(),
                enabled,
            },
        );
        self.coordinator.submit(&task);
        match task.wait(std::time::Duration::from_secs(10))? {
            coordinator::TaskResult::Ok => Ok(()),
            coordinator::TaskResult::Err(e) => Err(e),
            coordinator::TaskResult::Cancelled => {
                Err("set port enabled cancelled by a newer request".into())
            }
        }
    }
    /// 查询端口是否启用（未被用户禁用）。
    pub fn is_port_enabled(&self, card_id: u32, port_name: &str) -> Result<bool, String> {
        Ok(self.device_manager.read().is_port_enabled(card_id, port_name))
    }
    pub fn set_current_audio_server(&self, _server_name: &str) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn stop_audio_service(&self) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn reset(&self) -> Result<(), String> {
        Err("unimplemented".into())
    }
    pub fn no_restart_pulse_audio(&self) -> Result<(), String> {
        Err("unimplemented".into())
    }
}

/// 启动初始化：查询当前所有设备状态，填充 DeviceManager 并注册 D-Bus 子对象。
///
/// 顺序：cards → sinks → sources → sink_inputs，再查询默认 sink/source。
fn init_devices(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    connection: &zbus::blocking::Connection,
) -> Result<(), String> {
    use crate::backend::pulse::{card, sink, sink_input, source};
    use crate::manager::sink::SinkInterface;
    use crate::manager::sink_input::SinkInputInterface;
    use crate::manager::source::SourceInterface;

    // cards（无 D-Bus 对象，只填状态）
    for state in card::query_list(pulse)? {
        let state: crate::manager::card::Card = state.into();
        device_manager.write().add_card(state.index, state);
    }

    // sinks（注册 D-Bus 对象）
    for state in sink::query_list(pulse)? {
        let index = state.index;
        let state: crate::manager::sink::Sink = state.into();
        device_manager.write().add_sink(index, state);
        let obj = SinkInterface::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
        connection
            .object_server()
            .at(SinkInterface::path(index), obj)
            .map_err(|e| format!("register sink {index} failed: {e}"))?;
    }

    // sources
    for state in source::query_list(pulse)? {
        let index = state.index;
        let state: crate::manager::source::Source = state.into();
        device_manager.write().add_source(index, state);
        let obj = SourceInterface::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
        connection
            .object_server()
            .at(SourceInterface::path(index), obj)
            .map_err(|e| format!("register source {index} failed: {e}"))?;
    }

    // sink_inputs
    for state in sink_input::query_list(pulse)? {
        let index = state.index;
        let state: crate::manager::sink_input::SinkInput = state.into();
        device_manager.write().add_sink_input(index, state);
        let obj = SinkInputInterface::new_instance(index, pulse.clone(), device_manager.clone(), connection.clone());
        connection
            .object_server()
            .at(SinkInputInterface::path(index), obj)
            .map_err(|e| format!("register sink input {index} failed: {e}"))?;
    }

    // 默认 sink/source
    if let Ok((default_sink, default_source)) = pulse.default_sink_source() {
        device_manager.write().set_default_sink(default_sink);
        device_manager.write().set_default_source(default_source);
    }

    eprintln!("[dde-audio] device init done");
    Ok(())
}

/// 加载模块（统一接口）。
///
/// - 模块已存在 → 标记 Complete，不重复加载
/// - 模块不存在 → 记录 Loading，按模块参数编排加载，等待设备创建事件完成
///
/// `channel` 为主绑定设备名（单声道/降噪用），`extra_channel` 为附加绑定设备名（echo-cancel 的 sink_master）。
/// null-sink 均传 None。
fn load_module(
    pulse: &Arc<PulseManager>,
    device_manager: &Arc<RwLock<DeviceManager>>,
    name: &str,
    channel: Option<&str>,
    extra_channel: Option<&str>,
) -> Result<(), String> {
    use crate::backend::pulse::module::ModuleState;

    // 已加载完成则跳过
    if let ModuleState::Complete { .. } = device_manager.read().module_state(name) {
        return Ok(());
    }

    // 模块可能已被其他进程加载：查询索引并标记 Complete
    if let Some(index) = find_module_index(pulse, name)? {
        device_manager
            .write()
            .set_module_state(name, ModuleState::Complete { module_index: index });
        return Ok(());
    }

    // 不存在：记录 Loading 并加载
    let argument = module_argument(name, channel, extra_channel);
    device_manager.write().set_module_state(
        name,
        ModuleState::Loading { loaded_at: std::time::Instant::now() },
    );
    let index = pulse.load_module(name, &argument)?;
    device_manager
        .write()
        .set_module_state(name, ModuleState::Complete { module_index: index });
    eprintln!("[dde-audio] loaded module {name} (index {index})");
    Ok(())
}

/// 查询模块索引（存在则 Some，不存在则 None）。
fn find_module_index(
    pulse: &Arc<PulseManager>,
    name: &str,
) -> Result<Option<u32>, String> {
    use libpulse_binding::callbacks::ListResult;

    let name = name.to_owned();
    pulse.execute(|ctx, tx| {
        let intro = ctx.introspect();
        let mut found: Option<u32> = None;
        intro.get_module_info_list(move |res| {
            match res {
                ListResult::Item(info) => {
                    if info.name.as_deref() == Some(name.as_str()) {
                        found = Some(info.index);
                    }
                }
                ListResult::End | ListResult::Error => {
                    let _ = tx.send(found.take());
                }
            }
        });
        true
    })
}
