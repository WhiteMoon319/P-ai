use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use uuid::Uuid;

use crate::core::time_semantics::now_iso;
use super::*;

/// 会话变更互斥（简化版：全局 key→锁，从 runtime_lock.rs 迁入）。
fn with_conversation_mutation_for_data_path<T, F>(
    data_path: &PathBuf,
    conversation_id: &str,
    _task_name: &str,
    f: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    static GATES: OnceLock<Mutex<std::collections::HashMap<String, std::sync::Arc<Mutex<()>>>>> =
        OnceLock::new();
    let key = format!("{}:{}", data_path.display(), conversation_id.trim());
    let gate = {
        let mut gates = GATES
            .get_or_init(|| Mutex::new(std::collections::HashMap::new()))
            .lock()
            .unwrap_or_else(|poison| poison.into_inner());
        gates
            .entry(key)
            .or_insert_with(|| std::sync::Arc::new(Mutex::new(())))
            .clone()
    };
    let _guard = gate.lock().unwrap_or_else(|poison| poison.into_inner());
    f()
}

/// 打开聊天元数据 DB（精简版：仅确保 active_plan_records 表存在，
/// 完整 schema 由 src-tauri sqlite.rs 建立；此处只补充缺失表）。
pub(crate) fn chat_metadata_store_open(data_path: &PathBuf) -> Result<rusqlite::Connection, String> {
    let db_path = chat_metadata_store_db_path(data_path);
    if let Some(parent) = db_path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!("创建聊天元数据数据库目录失败，path={}，error={err}", parent.display())
        })?;
    }
    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|err| format!("打开聊天元数据数据库失败，path={}，error={err}", db_path.display()))?;
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=10000;
         CREATE TABLE IF NOT EXISTS active_plan_records (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           conversation_id TEXT NOT NULL,
           plan_id TEXT NOT NULL,
           record_json TEXT NOT NULL,
           created_at TEXT NOT NULL DEFAULT ''
         );",
    )
    .map_err(|err| format!("初始化聊天元数据数据库失败: {err}"))?;
    Ok(conn)
}

