use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap as StdHashMap, HashSet as StdHashSet};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use time::OffsetDateTime;
use uuid::Uuid;

use crate::core::domain::types_storage::MemoryEntry;
use crate::core::time_semantics::now_iso;
use crate::logging::{runtime_log_error, runtime_log_info, runtime_log_warn};
use super::*;

pub fn memory_store_collect_doc_texts(conn: &Connection) -> Result<StdHashMap<String, String>, String> {
    let mut stmt = conn
        .prepare("SELECT id, judgment FROM memory_record")
        .map_err(|err| format!("Prepare collect doc texts failed: {err}"))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(|err| format!("Query collect doc texts failed: {err}"))?;
    let mut out = StdHashMap::<String, String>::new();
    for row in rows {
        let (id, text) = row.map_err(|err| format!("Read collect doc row failed: {err}"))?;
        out.insert(id, text);
    }
    Ok(out)
}

pub fn memory_store_normalize_model_id(raw: &str) -> Result<String, String> {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' || ch == '.' || ch == '/' || ch == ':' {
            out.push('_');
        }
    }
    while out.contains("__") {
        out = out.replace("__", "_");
    }
    let out = out.trim_matches('_').to_string();
    if out.is_empty() {
        return Err("model_name is empty after normalization".to_string());
    }
    Ok(out)
}

pub fn memory_store_model_store_db_path(data_path: &PathBuf, model_name: &str) -> Result<PathBuf, String> {
    let norm = memory_store_normalize_model_id(model_name)?;
    Ok(app_root_from_data_path(data_path)
        .join("memory")
        .join(format!("{norm}_embedding_store.db")))
}

pub fn memory_store_validate_table_name(table: &str) -> Result<&str, String> {
    let trimmed = table.trim();
    if trimmed.is_empty()
        || !trimmed
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return Err(format!("Invalid provider table name: {table}"));
    }
    Ok(trimmed)
}

pub fn memory_store_provider_table(_provider_id: &str) -> Result<String, String> {
    let table = "memory_vector";
    debug_assert!(
        table
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    );
    Ok(table.to_string())
}

pub fn memory_store_open_provider_vector_db(
    data_path: &PathBuf,
    provider_id: &str,
) -> Result<Connection, String> {
    let main_conn = memory_store_open(data_path)?;
    let model = memory_store_provider_model_name(&main_conn, provider_id)?
        .ok_or_else(|| format!("provider '{provider_id}' has no model_name in embedding_provider"))?;
    memory_store_open_provider_vector_db_with_model(data_path, &model)
}

pub fn memory_store_open_provider_vector_db_with_model(
    data_path: &PathBuf,
    model_name: &str,
) -> Result<Connection, String> {
    let path = memory_store_model_store_db_path(data_path, model_name)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("Create provider vector db dir failed ({}): {err}", parent.display()))?;
    }
    let conn = Connection::open(&path)
        .map_err(|err| format!("Open provider vector db failed ({}): {err}", path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         CREATE TABLE IF NOT EXISTS memory_vector (
           chunk_id TEXT PRIMARY KEY,
           embedding_json TEXT NOT NULL,
           updated_at TEXT NOT NULL
         );",
    )
    .map_err(|err| format!("Init provider vector db failed ({}): {err}", path.display()))?;
    Ok(conn)
}

pub fn memory_store_provider_index_ids(conn: &Connection, table: &str) -> Result<StdHashSet<String>, String> {
    let table = memory_store_validate_table_name(table)?;
    let mut stmt = conn
        .prepare(&format!("SELECT chunk_id FROM {table}"))
        .map_err(|err| format!("Prepare list provider index ids failed: {err}"))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| format!("Query list provider index ids failed: {err}"))?;
    let mut out = StdHashSet::<String>::new();
    for row in rows {
        out.insert(row.map_err(|err| format!("Read provider index id failed: {err}"))?);
    }
    Ok(out)
}

pub fn memory_store_delete_provider_entries(
    vector_tx: &rusqlite::Transaction,
    table: &str,
    to_delete: &[String],
) -> Result<usize, String> {
    let table = memory_store_validate_table_name(table)?;
    let mut deleted = 0usize;
    for id in to_delete {
        vector_tx
            .execute(&format!("DELETE FROM {table} WHERE chunk_id=?1"), params![id])
            .map_err(|err| format!("Delete provider vector row failed: {err}"))?;
        deleted += 1;
    }
    Ok(deleted)
}

