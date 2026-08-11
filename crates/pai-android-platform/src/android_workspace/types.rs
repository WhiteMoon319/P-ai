use serde::{Deserialize, Serialize};

use crate::android_workspace::rootfs_paths::android_workspace_status_paths;
use pai_backend::core::time_semantics::now_iso;

/// 内置 rootfs 包预期字节数（从 src-tauri android_workspace.rs 迁入）。
pub const ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH: u64 = 29_865_086;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AndroidWorkspaceStateKind {
    NotDownloaded,
    Downloading,
    Ready,
}

pub const ANDROID_WORKSPACE_STATUS_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceStatus {
    pub state: AndroidWorkspaceStateKind,
    pub root_path: String,
    #[serde(default)]
    pub llm_workspace_root: String,
    #[serde(default)]
    pub runtime_root: String,
    pub initialized_at: Option<String>,
    pub updated_at: Option<String>,
    pub last_error: Option<String>,
    pub version: u32,
    #[serde(default)]
    pub runtime_version: Option<String>,
    #[serde(default)]
    pub download_bytes: Option<u64>,
    #[serde(default)]
    pub download_total_bytes: Option<u64>,
    #[serde(default)]
    pub download_stage: Option<String>,
}

impl AndroidWorkspaceStatus {
    pub fn new(state: AndroidWorkspaceStateKind, root: &std::path::Path) -> Self {
        let (llm_workspace_root, runtime_root) = android_workspace_status_paths(root);
        Self {
            state,
            root_path: llm_workspace_root.clone(),
            llm_workspace_root,
            runtime_root,
            initialized_at: None,
            updated_at: Some(now_iso()),
            last_error: None,
            version: ANDROID_WORKSPACE_STATUS_VERSION,
            runtime_version: None,
            download_bytes: None,
            download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_stage: None,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceImportResult {
    #[serde(flatten)]
    pub status: AndroidWorkspaceStatus,
    pub imported_path: String,
    pub file_name: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceExportResult {
    pub path: String,
    pub file_name: String,
    pub mime: String,
    pub data_base64: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceFileEntry {
    pub name: String,
    pub path: String,
    pub kind: String,
    pub bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceFileListResult {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub entries: Vec<AndroidWorkspaceFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceDeleteResult {
    pub deleted_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceTextResult {
    pub path: String,
    pub text: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceWriteResult {
    pub entry: AndroidWorkspaceFileEntry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceMoveResult {
    pub source_path: String,
    pub entry: AndroidWorkspaceFileEntry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceGlobResult {
    pub entries: Vec<AndroidWorkspaceFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceSearchMatch {
    pub path: String,
    pub line: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AndroidWorkspaceGrepResult {
    pub matches: Vec<AndroidWorkspaceSearchMatch>,
}

pub const ANDROID_WORKSPACE_TEXT_READ_MAX_BYTES: u64 = 512 * 1024;
pub const ANDROID_WORKSPACE_TEXT_WRITE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub const ANDROID_WORKSPACE_MAX_LIST_ENTRIES: usize = 500;
pub const ANDROID_WORKSPACE_MAX_SEARCH_RESULTS: usize = 100;
