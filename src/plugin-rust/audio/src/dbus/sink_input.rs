// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.SinkInput` 接口。
//!
//! 对应 Go 版 `SinkInput` 结构体导出属性和方法。
//! 直接持有 `Arc<PulseManager>`，操作委托给 `pulse::sink_input` 模块。

use std::sync::Arc;

use zbus::interface;

use crate::backend::pulse::PulseManager;
use crate::backend::pulse::sink_input as pulse_sink_input;
use crate::backend::SinkInputInfo;

/// SinkInput DBus 对象。
pub struct SinkInput {
    index: u32,
    pulse: Arc<PulseManager>,
}

impl SinkInput {
    pub fn new(index: u32, pulse: Arc<PulseManager>) -> Self {
        Self { index, pulse }
    }

    fn info(&self) -> Result<SinkInputInfo, zbus::fdo::Error> {
        // TODO: 通过 pulse.execute 查询 sink input info
        Err(zbus::fdo::Error::Failed("unimplemented".into()))
    }
}

#[interface(name = "org.deepin.dde.Audio2.SinkInput")]
impl SinkInput {
    // ========== 属性 ==========

    #[zbus(property)]
    pub fn name(&self) -> String {
        self.info().map(|i| i.name).unwrap_or_default()
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

    // ========== 方法 ==========

    fn set_balance(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_sink_input::set_balance(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_fade(&self, value: f64) -> zbus::fdo::Result<()> {
        pulse_sink_input::set_fade(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_mute(&self, value: bool) -> zbus::fdo::Result<()> {
        pulse_sink_input::set_mute(&self.pulse, self.index, value)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }

    fn set_volume(&self, value: f64, is_play: bool) -> zbus::fdo::Result<()> {
        pulse_sink_input::set_volume(&self.pulse, self.index, value, is_play)
            .map_err(|e| zbus::fdo::Error::Failed(e))
    }
}
