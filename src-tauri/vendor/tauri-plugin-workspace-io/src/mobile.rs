// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

use tauri::{plugin::PluginHandle, Runtime};

use crate::{ImportStreamRequest, ImportStreamResult};

/// Access to the workspace-io plugin (Android).
#[derive(Debug)]
pub struct WorkspaceIo<R: Runtime>(PluginHandle<R>);

impl<R: Runtime> WorkspaceIo<R> {
    pub fn from_handle(handle: PluginHandle<R>) -> Self {
        Self(handle)
    }

    /// Stream a `content://` URI into `target_path` on Android.
    pub fn import_stream(&self, request: ImportStreamRequest) -> crate::Result<ImportStreamResult> {
        let result = self
            .0
            .run_mobile_plugin::<ImportStreamResult>("importStream", request)
            .map_err(crate::Error::PluginInvoke)?;
        Ok(result)
    }

    /// Resolve the display name of a `content://` URI on Android.
    pub fn resolve_display_name(&self, uri: String) -> crate::Result<String> {
        let result = self
            .0
            .run_mobile_plugin::<String>(
                "resolveDisplayName",
                serde_json::json!({ "uri": uri }),
            )
            .map_err(crate::Error::PluginInvoke)?;
        Ok(result)
    }
}
