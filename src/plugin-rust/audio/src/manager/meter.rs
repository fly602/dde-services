// SPDX-FileCopyrightText: 2026 UnionTech Software Technology Co., Ltd.
//
// SPDX-License-Identifier: LGPL-3.0-or-later

//! `org.deepin.dde.Audio2.Meter` 接口。

use std::sync::Arc;

use zbus::interface;

use crate::backend::pulse::PulseManager;

/// Meter D-Bus 对象。
pub struct Meter {
    id: u32,
    #[allow(dead_code)]
    pulse: Arc<PulseManager>,
}

impl Meter {
    pub fn new(id: u32, pulse: Arc<PulseManager>) -> Self {
        Self { id, pulse }
    }
}

#[interface(name = "org.deepin.dde.Audio2.Meter")]
impl Meter {
    /// 音量计量 tick 方法。
    fn tick(&self) -> zbus::fdo::Result<()> {
        // TODO: 通过 pulse 查询 meter 值
        let _ = self.id;
        Err(zbus::fdo::Error::Failed("unimplemented".into()))
    }
}