/// 读取活跃计划（SQLite 版，从 src-tauri sqlite.rs 迁入）。
fn chat_metadata_store_read_active_plans(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Option<Vec<ActivePlanRecord>>, String> {
    if !chat_metadata_store_db_path(data_path).exists() {
        return Ok(None);
    }
    let conn = chat_metadata_store_open(data_path)?;
    let mut stmt = conn.prepare(
        "SELECT record_json FROM active_plan_records WHERE conversation_id=?1 ORDER BY rowid DESC",
    ).map_err(|err| format!("准备读取 SQLite 活动计划失败: {err}"))?;
    let rows = stmt.query_map([conversation_id], |row| row.get::<_, String>(0))
        .map_err(|err| format!("读取 SQLite 活动计划失败: {err}"))?;
    let mut out = Vec::new();
    for row in rows {
        let raw = row.map_err(|err| format!("读取 SQLite 活动计划记录失败: {err}"))?;
        out.push(serde_json::from_str(&raw).map_err(|err| format!("解析 SQLite 活动计划记录失败: {err}"))?);
    }
    Ok(Some(out))
}

/// 追加活跃计划（SQLite 版，从 src-tauri sqlite.rs 迁入）。
fn chat_metadata_store_append_active_plan(
    paths: &MessageStorePaths,
    record: &ActivePlanRecord,
) -> Result<(), String> {
    let conn = chat_metadata_store_open(&paths.data_path)?;
    let raw = serde_json::to_string(record)
        .map_err(|err| format!("序列化 SQLite 活动计划失败: {err}"))?;
    conn.execute(
        "INSERT INTO active_plan_records(conversation_id, plan_id, record_json) VALUES (?1, ?2, ?3)",
        rusqlite::params![paths.conversation_id, record.plan_id, raw],
    )
    .map_err(|err| format!("写入 SQLite 活动计划失败: {err}"))?;
    Ok(())
}

/// 按路径完成活跃计划（SQLite 版，从 src-tauri sqlite.rs 迁入）。
fn chat_metadata_store_complete_active_plan_by_path(
    paths: &MessageStorePaths,
    normalized_path: &str,
    completion_text: Option<&str>,
) -> Result<bool, String> {
    let conn = chat_metadata_store_open(&paths.data_path)?;
    let matched = {
        let mut stmt = conn
            .prepare(
                "SELECT rowid, record_json FROM active_plan_records
                 WHERE conversation_id=?1 ORDER BY rowid DESC",
            )
            .map_err(|err| format!("准备读取 SQLite 活动计划失败: {err}"))?;
        let rows = stmt
            .query_map([&paths.conversation_id], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(|err| format!("读取 SQLite 活动计划失败: {err}"))?;
        let mut matched = None;
        for row in rows {
            let (rowid, raw) = row.map_err(|err| format!("读取 SQLite 活动计划记录失败: {err}"))?;
            let record = serde_json::from_str::<ActivePlanRecord>(&raw)
                .map_err(|err| format!("解析 SQLite 活动计划记录失败: {err}"))?;
            if record.status.trim() == ACTIVE_PLAN_STATUS_IN_PROGRESS
                && record.path.trim().eq_ignore_ascii_case(normalized_path)
            {
                matched = Some((rowid, record));
                break;
            }
        }
        matched
    };
    let Some((rowid, mut record)) = matched else {
        return Ok(false);
    };
    record.status = ACTIVE_PLAN_STATUS_COMPLETED.to_string();
    record.completed_at = Some(now_iso());
    record.completion_text = completion_text
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let raw = serde_json::to_string(&record)
        .map_err(|err| format!("序列化 SQLite 活动计划失败: {err}"))?;
    let updated = conn
        .execute(
            "UPDATE active_plan_records SET record_json=?1 WHERE rowid=?2",
            rusqlite::params![raw, rowid],
        )
        .map_err(|err| format!("更新 SQLite 活动计划失败: {err}"))?;
    if updated != 1 {
        return Err(format!("更新 SQLite 活动计划失败：记录不存在，rowid={rowid}"));
    }
    Ok(true)
}

pub const ACTIVE_PLAN_STATUS_IN_PROGRESS: &str = "in_progress";
pub const ACTIVE_PLAN_STATUS_COMPLETED: &str = "completed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePlanRecord {
    pub plan_id: String,
    pub source_message_id: String,
    pub status: String,
    #[serde(default)]
    pub path: String,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completion_text: Option<String>,
}

pub fn encode_active_plan_record(record: &ActivePlanRecord) -> Result<String, String> {
    serde_json::to_string(record)
        .map(|value| format!("{value}\n"))
        .map_err(|err| format!("序列化执行中计划失败: {err}"))
}

pub fn read_active_plan_records(path: &PathBuf) -> Result<Vec<ActivePlanRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "读取执行中计划文件失败，path={}，error={err}",
            path.display()
        )
    })?;
    let mut records = Vec::<ActivePlanRecord>::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record = serde_json::from_str::<ActivePlanRecord>(trimmed).map_err(|err| {
            format!(
                "解析执行中计划失败，path={}，line={}，error={err}",
                path.display(),
                index + 1
            )
        })?;
        if record.path.trim().is_empty() {
            continue;
        }
        records.push(record);
    }
    Ok(records)
}

pub fn write_active_plan_records(path: &PathBuf, records: &[ActivePlanRecord]) -> Result<(), String> {
    let mut content = String::new();
    for record in records {
        content.push_str(&encode_active_plan_record(record)?);
    }
    write_message_store_text_atomic(path, "jsonl.tmp", &content, "执行中计划")
}

pub fn append_active_plan_record(path: &PathBuf, record: &ActivePlanRecord) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "创建执行中计划目录失败，path={}，error={err}",
                parent.display()
            )
        })?;
    }
    let line = encode_active_plan_record(record)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| {
            format!(
                "打开执行中计划文件失败，path={}，error={err}",
                path.display()
            )
        })?;
    use std::io::Write as _;
    file.write_all(line.as_bytes()).map_err(|err| {
        format!(
            "追加执行中计划失败，path={}，error={err}",
            path.display()
        )
    })
}

