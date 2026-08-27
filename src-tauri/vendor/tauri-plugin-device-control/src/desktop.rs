// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Desktop stub: device-control 仅在 Android 端有意义。

use tauri::{AppHandle, Runtime};

use crate::{ExecuteCommandRequest, ExecuteCommandResult, PrivilegeStatus};

/// Access to the device-control plugin (desktop stub).
#[derive(Debug)]
pub struct DeviceControl<R: Runtime>(AppHandle<R>);

impl<R: Runtime> DeviceControl<R> {
    pub fn status(&self) -> crate::Result<PrivilegeStatus> {
        Err(crate::Error::UnsupportedPlatform)
    }

    /// 触发 Shizuku 授权弹窗（桌面端无意义）。
    pub fn request_privilege(&self) -> crate::Result<()> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn execute_command(&self, _request: ExecuteCommandRequest) -> crate::Result<ExecuteCommandResult> {
        Err(crate::Error::UnsupportedPlatform)
    }
}
