// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

use tauri::{plugin::PluginHandle, Runtime};

use crate::{ExecuteCommandRequest, ExecuteCommandResult, PrivilegeStatus};

/// Access to the device-control plugin (Android).
#[derive(Debug)]
pub struct DeviceControl<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> DeviceControl<R> {
    pub fn from_handle(handle: PluginHandle<R>) -> Self {
        Self(handle)
    }

    /// 查询提权状态（Shizuku 可用/已授权、root 可用）。
    pub fn status(&self) -> crate::Result<PrivilegeStatus> {
        let result = self
            .0
            .run_mobile_plugin::<PrivilegeStatus>("status", serde_json::json!({}))
            .map_err(crate::Error::PluginInvoke)?;
        Ok(result)
    }

    /// 触发 Shizuku 授权弹窗。
    pub fn request_privilege(&self) -> crate::Result<()> {
        self.0
            .run_mobile_plugin::<serde_json::Value>("requestPrivilege", serde_json::json!({}))
            .map_err(crate::Error::PluginInvoke)?;
        Ok(())
    }

    /// 以提权身份执行受控命令（Rust 侧已白名单校验）。
    pub fn execute_command(&self, request: ExecuteCommandRequest) -> crate::Result<ExecuteCommandResult> {
        let result = self
            .0
            .run_mobile_plugin::<ExecuteCommandResult>("executeCommand", request)
            .map_err(crate::Error::PluginInvoke)?;
        Ok(result)
    }
}