pub fn memory_store_batch_embed_and_insert<F>(
    vector_tx: &rusqlite::Transaction,
    table: &str,
    doc_map: &StdHashMap<String, String>,
    to_add: &[String],
    batch_size: usize,
    tx: &rusqlite::Transaction,
    embedder: &mut F,
) -> Result<(usize, usize, usize), String>
where
    F: FnMut(&[String]) -> Result<Vec<Vec<f32>>, String>,
{
    let table = memory_store_validate_table_name(table)?;
    let mut added = 0usize;
    let mut batch_count = 0usize;
    let mut dimension: usize = 0;
    let effective_batch = batch_size.max(1);

    for chunk in to_add.chunks(effective_batch) {
        let pairs = chunk
            .iter()
            .filter_map(|id| doc_map.get(id).map(|text| (id.clone(), text.clone())))
            .collect::<Vec<(String, String)>>();
        if pairs.is_empty() {
            continue;
        }
        let texts = pairs.iter().map(|(_, text)| text.clone()).collect::<Vec<_>>();
        let vectors = embedder(&texts)?;
        if vectors.len() != texts.len() {
            return Err("Embedding output length mismatch".to_string());
        }
        if let Some(first) = vectors.first() {
            dimension = first.len();
        }

        for (idx, (chunk_id, _)) in pairs.iter().enumerate() {
            let embedding = vectors
                .get(idx)
                .cloned()
                .ok_or_else(|| "Missing embedding row".to_string())?;
            let embedding_json = serde_json::to_string(&embedding)
                .map_err(|err| format!("Serialize embedding failed: {err}"))?;
            vector_tx
                .execute(
                    &format!(
                        "INSERT INTO {table}(chunk_id, embedding_json, updated_at)
                         VALUES (?1, ?2, ?3)
                         ON CONFLICT(chunk_id) DO UPDATE SET embedding_json=excluded.embedding_json, updated_at=excluded.updated_at"
                    ),
                    params![chunk_id, embedding_json, now_iso()],
                )
                .map_err(|err| format!("Upsert provider embedding failed: {err}"))?;
            added += 1;
        }
        batch_count += 1;
        memory_store_set_runtime_state(tx, KB_STATE_REBUILD_DONE_BATCHES, &batch_count.to_string())?;
    }

    Ok((added, batch_count, dimension))
}

pub fn memory_store_update_provider_metadata(
    tx: &rusqlite::Transaction,
    new_provider_id: &str,
    model_name: &str,
    dimension: usize,
    batch_count: usize,
) -> Result<(), String> {
    tx.execute(
        "UPDATE embedding_provider SET is_active=0, updated_at=?1",
        params![now_iso()],
    )
    .map_err(|err| format!("Reset embedding provider active flag failed: {err}"))?;
    tx.execute(
        "INSERT INTO embedding_provider(provider_id, dimension, model_name, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, 1, ?4, ?5)
         ON CONFLICT(provider_id) DO UPDATE SET dimension=excluded.dimension, model_name=excluded.model_name, is_active=1, updated_at=excluded.updated_at",
        params![new_provider_id.trim(), (dimension as i64), model_name.trim(), now_iso(), now_iso()],
    )
    .map_err(|err| format!("Upsert embedding provider failed: {err}"))?;
    memory_store_set_runtime_state(tx, KB_STATE_ACTIVE_INDEX_PROVIDER_ID, new_provider_id.trim())?;
    memory_store_set_runtime_state(tx, KB_STATE_REBUILD_STATUS, "idle")?;
    memory_store_set_runtime_state(tx, KB_STATE_REBUILD_DONE_BATCHES, &batch_count.to_string())?;
    Ok(())
}

