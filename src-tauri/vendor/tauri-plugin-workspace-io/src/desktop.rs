// Copyright 2026 WhiteMoon319
// SPDX-License-Identifier: Apache-2.0 OR MIT

// Desktop stub: workspace-io 仅在 Android 端有意义。

use tauri::{AppHandle, Runtime};

use crate::{ImportStreamRequest, ImportStreamResult};

/// Access to the workspace-io plugin (desktop stub).
#[derive(Debug)]
pub struct WorkspaceIo<R: Runtime>(AppHandle<R>);

impl<R: Runtime> WorkspaceIo<R> {
    pub fn import_stream(&self, _request: ImportStreamRequest) -> crate::Result<ImportStreamResult> {
        Err(crate::Error::UnsupportedPlatform)
    }

    pub fn resolve_display_name(&self, _uri: String) -> crate::Result<String> {
        Err(crate::Error::UnsupportedPlatform)
    }
}
