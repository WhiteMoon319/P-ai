// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Shizuku/root 提权设备控制（Android-only）。
//!
//! Rust 侧只做命令白名单校验与调用转发；真正的提权 shell 执行与
//! Shizuku 授权在 Kotlin 侧（`DeviceControlPlugin`）完成。
//! 桌面端为 stub（返回 UnsupportedPlatform）。

#![doc(
    html_logo_url = "https://github.com/tauri-apps/tauri/raw/dev/app-icon.png",
    html_favicon_url = "https://github.com/tauri-apps/tauri/raw/dev/app-icon.png"
)]

use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

#[cfg(desktop)]
pub use desktop::DeviceControl;
#[cfg(mobile)]
pub use mobile::DeviceControl;

/// 提权状态（与前端/契约对齐）。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivilegeStatus {
    /// Shizuku 是否已安装且服务可 ping。
    pub shizuku_available: bool,
    /// 当前应用是否已获得 Shizuku 授权。
    pub shizuku_granted: bool,
    /// 设备是否可用 root（su 存在）。
    pub root_available: bool,
    /// 综合提权状态：disabled | shizuku_pending | shizuku_ready | root_ready
    pub privilege_state: String,
}

/// 提权命令执行请求（Rust 侧白名单校验通过后转发到 Kotlin）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCommandRequest {
    /// 已渲染的受控命令字符串（由 Rust 侧 DeviceCommand 白名单枚举生成，禁止自由拼接）。
    pub command: String,
    /// 命令超时（毫秒）。
    pub timeout_ms: u64,
}

/// 提权命令执行结果。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteCommandResult {
    /// 进程退出码；-1 表示未能启动或超时。
    pub exit_code: i32,
    /// stdout 文本。
    pub stdout: String,
    /// stderr 文本。
    pub stderr: String,
}

/// Extension trait to access the device-control plugin.
pub trait DeviceControlExt<R: Runtime> {
    fn device_control(&self) -> &DeviceControl<R>;
}

impl<R: Runtime, T: Manager<R>> DeviceControlExt<R> for T {
    fn device_control(&self) -> &DeviceControl<R> {
        self.state::<DeviceControl<R>>().inner()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("device-control 插件仅在 Android 端可用")]
    UnsupportedPlatform,
    #[error(transparent)]
    Tauri(#[from] tauri::Error),
    #[cfg(mobile)]
    #[error(transparent)]
    PluginInvoke(#[from] tauri::plugin::mobile::PluginInvokeError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    Builder::new("device-control")
        .setup(|app, api| {
            #[cfg(mobile)]
            {
                let handle = api.register_android_plugin("app.tauri.device_control", "DeviceControlPlugin")?;
                app.manage(DeviceControl::from_handle(handle));
            }
            #[cfg(desktop)]
            {
                let _ = api;
            }
            Ok(())
        })
        .build()
}
