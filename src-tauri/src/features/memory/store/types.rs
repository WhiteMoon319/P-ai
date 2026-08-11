
// ========== constants ==========
pub(crate) const MEMORY_DB_FILE_NAME: &str = "memory_store.db";
pub(crate) const KB_STATE_ACTIVE_INDEX_PROVIDER_ID: &str = "active_index_provider_id";
pub(crate) const KB_STATE_EMBEDDING_API_CONFIG_ID: &str = "embedding_api_config_id";
pub(crate) const KB_STATE_RERANK_API_CONFIG_ID: &str = "rerank_api_config_id";
pub(crate) const KB_STATE_REBUILD_STATUS: &str = "rebuild_status";
pub(crate) const KB_STATE_REBUILD_TRACE_ID: &str = "rebuild_trace_id";
pub(crate) const KB_STATE_REBUILD_DONE_BATCHES: &str = "rebuild_done_batches";
pub(crate) const KB_STATE_REBUILD_TOTAL_BATCHES: &str = "rebuild_total_batches";
pub(crate) const KB_STATE_REBUILD_ERROR: &str = "rebuild_error";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStoreImportStats {
    pub(crate) imported_count: usize,
    pub(crate) created_count: usize,
    pub(crate) merged_count: usize,
    pub(crate) total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStoreProviderSyncReport {
    pub(crate) status: String,
    pub(crate) old_provider_id: Option<String>,
    pub(crate) new_provider_id: String,
    pub(crate) deleted: usize,
    pub(crate) added: usize,
    pub(crate) batch_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStoreRebuildReport {
    pub(crate) memory_rows: usize,
    pub(crate) memory_fts_rows: usize,
    pub(crate) note_rows: usize,
    pub(crate) note_fts_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStoreHealthReport {
    pub(crate) status: String,
    pub(crate) memory_rows: usize,
    pub(crate) memory_fts_rows: usize,
    pub(crate) note_rows: usize,
    pub(crate) note_fts_rows: usize,
    pub(crate) orphan_memory_tag_rows: usize,
    pub(crate) orphan_note_tag_rows: usize,
    pub(crate) repaired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryStoreBackupResult {
    pub(crate) path: String,
    pub(crate) bytes: u64,
}

#[derive(Debug, Clone)]
pub(crate) struct MemoryDraftInput {
    pub(crate) memory_type: String,
    pub(crate) judgment: String,
    pub(crate) reasoning: String,
    pub(crate) tags: Vec<String>,
    pub(crate) owner_agent_id: Option<String>,
}


