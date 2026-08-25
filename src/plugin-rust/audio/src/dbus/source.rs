// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Source` 接口。
//!
//! 对应 Go 版 `Source` 结构体导出属性和方法。
//! 直接持有 `Arc<PulseManager>`，操作委托给 `pulse::source` 模块。

use std::sync::Arc;

use zbus::interface;

use crate::backend::pulse::PulseManager;
use crate::backend::pulse::source as pulse_source;
use crate::backend::SourceInfo;

/// Source DBus 对象。
pub struct Source {
    index: u32,
    pulse: Arc<PulseManager>,
}

impl Source {
    pub fn new(index: u32, pulse: Arc<PulseManager>) -> Self {
        Self { index, pulse }
    }

    fn info(&self) -> Result<SourceInfo, zbus::fdo::Error> {
        // TODO: 通过 pulse.execute 查询 source info
        Err(zbus::fdo::Error::Failed("unimplemented".into()))
    }
}

#[interface(name = "org.deepin.dde.Audio2.Source")]
impl Source {
    // ========== 属性 ==========

    #[zbus(property)]
    pub fn name(&self) -> String {
        self.info().map(|i| i.name).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn description(&self) -> String {
        self.info().map(|i| i.description).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn base_volume(&self) -> f64 {
        self.info().map(|i| i.base_volume).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn mute(&self) -> bool {
        self.info().map(|i| i.mute).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn volume(&self) -> f64 {
        self.info().map(|i| i.volume).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn balance(&self) -> f64 {
        self.info().map(|i| i.balance).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn support_balance(&self) -> bool {
        self.info().map(|i| i.support_balance).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn fade(&self) -> f64 {
        self.info().map(|i| i.fade).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn support_fade(&self) -> bool {
        self.info().map(|i| i.support_fade).unwrap_or_default()
    }

    #[zbus(property)]
    pub fn card(&self) -> u32 {
        self.info().map(|i| i.card).unwrap_or_default()
    }

    // ========== 方法 ==========

    fn get_meter(&self) -> zbus::fdo::Result<zbus::zvariant::OwnedObjectPath> {
        // TODO: 通过 pulse::source::get_meter 创建 Meter 并返回路径
        Err(zbus::fdo::Error::Failed("unimplemented".into()))
    }

    fn set_balance(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_source::set_balance(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_fade(&self, value: f64) -> zbus::fdo::Result<()> {
        pulse_source::set_fade(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_mute(&self, value: bool) -> zbus::fdo::Result<()> {
        pulse_source::set_mute(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_port(&self, name: &str) -> zbus::fdo::Result<()> {
        pulse_source::set_port(&self.pulse, self.index, name)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_volume(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_source::set_volume(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }
}
