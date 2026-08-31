// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! 端口设置 / 优先级切换协调器。
//!
//! 协调「手动设置端口」与「优先级自动切换」两类操作的互斥与取消，
//! 遵守协调规则：
//! - R1: 声卡 Pending 时，拒绝其优先级切换和端口设置
//! - R3: 端口设置过程中不触发优先级自动切换
//! - R4: 优先级自动切换中触发端口设置 → 中止切换，标记重排
//! - R8: 操作完成/失败需复位声卡状态
//!
//! 采用协作式取消：操作在阻塞点检查取消令牌（`StatusChange::wait` 的超时/Removed）。

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// 切换操作类型。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwitchKind {
    /// 手动设置端口（用户触发）。
    ManualSetPort,
    /// 优先级自动切换（声卡增删改后）。
    PriorityAuto,
}

/// 协调器：全局互斥 + 取消传播。
///
/// 进程内同一时刻只允许一个「会改变端口的操作」进行。
#[derive(Default)]
pub struct SwitchCoordinator {
    /// 当前进行中的操作类型（None=空闲）。
    in_progress: Mutex<Option<SwitchKind>>,
    /// 取消令牌：中止当前进行中的操作。
    cancel: Arc<AtomicBool>,
    /// 标记：手动端口设置打断自动切换后，需要重新执行优先级优选。
    reprioritize: AtomicBool,
}

impl SwitchCoordinator {
    pub fn new() -> Self {
        Self::default()
    }

    /// 尝试获取一个操作的互斥权。
    ///
    /// 若已有其他操作进行中，返回 Err（拒绝）。`kind` 为要执行的操作类型。
    pub fn acquire(&self, kind: SwitchKind) -> Result<SwitchGuard<'_>, String> {
        let mut cur = self
            .in_progress
            .lock()
            .map_err(|_| "coordinator mutex poisoned".to_string())?;
        if cur.is_some() {
            return Err("a switch operation is already in progress".into());
        }
        self.cancel.store(false, Ordering::SeqCst);
        *cur = Some(kind);
        Ok(SwitchGuard {
            _kind: kind,
            coord: self,
        })
    }

    /// 手动端口设置到来时，请求中止正在进行的优先级自动切换（R4）。
    ///
    /// 返回 true 表示确有自动切换在进行并已请求中止。
    pub fn interrupt_auto(&self) -> bool {
        let cur = self.in_progress.lock().ok();
        let is_auto = match cur.as_deref() {
            Some(Some(SwitchKind::PriorityAuto)) => true,
            _ => false,
        };
        if is_auto {
            self.cancel.store(true, Ordering::SeqCst);
            self.reprioritize.store(true, Ordering::SeqCst);
        }
        is_auto
    }

    /// 查询是否已请求取消进行中的操作。
    pub fn cancel_requested(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// 请求中止当前操作（如声卡移除触发）。
    pub fn request_cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    /// 是否需要重新执行优先级优选（R4 标记）。
    pub fn reprioritize_needed(&self) -> bool {
        self.reprioritize.load(Ordering::SeqCst)
    }

    /// 清除重排标记。
    pub fn clear_reprioritize(&self) {
        self.reprioritize.store(false, Ordering::SeqCst);
    }
}

/// 互斥守卫：Drop 时释放全局锁。
pub struct SwitchGuard<'a> {
    _kind: SwitchKind,
    coord: &'a SwitchCoordinator,
}

impl Drop for SwitchGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut cur) = self.coord.in_progress.lock() {
            *cur = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acquire_rejects_concurrent() {
        let c = SwitchCoordinator::new();
        let _g = c.acquire(SwitchKind::ManualSetPort).unwrap();
        // 已在 ManualSetPort 中，再 acquire 应拒绝
        assert!(c.acquire(SwitchKind::PriorityAuto).is_err());
    }

    #[test]
    fn guard_drop_releases() {
        let c = SwitchCoordinator::new();
        {
            let _g = c.acquire(SwitchKind::ManualSetPort).unwrap();
            assert!(c.acquire(SwitchKind::PriorityAuto).is_err());
        } // guard drop
        assert!(c.acquire(SwitchKind::PriorityAuto).is_ok());
    }

    #[test]
    fn interrupt_auto_only_auto() {
        let c = SwitchCoordinator::new();
        // 手动操作进行中，interrupt_auto 不应中断（不是自动切换）
        let _g = c.acquire(SwitchKind::ManualSetPort).unwrap();
        assert!(!c.interrupt_auto());
        assert!(!c.reprioritize_needed());
    }

    #[test]
    fn interrupt_auto_interrupts() {
        let c = SwitchCoordinator::new();
        let _g = c.acquire(SwitchKind::PriorityAuto).unwrap();
        assert!(c.interrupt_auto());
        assert!(c.cancel_requested());
        assert!(c.reprioritize_needed());
    }
}