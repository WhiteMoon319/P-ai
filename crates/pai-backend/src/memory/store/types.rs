use serde::{Deserialize, Serialize};

// ========== constants ==========
pub const MEMORY_DB_FILE_NAME: &str = "memory_store.db";pub const KB_STATE_ACTIVE_INDEX_PROVIDER_ID: &str = "active_index_provider_id";
pub const KB_STATE_EMBEDDING_API_CONFIG_ID: &str = "embedding_api_config_id";
pub const KB_STATE_RERANK_API_CONFIG_ID: &str = "rerank_api_config_id";
pub const KB_STATE_REBUILD_STATUS: &str = "rebuild_status";
pub const KB_STATE_REBUILD_TRACE_ID: &str = "rebuild_trace_id";
pub const KB_STATE_REBUILD_DONE_BATCHES: &str = "rebuild_done_batches";
pub const KB_STATE_REBUILD_TOTAL_BATCHES: &str = "rebuild_total_batches";
pub const KB_STATE_REBUILD_ERROR: &str = "rebuild_error";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStoreImportStats {
    pub imported_count: usize,
    pub created_count: usize,
    pub merged_count: usize,
    pub total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStoreProviderSyncReport {
    pub status: String,
    pub old_provider_id: Option<String>,
    pub new_provider_id: String,
    pub deleted: usize,
    pub added: usize,
    pub batch_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStoreRebuildReport {
    pub memory_rows: usize,
    pub memory_fts_rows: usize,
    pub note_rows: usize,
    pub note_fts_rows: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStoreHealthReport {
    pub status: String,
    pub memory_rows: usize,
    pub memory_fts_rows: usize,
    pub note_rows: usize,
    pub note_fts_rows: usize,
    pub orphan_memory_tag_rows: usize,
    pub orphan_note_tag_rows: usize,
    pub repaired: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryStoreBackupResult {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct MemoryDraftInput {
    pub memory_type: String,
    pub judgment: String,
    pub reasoning: String,
    pub tags: Vec<String>,
    pub owner_agent_id: Option<String>,
}

/// 记忆保存/更新单条结果（从 src-tauri builtin_memory.rs 迁入）。
#[derive(Debug, Clone, Serialize)]
pub struct MemorySaveUpsertItemResult {
    pub saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// 归一化空白（从 src-tauri core_provider_utils.rs 迁入）。
pub fn clean_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// 记忆关键词去重归一化（从 src-tauri builtin_memory.rs 迁入）。
pub fn normalize_memory_keywords(raw: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for item in raw {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        let v = trimmed.to_string();
        if !out.iter().any(|x| x == &v) {
            out.push(v);
        }
    }
    out
}

/// 敏感内容检测（从 src-tauri builtin_memory.rs 迁入）。
pub fn memory_contains_sensitive(content: &str, keywords: &[String]) -> bool {
    let mut full = content.to_lowercase();
    if !keywords.is_empty() {
        full.push('\n');
        full.push_str(&keywords.join(" ").to_lowercase());
    }
    let danger_tokens = [
        "password",
        "passwd",
        "api key",
        "apikey",
        "token",
        "secret",
        "private key",
        "sk-",
        "ssh-rsa",
        "验证码",
        "密码",
        "密钥",
        "身份证",
        "银行卡",
        "cvv",
    ];
    danger_tokens.iter().any(|token| full.contains(token))
}

/// FTS 分词（从 src-tauri matcher.rs 迁入的简化实现：ASCII 词 + CJK 单字）。
pub fn memory_tokenize_terms_simple(text: &str, dedup: bool) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let mut ascii = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            ascii.push(ch.to_ascii_lowercase());
            continue;
        }
        if !ascii.is_empty() {
            memory_push_token(&mut out, &mut seen, std::mem::take(&mut ascii), dedup);
        }
        if !ch.is_whitespace() {
            memory_push_token(&mut out, &mut seen, ch.to_string(), dedup);
        }
    }
    if !ascii.is_empty() {
        memory_push_token(&mut out, &mut seen, ascii, dedup);
    }
    out
}

fn memory_push_token(out: &mut Vec<String>, seen: &mut std::collections::HashSet<String>, token: String, dedup: bool) {
    if token.trim().is_empty() {
        return;
    }
    if dedup && !seen.insert(token.clone()) {
        return;
    }
    out.push(token);
}


