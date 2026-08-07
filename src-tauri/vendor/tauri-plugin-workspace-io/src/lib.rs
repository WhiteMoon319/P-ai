//!
//! Android-only plugin: the WebView only passes a `content://` URI string,
//! and the Kotlin side streams the bytes into a sandbox target path without
//! round-tripping through base64 in the frontend.

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
pub use desktop::WorkspaceIo;
#[cfg(mobile)]
pub use mobile::WorkspaceIo;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStreamRequest {
    /// Android `content://` URI string.
    pub uri: String,
    /// Absolute destination path inside the app sandbox.
    pub target_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportStreamResult {
    /// Bytes written to the destination.
    pub bytes: u64,
    /// Final destination path.
    pub path: String,
}

/// Extension trait to access the workspace-io plugin.
pub trait WorkspaceIoExt<R: Runtime> {
    fn workspace_io(&self) -> &WorkspaceIo<R>;
}

impl<R: Runtime, T: Manager<R>> WorkspaceIoExt<R> for T {
    fn workspace_io(&self) -> &WorkspaceIo<R> {
        self.state::<WorkspaceIo<R>>().inner()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("workspace-io 插件仅在 Android 端可用")]
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
    Builder::new("workspace-io")
        .setup(|app, api| {
            #[cfg(mobile)]
            {
                let handle = api.register_android_plugin("app.tauri.workspace_io", "WorkspaceIoPlugin")?;
                app.manage(WorkspaceIo::from_handle(handle));
            }
            #[cfg(desktop)]
            {
                let _ = api;
            }
            Ok(())
        })
        .build()
}