pub fn memory_store_sync_provider_index<F>(
    data_path: &PathBuf,
    new_provider_id: &str,
    model_name: &str,
    batch_size: usize,
    force: bool,
    mut embedder: F,
) -> Result<MemoryStoreProviderSyncReport, String>
where
    F: FnMut(&[String]) -> Result<Vec<Vec<f32>>, String>,
{
    let started_at = std::time::Instant::now();
    let mut conn = memory_store_open(data_path)?;

    let old_provider_id = memory_store_get_runtime_state(&conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID)?;
    runtime_log_info(format!(
        "[记忆存储] 开始，任务=向量索引同步，old_provider_id={}，new_provider_id={}，model_name={}",
        old_provider_id.clone().unwrap_or_default(),
        new_provider_id.trim(),
        model_name.trim()
    ));
    let provider_store_exists = memory_store_model_store_db_path(data_path, model_name)
        .map(|p| p.exists())
        .unwrap_or(false);
    if !force && old_provider_id.as_deref() == Some(new_provider_id.trim()) && provider_store_exists {
        runtime_log_warn(format!(
            "[记忆存储] 跳过，任务=向量索引同步，reason=no_op，provider_id={}",
            new_provider_id.trim()
        ));
        return Ok(MemoryStoreProviderSyncReport {
            status: "no_op".to_string(),
            old_provider_id,
            new_provider_id: new_provider_id.trim().to_string(),
            deleted: 0,
            added: 0,
            batch_count: 0,
        });
    }

    memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_STATUS, "running")?;
    memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_TRACE_ID, &Uuid::new_v4().to_string())?;
    memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_DONE_BATCHES, "0")?;
    memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_TOTAL_BATCHES, "0")?;
    memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_ERROR, "")?;

    let sync_result: Result<MemoryStoreProviderSyncReport, String> = (|| -> Result<MemoryStoreProviderSyncReport, String> {
        let tx = conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| format!("Begin provider sync tx failed: {err}"))?;

        let doc_map = memory_store_collect_doc_texts(&tx)?;
        let doc_ids = doc_map.keys().cloned().collect::<StdHashSet<_>>();
        let table = memory_store_provider_table(new_provider_id)?;
        let mut vector_conn = memory_store_open_provider_vector_db_with_model(data_path, model_name)?;
        let vector_tx = vector_conn
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|err| format!("Begin provider vector tx failed: {err}"))?;
        let index_ids = memory_store_provider_index_ids(&vector_tx, &table)?;

        let to_delete = index_ids
            .difference(&doc_ids)
            .cloned()
            .collect::<Vec<_>>();
        let mut to_add = doc_ids
            .difference(&index_ids)
            .cloned()
            .collect::<Vec<_>>();
        to_add.sort();
        let effective_batch = batch_size.max(1);
        let total_batches = if to_add.is_empty() { 0 } else { to_add.len().div_ceil(effective_batch) };
        memory_store_set_runtime_state(&tx, KB_STATE_REBUILD_TOTAL_BATCHES, &total_batches.to_string())?;

        let deleted = memory_store_delete_provider_entries(&vector_tx, &table, &to_delete)?;
        let (added, batch_count, dimension) = memory_store_batch_embed_and_insert(
            &vector_tx,
            &table,
            &doc_map,
            &to_add,
            effective_batch,
            &tx,
            &mut embedder,
        )?;
        vector_tx
            .commit()
            .map_err(|err| format!("Commit provider vector tx failed: {err}"))?;
        memory_store_update_provider_metadata(
            &tx,
            new_provider_id,
            model_name,
            dimension,
            batch_count,
        )?;
        tx.commit()
            .map_err(|err| format!("Commit provider sync tx failed: {err}"))?;

        Ok(MemoryStoreProviderSyncReport {
            status: "synced".to_string(),
            old_provider_id,
            new_provider_id: new_provider_id.trim().to_string(),
            deleted,
            added,
            batch_count,
        })
    })();

    if let Err(err) = sync_result.as_ref() {
        let _ = memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_STATUS, "failed");
        let _ = memory_store_set_runtime_state(&conn, KB_STATE_REBUILD_ERROR, err);
        runtime_log_error(format!(
            "[记忆存储] 失败，任务=向量索引同步，provider_id={}，error={}，duration_ms={}",
            new_provider_id.trim(),
            err,
            started_at.elapsed().as_millis()
        ));
    }

    if let Ok(report) = sync_result.as_ref() {
        runtime_log_info(format!(
            "[记忆存储] 完成，任务=向量索引同步，provider_id={}，deleted={}，added={}，batch_count={}，duration_ms={}",
            new_provider_id.trim(),
            report.deleted,
            report.added,
            report.batch_count,
            started_at.elapsed().as_millis()
        ));
    }

    sync_result
}

/// 是否已配置向量索引绑定（从 src-tauri matcher.rs 迁入）。
pub fn memory_has_embedding_binding(data_path: &std::path::PathBuf) -> bool {
    let Ok(conn) = memory_store_open(data_path) else {
        return false;
    };
    let active = memory_store_get_runtime_state(&conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID)
        .ok()
        .flatten()
        .unwrap_or_default();
    let embedding_api = memory_store_get_runtime_state(&conn, KB_STATE_EMBEDDING_API_CONFIG_ID)
        .ok()
        .flatten()
        .unwrap_or_default();
    !active.trim().is_empty() && !embedding_api.trim().is_empty()
}

/// 当前激活的向量索引 provider（从 src-tauri matcher.rs 迁入）。
pub fn memory_store_active_embedding_provider_id(conn: &Connection) -> Result<Option<String>, String> {
    memory_store_get_runtime_state(conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID)
}

/// 绑定向量索引的 API 配置 id（从 src-tauri matcher.rs 迁入）。
pub fn memory_store_embedding_binding_api_id(conn: &Connection) -> Result<Option<String>, String> {
    memory_store_get_runtime_state(conn, KB_STATE_EMBEDDING_API_CONFIG_ID)
}

/// provider 对应的嵌入模型名（从 src-tauri matcher.rs 迁入）。
pub fn memory_store_embedding_provider_model_name(
    conn: &Connection,
    provider_id: &str,
) -> Result<Option<String>, String> {
    conn.query_row(
        "SELECT model_name FROM embedding_provider WHERE provider_id=?1",
        params![provider_id],
        |row| row.get::<_, String>(0),
    )
    .optional()
    .map_err(|err| format!("Query embedding provider model name failed: {err}"))
}

