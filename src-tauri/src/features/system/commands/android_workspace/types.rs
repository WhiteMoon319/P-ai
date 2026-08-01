use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AndroidWorkspaceStateKind {
    NotDownloaded,
    Downloading,
    Ready,
}

pub(crate) const ANDROID_WORKSPACE_STATUS_VERSION: u32 = 2;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceStatus {
    pub(crate) state: AndroidWorkspaceStateKind,
    pub(crate) root_path: String,
    #[serde(default)]
    pub(crate) llm_workspace_root: String,
    #[serde(default)]
    pub(crate) runtime_root: String,
    pub(crate) initialized_at: Option<String>,
    pub(crate) updated_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) runtime_version: Option<String>,
    #[serde(default)]
    pub(crate) download_bytes: Option<u64>,
    #[serde(default)]
    pub(crate) download_total_bytes: Option<u64>,
    #[serde(default)]
    pub(crate) download_stage: Option<String>,
}

impl AndroidWorkspaceStatus {
    pub(crate) fn new(state: AndroidWorkspaceStateKind, root: &std::path::Path) -> Self {
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
pub(crate) struct AndroidWorkspaceImportResult {
    #[serde(flatten)]
    pub(crate) status: AndroidWorkspaceStatus,
    pub(crate) imported_path: String,
    pub(crate) file_name: String,
    pub(crate) bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceExportResult {
    pub(crate) path: String,
    pub(crate) file_name: String,
    pub(crate) mime: String,
    pub(crate) data_base64: String,
    pub(crate) bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceFileEntry {
    pub(crate) name: String,
    pub(crate) path: String,
    pub(crate) kind: String,
    pub(crate) bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceFileListResult {
    pub(crate) current_path: String,
    pub(crate) parent_path: Option<String>,
    pub(crate) entries: Vec<AndroidWorkspaceFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceDeleteResult {
    pub(crate) deleted_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceTextResult {
    pub(crate) path: String,
    pub(crate) text: String,
    pub(crate) bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceWriteResult {
    pub(crate) entry: AndroidWorkspaceFileEntry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceMoveResult {
    pub(crate) source_path: String,
    pub(crate) entry: AndroidWorkspaceFileEntry,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceGlobResult {
    pub(crate) entries: Vec<AndroidWorkspaceFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceSearchMatch {
    pub(crate) path: String,
    pub(crate) line: usize,
    pub(crate) text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AndroidWorkspaceGrepResult {
    pub(crate) matches: Vec<AndroidWorkspaceSearchMatch>,
}

pub(crate) const ANDROID_WORKSPACE_TEXT_READ_MAX_BYTES: u64 = 512 * 1024;
pub(crate) const ANDROID_WORKSPACE_TEXT_WRITE_MAX_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const ANDROID_WORKSPACE_MAX_LIST_ENTRIES: usize = 500;
pub(crate) const ANDROID_WORKSPACE_MAX_SEARCH_RESULTS: usize = 100;