pub fn active_plan_records_in_progress(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Vec<ActivePlanRecord>, String> {
    let paths = message_store_paths(data_path, conversation_id)?;
    if paths.is_v3_ready()? {
        return Ok(chat_metadata_store_read_active_plans(data_path, conversation_id)?
            .unwrap_or_default()
            .into_iter()
            .filter(|record| record.status.trim() == ACTIVE_PLAN_STATUS_IN_PROGRESS)
            .collect());
    }
    Ok(read_active_plan_records(&paths.active_plans_file)?
        .into_iter()
        .rev()
        .filter(|record| record.status.trim() == ACTIVE_PLAN_STATUS_IN_PROGRESS)
        .collect())
}

pub fn active_plan_append_in_progress(
    data_path: &PathBuf,
    conversation_id: &str,
    source_message_id: &str,
    path: &str,
) -> Result<(), String> {
    let paths = message_store_paths(data_path, conversation_id)?;
    let record = ActivePlanRecord {
        plan_id: Uuid::new_v4().to_string(),
        source_message_id: source_message_id.trim().to_string(),
        status: ACTIVE_PLAN_STATUS_IN_PROGRESS.to_string(),
        path: path.trim().to_string(),
        created_at: now_iso(),
        completed_at: None,
        completion_text: None,
    };
    if record.source_message_id.is_empty() {
        return Err("sourceMessageId 为空，无法写入执行中计划。".to_string());
    }
    if record.path.is_empty() {
        return Err("计划路径为空，无法写入执行中计划。".to_string());
    }
    with_conversation_mutation_for_data_path(
        data_path,
        conversation_id,
        "active_plan_append_in_progress",
        || {
            if paths.is_v3_ready()? {
                return chat_metadata_store_append_active_plan(&paths, &record);
            }
            append_active_plan_record(&paths.active_plans_file, &record)?;
            Ok(())
        },
    )
}

pub fn active_plan_complete_by_path(
    data_path: &PathBuf,
    conversation_id: &str,
    path: &str,
    completion_text: Option<&str>,
) -> Result<bool, String> {
    let normalized_path = path.trim();
    if normalized_path.is_empty() {
        return Err("计划路径为空，无法完成执行中计划。".to_string());
    }
    let paths = message_store_paths(data_path, conversation_id)?;
    with_conversation_mutation_for_data_path(
        data_path,
        conversation_id,
        "active_plan_complete_by_path",
        || {
            if paths.is_v3_ready()? {
                return chat_metadata_store_complete_active_plan_by_path(
                    &paths,
                    normalized_path,
                    completion_text,
                );
            }
            let mut records = read_active_plan_records(&paths.active_plans_file)?;
            let Some(index) = records
                .iter()
                .rposition(|record| {
                    record.status.trim() == ACTIVE_PLAN_STATUS_IN_PROGRESS
                        && record.path.trim().eq_ignore_ascii_case(normalized_path)
                })
            else {
                return Ok(false);
            };
            records[index].status = ACTIVE_PLAN_STATUS_COMPLETED.to_string();
            records[index].completed_at = Some(now_iso());
            records[index].completion_text = completion_text
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToOwned::to_owned);
            write_active_plan_records(&paths.active_plans_file, &records)?;
            Ok(true)
        },
    )
}

#[cfg(test)]
#[test]
pub fn read_active_plan_records_should_skip_legacy_record_without_path() {
    let root = std::env::temp_dir().join(format!("eca-active-plan-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp dir");
    let file = root.join("active_plans.jsonl");
    fs::write(
        &file,
        concat!(
            "{\"planId\":\"legacy\",\"sourceMessageId\":\"msg-1\",\"status\":\"in_progress\",\"createdAt\":\"2026-01-01T00:00:00Z\"}\n",
            "{\"planId\":\"valid\",\"sourceMessageId\":\"msg-2\",\"status\":\"in_progress\",\"path\":\"C:/plan.md\",\"createdAt\":\"2026-01-01T00:00:00Z\"}\n"
        ),
    )
    .expect("write active plans");

    let records = read_active_plan_records(&file).expect("read active plans");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].plan_id, "valid");
    assert_eq!(records[0].path, "C:/plan.md");

    let _ = fs::remove_dir_all(root);
}

#[cfg(test)]
#[test]
pub fn active_plan_records_in_progress_should_return_newest_first() {
    let root = std::env::temp_dir().join(format!("eca-active-plan-order-{}", Uuid::new_v4()));
    let conversation_id = "conv-active-plan-order";
    let paths = message_store_paths(&root, conversation_id).expect("message store paths");
    fs::create_dir_all(paths.active_plans_file.parent().expect("active plans dir"))
        .expect("create active plans dir");
    fs::write(
        &paths.active_plans_file,
        concat!(
            "{\"planId\":\"old\",\"sourceMessageId\":\"msg-1\",\"status\":\"in_progress\",\"path\":\"C:/old.md\",\"createdAt\":\"2026-01-01T00:00:00Z\"}\n",
            "{\"planId\":\"done\",\"sourceMessageId\":\"msg-2\",\"status\":\"completed\",\"path\":\"C:/done.md\",\"createdAt\":\"2026-01-01T00:00:01Z\"}\n",
            "{\"planId\":\"new\",\"sourceMessageId\":\"msg-3\",\"status\":\"in_progress\",\"path\":\"C:/new.md\",\"createdAt\":\"2026-01-01T00:00:02Z\"}\n"
        ),
    )
    .expect("write active plans");

    let records =
        active_plan_records_in_progress(&root, conversation_id).expect("read active plans");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].plan_id, "new");
    assert_eq!(records[1].plan_id, "old");

    let _ = fs::remove_dir_all(root);
}
