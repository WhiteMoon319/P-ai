use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use futures_util::{future::AbortHandle, future::join_all, future::BoxFuture, StreamExt};
use image::ImageFormat;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rmcp::{schemars, ServiceExt};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use uuid::Uuid;


use std::collections::{HashSet};
use super::*;
// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐


pub(crate) fn migrate_app_data_inline_media_to_refs(data_path: &PathBuf, data: &mut AppData) -> bool {
    let mut changed = false;
    for conversation in &mut data.conversations {
        for message in &mut conversation.messages {
            changed |= externalize_message_parts_to_media_refs_lossy(&mut message.parts, data_path);
        }
    }
    for archive in &mut data.archived_conversations {
        for message in &mut archive.source_conversation.messages {
            changed |= externalize_message_parts_to_media_refs_lossy(&mut message.parts, data_path);
        }
    }
    changed
}

pub(crate) fn migrate_app_data_archives_into_conversations(
    data_path: &PathBuf,
    data: &mut AppData,
) -> Result<bool, String> {
    if data.archived_conversations.is_empty() {
        return Ok(false);
    }
    let backup_file = app_layout_backups_dir(data_path).join(format!(
        "app_data.pre_archive_merge.{}.json",
        now_utc().unix_timestamp()
    ));
    write_json_file_atomic(&backup_file, data, "pre-migration app_data backup")?;

    for archive in data.archived_conversations.clone() {
        let mut conv = archive.source_conversation;
        if conv.id.trim().is_empty() {
            conv.id = Uuid::new_v4().to_string();
        }
        if conv.archived_at.as_deref().unwrap_or("").trim().is_empty() {
            conv.archived_at = Some(archive.archived_at.clone());
        }
        if conv.status.trim() != "archived" {
            conv.status = "archived".to_string();
        }
        conv.fast_request_turns.clear();

        if let Some(existing_idx) = data.conversations.iter().position(|c| c.id == conv.id) {
            let should_replace = {
                let existing = &data.conversations[existing_idx];
                existing.summary.trim().is_empty() && !conv.summary.trim().is_empty()
            };
            if should_replace {
                data.conversations[existing_idx] = conv;
            }
        } else {
            data.conversations.push(conv);
        }
    }

    data.archived_conversations.clear();
    Ok(true)
}

pub(crate) fn migrate_agent_avatar_paths(data_path: &PathBuf, data: &mut AppData) -> bool {
    let root = app_root_from_data_path(data_path);
    let new_avatar_dir = root.join("avatars");
    let legacy_avatar_dir = root.join("config").join("avatars");
    let mut changed = false;

    for agent in &mut data.agents {
        let Some(path_raw) = agent.avatar_path.as_ref() else {
            continue;
        };
        if path_raw.trim().is_empty() {
            continue;
        }
        let old_path = PathBuf::from(path_raw);
        let file_name = old_path
            .file_name()
            .map(|v| v.to_owned())
            .or_else(|| {
                PathBuf::from(path_raw)
                    .components()
                    .last()
                    .map(|c| std::ffi::OsString::from(c.as_os_str()))
            });
        let Some(file_name) = file_name else {
            continue;
        };
        let new_path = new_avatar_dir.join(file_name);

        if new_path.exists() {
            let next = new_path.to_string_lossy().to_string();
            if next != *path_raw {
                agent.avatar_path = Some(next);
                changed = true;
            }
            continue;
        }

        let legacy_candidate = if old_path.exists() {
            old_path.clone()
        } else {
            legacy_avatar_dir.join(
                old_path
                    .file_name()
                    .unwrap_or_else(|| std::ffi::OsStr::new("")),
            )
        };
        if !legacy_candidate.exists() {
            continue;
        }
        let _ = fs::create_dir_all(&new_avatar_dir);
        if fs::rename(&legacy_candidate, &new_path).is_err() {
            if fs::copy(&legacy_candidate, &new_path).is_ok() {
                let _ = fs::remove_file(&legacy_candidate);
            }
        }
        if new_path.exists() {
            let next = new_path.to_string_lossy().to_string();
            if next != *path_raw {
                agent.avatar_path = Some(next);
                changed = true;
            }
        }
    }

    changed
}

pub(crate) const LEGACY_APP_DATA_SPLIT_DIR_NAME: &str = "app_data";

pub(crate) const LAYOUT_DIR_CONFIG: &str = "config";
pub(crate) const LAYOUT_DIR_STATE: &str = "state";
pub(crate) const LAYOUT_DIR_CHAT: &str = "chat";
pub(crate) const LAYOUT_DIR_CHAT_CONVERSATIONS: &str = "conversations";
pub(crate) const LAYOUT_DIR_BACKUPS: &str = "backups";
pub(crate) const LAYOUT_FILE_AGENTS: &str = "agents.json";
pub(crate) const LAYOUT_FILE_RUNTIME: &str = "runtime_state.json";
pub(crate) const LAYOUT_FILE_CHAT_INDEX: &str = "index.json";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentsFile {
    #[serde(default)]
    pub(crate) agents: Vec<AgentProfile>,
}

// ChatIndexConversationItem / ChatIndexFile 已迁至 crates/pai-backend
// message_store::sqlite（阶段 4/6），通过 crate 根重导出生效。

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AppDataWriteStats {
    pub(crate) agents_written: bool,
    pub(crate) runtime_written: bool,
    pub(crate) conversation_writes: usize,
    pub(crate) conversation_deletes: usize,
}

pub(crate) fn app_layout_config_dir(path: &PathBuf) -> PathBuf {
    app_root_from_data_path(path).join(LAYOUT_DIR_CONFIG)
}

pub(crate) fn app_layout_state_dir(path: &PathBuf) -> PathBuf {
    app_root_from_data_path(path).join(LAYOUT_DIR_STATE)
}

// app_layout_chat_dir / app_layout_chat_conversations_dir 已迁至
// crates/pai-backend message_store::paths（阶段 4），通过 crate 根重导出生效。

pub(crate) fn app_layout_backups_dir(path: &PathBuf) -> PathBuf {
    app_root_from_data_path(path).join(LAYOUT_DIR_BACKUPS)
}

pub(crate) fn app_layout_agents_path(path: &PathBuf) -> PathBuf {
    app_layout_config_dir(path).join(LAYOUT_FILE_AGENTS)
}

pub(crate) fn app_layout_runtime_state_path(path: &PathBuf) -> PathBuf {
    app_layout_state_dir(path).join(LAYOUT_FILE_RUNTIME)
}

pub(crate) fn app_layout_chat_index_path(path: &PathBuf) -> PathBuf {
    pai_backend::message_store::paths::app_layout_chat_dir(path).join(LAYOUT_FILE_CHAT_INDEX)
}

pub(crate) fn app_layout_chat_conversation_path(path: &PathBuf, conversation_id: &str) -> PathBuf {
    pai_backend::message_store::paths::app_layout_chat_conversations_dir(path).join(format!("{conversation_id}.json"))
}

pub(crate) fn build_agents_file(agents: &[AgentProfile]) -> AgentsFile {
    AgentsFile {
        agents: agents.to_vec(),
    }
}

pub(crate) fn normalize_runtime_state_contact_communication(runtime: &mut RuntimeStateFile) {
    for contact in &mut runtime.remote_im_contacts {
        contact.allow_send = contact.allow_send || contact.allow_receive;
        contact.allow_receive = contact.allow_send;
    }
}

pub(crate) fn normalize_runtime_state_system_notification_pointer(runtime: &mut RuntimeStateFile) -> bool {
    if runtime.main_conversation_id.as_deref().map(str::trim)
        == Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
    {
        return false;
    }
    runtime.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    true
}

pub(crate) fn system_notification_conversation_shard_has_artifacts(path: &PathBuf) -> Result<bool, String> {
    if app_layout_chat_conversation_path(path, SYSTEM_NOTIFICATION_CONVERSATION_ID).exists() {
        return Ok(true);
    }
    let store_paths = message_store::message_store_paths(path, SYSTEM_NOTIFICATION_CONVERSATION_ID)?;
    Ok(message_store::message_store_shard_modified_time(&store_paths).is_some())
}

pub(crate) fn ensure_system_notification_conversation_shard(path: &PathBuf) -> Result<bool, String> {
    match read_conversation_shard(path, SYSTEM_NOTIFICATION_CONVERSATION_ID) {
        Ok(mut conversation) => {
            if normalize_system_notification_conversation(&mut conversation) {
                return write_conversation_shard(path, &conversation);
            }
            Ok(false)
        }
        Err(err) => {
            if system_notification_conversation_shard_has_artifacts(path)? {
                runtime_log_warn(format!(
                    "[系统通知会话] 跳过，任务=确保固定会话分片，原因=固定会话分片已存在但暂不可读，conversation_id={}，error={}",
                    SYSTEM_NOTIFICATION_CONVERSATION_ID,
                    err
                ));
                return Ok(false);
            }
            let conversation = build_system_notification_conversation_record();
            write_conversation_shard(path, &conversation)
        }
    }
}

pub(crate) fn build_runtime_state_file(data: &AppData) -> RuntimeStateFile {
    let mut runtime = RuntimeStateFile {
        version: APP_DATA_SCHEMA_VERSION,
        runtime_revision: 0,
        data_migration_version: data.data_migration_version,
        message_store_migration_version: data.message_store_migration_version,
        assistant_department_agent_id: data.assistant_department_agent_id.clone(),
        response_style_id: data.response_style_id.clone(),
        pdf_read_mode: data.pdf_read_mode.clone(),
        background_voice_screenshot_keywords: data.background_voice_screenshot_keywords.clone(),
        background_voice_screenshot_mode: data.background_voice_screenshot_mode.clone(),
        instruction_presets: data.instruction_presets.clone(),
        main_conversation_id: Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string()),
        pinned_conversation_ids: data.pinned_conversation_ids.clone(),
        conversation_section_orders: data.conversation_section_orders.clone(),
        image_text_cache: data.image_text_cache.clone(),
        pdf_text_cache: data.pdf_text_cache.clone(),
        pdf_image_cache: data.pdf_image_cache.clone(),
        remote_im_contacts: data.remote_im_contacts.clone(),
        remote_im_contact_checkpoints: data.remote_im_contact_checkpoints.clone(),
    };
    normalize_runtime_state_contact_communication(&mut runtime);
    let _ = normalize_runtime_state_system_notification_pointer(&mut runtime);
    runtime
}

pub(crate) fn build_chat_index_item(conversation: &Conversation) -> ChatIndexConversationItem {
    ChatIndexConversationItem {
        id: conversation.id.clone(),
        updated_at: conversation.updated_at.clone(),
        status: conversation.status.clone(),
        summary: conversation.summary.clone(),
        archived_at: conversation.archived_at.clone(),
    }
}

#[cfg(test)]
pub(crate) fn build_chat_index_file(conversations: &[Conversation]) -> ChatIndexFile {
    ChatIndexFile {
        conversations: conversations
            .iter()
            .map(build_chat_index_item)
            .collect::<Vec<_>>(),
    }
}

/// 从 Conversation 构建索引项后 upsert（Conversation 仍为 src-tauri 类型，
/// 故保留在本地；索引结构操作已迁至 pai-backend）。
pub(crate) fn upsert_chat_index_from_conversation(
    index: &mut ChatIndexFile,
    conversation: &Conversation,
) {
    let item = build_chat_index_item(conversation);
    upsert_chat_index_conversation(index, item);
}

pub(crate) fn apply_runtime_state_to_app_data(data: &mut AppData, runtime: &RuntimeStateFile) {
    data.version = runtime.version;
    data.data_migration_version = runtime.data_migration_version;
    data.message_store_migration_version = runtime.message_store_migration_version;
    data.assistant_department_agent_id = runtime.assistant_department_agent_id.clone();
    data.response_style_id = runtime.response_style_id.clone();
    data.pdf_read_mode = runtime.pdf_read_mode.clone();
    data.background_voice_screenshot_keywords =
        runtime.background_voice_screenshot_keywords.clone();
    data.background_voice_screenshot_mode = runtime.background_voice_screenshot_mode.clone();
    data.instruction_presets = runtime.instruction_presets.clone();
    data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    data.pinned_conversation_ids = runtime.pinned_conversation_ids.clone();
    data.conversation_section_orders = runtime.conversation_section_orders.clone();
    data.image_text_cache = runtime.image_text_cache.clone();
    data.pdf_text_cache = runtime.pdf_text_cache.clone();
    data.pdf_image_cache = runtime.pdf_image_cache.clone();
    data.remote_im_contacts = runtime.remote_im_contacts.clone();
    data.remote_im_contact_checkpoints = runtime.remote_im_contact_checkpoints.clone();
}

pub(crate) fn read_agents_shard(path: &PathBuf) -> Result<Vec<AgentProfile>, String> {
    let mut agents = if !app_layout_exists(path) && path.exists() {
        read_app_data(path)?.agents
    } else if app_layout_agents_path(path).exists() {
        read_json_file::<AgentsFile>(&app_layout_agents_path(path), "agents file")?.agents
    } else {
        AppData::default().agents
    };
    ensure_required_builtin_agents_in_list(&mut agents);
    Ok(agents)
}

pub(crate) fn write_agents_shard(path: &PathBuf, agents: &[AgentProfile]) -> Result<bool, String> {
    fs::create_dir_all(app_layout_config_dir(path))
        .map_err(|err| format!("Create config layout dir failed: {err}"))?;
    let mut normalized_agents = agents.to_vec();
    ensure_required_builtin_agents_in_list(&mut normalized_agents);
    write_json_file_atomic_if_changed(
        &app_layout_agents_path(path),
        &build_agents_file(&normalized_agents),
        "agents file",
    )
}

pub(crate) fn read_runtime_state_shard(path: &PathBuf) -> Result<RuntimeStateFile, String> {
    let mut runtime = if app_layout_runtime_state_path(path).exists() {
        read_json_file::<RuntimeStateFile>(&app_layout_runtime_state_path(path), "runtime state file")?
    } else {
        RuntimeStateFile::default()
    };
    normalize_runtime_state_contact_communication(&mut runtime);
    let _ = normalize_runtime_state_system_notification_pointer(&mut runtime);
    ensure_system_notification_conversation_shard(path)?;
    Ok(runtime)
}

pub(crate) fn write_runtime_state_shard(path: &PathBuf, runtime: &RuntimeStateFile) -> Result<bool, String> {
    fs::create_dir_all(app_layout_state_dir(path))
        .map_err(|err| format!("Create state layout dir failed: {err}"))?;
    let mut normalized = runtime.clone();
    normalize_runtime_state_contact_communication(&mut normalized);
    let _ = normalize_runtime_state_system_notification_pointer(&mut normalized);
    let conversation_written = ensure_system_notification_conversation_shard(path)?;
    let runtime_written = write_json_file_atomic_if_changed(
        &app_layout_runtime_state_path(path),
        &normalized,
        "runtime state file",
    )?;
    Ok(runtime_written || conversation_written)
}

pub(crate) fn read_conversation_shard(path: &PathBuf, conversation_id: &str) -> Result<Conversation, String> {
    let mut conversation = read_conversation_shard_raw(path, conversation_id)?;
    normalize_conversation_runtime_volatile_fields(&mut conversation);
    Ok(conversation)
}

pub(crate) fn read_conversation_meta_shard(
    path: &PathBuf,
    conversation_id: &str,
) -> Result<message_store::ConversationShardMeta, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("Conversation id is empty".to_string());
    }
    let store_paths = message_store::message_store_paths(path, conversation_id)?;
    match message_store::read_ready_message_store_meta(&store_paths) {
        Ok(Some(meta))
            if meta.schema_version() >= message_store::CONVERSATION_META_SCHEMA_VERSION =>
        {
            if meta.cumulative_usage().needs_legacy_total_tokens_backfill() {
                let repaired = meta.clone().normalized_legacy_usage_totals();
                write_conversation_meta_shard_from_meta(path, &repaired)?;
                return Ok(repaired);
            }
            return Ok(meta);
        }
        Ok(Some(_)) | Ok(None) | Err(_) => {}
    }
    if message_store::message_store_is_v3_ready(&store_paths)? {
        return Err(format!("Conversation '{conversation_id}' not found."));
    }
    if message_store::read_message_store_manifest_status(&store_paths)?.is_some() {
        let conversation = read_conversation_shard_raw(path, conversation_id)?;
        let rebuilt = message_store::ConversationShardMeta::from_conversation(&conversation);
        write_conversation_meta_shard_from_meta(path, &rebuilt)?;
        return Ok(rebuilt);
    }
    let conversation_path = app_layout_chat_conversation_path(path, conversation_id);
    if conversation_path.exists() {
        let conversation = read_json_file::<Conversation>(&conversation_path, "conversation file")?;
        let rebuilt = message_store::ConversationShardMeta::from_conversation(&conversation);
        write_conversation_meta_shard_from_meta(path, &rebuilt)?;
        return Ok(rebuilt);
    }
    Err(format!("Conversation '{conversation_id}' not found."))
}

pub(crate) fn refresh_conversation_meta_shard_if_needed(
    path: &PathBuf,
    conversation_id: &str,
) -> Result<bool, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Ok(false);
    }
    let store_paths = message_store::message_store_paths(path, conversation_id)?;
    if let Ok(Some(meta)) = message_store::read_ready_message_store_meta(&store_paths) {
        if meta.schema_version() >= message_store::CONVERSATION_META_SCHEMA_VERSION {
            return Ok(false);
        }
    }
    if message_store::message_store_is_v3_ready(&store_paths)? {
        return Ok(false);
    }
    if message_store::read_message_store_manifest_status(&store_paths)?.is_none() {
        let conversation_path = app_layout_chat_conversation_path(path, conversation_id);
        if !conversation_path.exists() && (app_layout_exists(path) || !path.exists()) {
            return Ok(false);
        }
    }
    let _ = read_conversation_meta_shard(path, conversation_id)?;
    Ok(true)
}

pub(crate) fn read_conversation_shard_raw(path: &PathBuf, conversation_id: &str) -> Result<Conversation, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("Conversation id is empty".to_string());
    }
    let store_paths = message_store::message_store_paths(path, conversation_id)?;
    if let Some(conversation) =
        message_store::read_ready_message_store_directory_conversation(&store_paths)?
    {
        if conversation
            .cumulative_usage
            .needs_legacy_total_tokens_backfill()
        {
            let mut repaired = conversation;
            repaired.cumulative_usage = repaired.cumulative_usage.clone().normalized_legacy_totals();
            let _ = write_conversation_shard(path, &repaired)?;
            return Ok(repaired);
        }
        return Ok(conversation);
    }
    if message_store::message_store_is_v3_ready(&store_paths)? {
        return Err(format!("Conversation '{conversation_id}' not found."));
    }
    let recovered_manifest =
        message_store::recover_ready_jsonl_snapshot_manifest_from_directory(&store_paths)?;
    if recovered_manifest.is_some() {
        if let Some(conversation) =
            message_store::read_ready_message_store_directory_conversation(&store_paths)?
        {
            if conversation
                .cumulative_usage
                .needs_legacy_total_tokens_backfill()
            {
                let mut repaired = conversation;
                repaired.cumulative_usage = repaired.cumulative_usage.clone().normalized_legacy_totals();
                let _ = write_conversation_shard(path, &repaired)?;
                return Ok(repaired);
            }
            return Ok(conversation);
        }
    }
    if let Some(status) = message_store::read_message_store_manifest_status(&store_paths)? {
        return Err(format!(
            "会话消息仓库未处于可读取状态，conversation_id={}，kind={}，state={}",
            conversation_id, status.message_store_kind, status.migration_state
        ));
    }
    let conversation_path = app_layout_chat_conversation_path(path, conversation_id);
    if conversation_path.exists() {
        let conversation = read_json_file::<Conversation>(&conversation_path, "conversation file")?;
        if conversation
            .cumulative_usage
            .needs_legacy_total_tokens_backfill()
        {
            let mut repaired = conversation;
            repaired.cumulative_usage = repaired.cumulative_usage.clone().normalized_legacy_totals();
            let _ = write_conversation_shard(path, &repaired)?;
            return Ok(repaired);
        }
        return Ok(conversation);
    }
    Err(format!("Conversation '{conversation_id}' not found."))
}

pub(crate) fn write_conversation_shard(path: &PathBuf, conversation: &Conversation) -> Result<bool, String> {
    fs::create_dir_all(pai_backend::message_store::paths::app_layout_chat_conversations_dir(path))
        .map_err(|err| format!("Create chat conversations dir failed: {err}"))?;
    let store_paths = message_store::message_store_paths(path, &conversation.id)?;
    if message_store::message_store_is_v3_ready(&store_paths)?
        && message_store::read_ready_message_store_meta(&store_paths)?.is_some()
    {
        // v3 的正文只能由 append/replace/truncate/splice 原子接口发布。
        // 后台 metadata 刷新不得整读或重建 locator 与 JSONL block。
        write_conversation_meta_shard_from_meta(
            path,
            &message_store::ConversationShardMeta::from_conversation(conversation),
        )?;
        return Ok(true);
    }
    message_store::write_jsonl_snapshot_directory_shard_if_changed(&store_paths, conversation)
}

pub(crate) fn write_conversation_meta_shard_from_meta(
    path: &PathBuf,
    meta: &message_store::ConversationShardMeta,
) -> Result<(), String> {
    let paths = message_store::message_store_paths(path, meta.id())?;
    let mut meta_to_persist = meta.clone();
    if let Some(ready_meta) = message_store::read_ready_message_store_meta(&paths)? {
        meta_to_persist.preserve_message_derived_fields_from(&ready_meta);
    }
    let persist_meta = meta_to_persist.to_persist_meta();
    message_store::write_conversation_directory_meta_shard(&paths, &persist_meta)
}

pub(crate) fn delete_conversation_shard(path: &PathBuf, conversation_id: &str) -> Result<bool, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Ok(false);
    }
    let store_paths = message_store::message_store_paths(path, conversation_id)?;
    message_store::delete_message_store_shard_artifacts(&store_paths)
}

pub(crate) fn app_layout_exists(path: &PathBuf) -> bool {
    app_layout_agents_path(path).exists()
        || app_layout_runtime_state_path(path).exists()
        || pai_backend::message_store::paths::app_layout_chat_conversations_dir(path).exists()
}

pub(crate) fn legacy_app_data_split_dir(path: &PathBuf) -> PathBuf {
    let parent = path
        .parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| PathBuf::from("."));
    parent.join(LEGACY_APP_DATA_SPLIT_DIR_NAME)
}

pub(crate) fn read_json_file<T>(path: &PathBuf, label: &str) -> Result<T, String>
where
    T: serde::de::DeserializeOwned,
{
    let content = fs::read_to_string(path).map_err(|err| format!("Read app_data failed: {err}"))?;
    serde_json::from_str::<T>(&content).map_err(|err| {
        runtime_log_error(format!("[配置] 解析{label}失败 ({}): {err}", path.display()));
        format!("Parse {label} failed ({}): {err}", path.display())
    })
}

pub(crate) fn file_metadata_signature(path: &PathBuf) -> (u64, Option<std::time::SystemTime>) {
    match fs::metadata(path) {
        Ok(metadata) => (metadata.len(), metadata.modified().ok()),
        Err(_) => (0, None),
    }
}

pub(crate) fn update_conversation_cache_signature_for_file(
    conversations: &mut ConversationDirCacheSignature,
    file_path: &PathBuf,
    file_name: String,
) {
    let Ok(metadata) = fs::metadata(file_path) else {
        return;
    };
    if !metadata.is_file() {
        return;
    }
    conversations.file_count += 1;
    conversations.total_size = conversations.total_size.saturating_add(metadata.len());
    let modified = metadata.modified().ok();
    let should_replace_latest = match (
        conversations.latest_modified,
        modified,
        conversations.latest_file_name.as_str(),
    ) {
        (None, Some(_), _) => true,
        (None, None, current_name) => file_name.as_str() > current_name,
        (Some(current), Some(next), current_name) => {
            next > current || (next == current && file_name.as_str() > current_name)
        }
        (Some(_), None, _) => false,
    };
    if should_replace_latest {
        conversations.latest_modified = modified;
        conversations.latest_file_name = file_name;
    }
}

pub(crate) fn app_data_cache_signature(path: &PathBuf) -> AppDataCacheSignature {
    let agents_path = app_layout_agents_path(path);
    let runtime_path = app_layout_runtime_state_path(path);
    let (agents_len, agents_modified) = file_metadata_signature(&agents_path);
    let (runtime_len, runtime_modified) = file_metadata_signature(&runtime_path);

    let mut conversations = ConversationDirCacheSignature::default();
    let conversations_dir = pai_backend::message_store::paths::app_layout_chat_conversations_dir(path);
    if let Ok(entries) = fs::read_dir(conversations_dir) {
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let file_name = entry.file_name().to_string_lossy().to_string();
            if entry_path.extension().and_then(|value| value.to_str()) == Some("json") {
                update_conversation_cache_signature_for_file(
                    &mut conversations,
                    &entry_path,
                    file_name,
                );
                continue;
            }
            if !entry_path.is_dir() {
                continue;
            }
            for shard_file_name in [
                message_store::MESSAGE_STORE_MANIFEST_FILE_NAME,
                message_store::MESSAGE_STORE_META_FILE_NAME,
                message_store::MESSAGE_STORE_INDEX_FILE_NAME,
            ] {
                update_conversation_cache_signature_for_file(
                    &mut conversations,
                    &entry_path.join(shard_file_name),
                    format!("{file_name}/{shard_file_name}"),
                );
            }
            let blocks_dir = entry_path.join(message_store::MESSAGE_STORE_BLOCKS_DIR_NAME);
            if let Ok(block_entries) = fs::read_dir(blocks_dir) {
                for block_entry in block_entries.flatten() {
                    let block_path = block_entry.path();
                    if !block_path.is_file() {
                        continue;
                    }
                    let block_file_name = block_entry.file_name().to_string_lossy().to_string();
                    update_conversation_cache_signature_for_file(
                        &mut conversations,
                        &block_path,
                        format!("{file_name}/{}/{}", message_store::MESSAGE_STORE_BLOCKS_DIR_NAME, block_file_name),
                    );
                }
            }
        }
    }

    AppDataCacheSignature {
        agents_len,
        agents_modified,
        runtime_len,
        runtime_modified,
        conversations,
    }
}

pub(crate) fn write_json_file_atomic<T>(path: &PathBuf, value: &T, label: &str) -> Result<(), String>
where
    T: Serialize,
{
    ensure_parent_dir(path)?;
    let body = serde_json::to_vec_pretty(value).map_err(|err| format!("Serialize {label} failed: {err}"))?;
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| format!("Invalid {label} file path"))?;
    let tmp = path.with_file_name(format!("{file_name}.tmp"));
    fs::write(&tmp, body).map_err(|err| format!("Write temp {label} failed: {err}"))?;
    if let Err(rename_err) = fs::rename(&tmp, path) {
        fs::copy(&tmp, path).map_err(|copy_err| {
            format!(
                "Finalize {label} failed (rename: {rename_err}; copy: {copy_err})"
            )
        })?;
        let _ = fs::remove_file(&tmp);
    }
    Ok(())
}

pub(crate) fn write_json_file_atomic_if_changed<T>(
    path: &PathBuf,
    value: &T,
    label: &str,
) -> Result<bool, String>
where
    T: Serialize,
{
    ensure_parent_dir(path)?;
    let body = serde_json::to_vec_pretty(value).map_err(|err| format!("Serialize {label} failed: {err}"))?;
    if let Ok(existing) = fs::read(path) {
        if existing == body {
            return Ok(false);
        }
    }
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .ok_or_else(|| format!("Invalid {label} file path"))?;
    let tmp = path.with_file_name(format!("{file_name}.tmp"));
    fs::write(&tmp, body).map_err(|err| format!("Write temp {label} failed: {err}"))?;
    if let Err(rename_err) = fs::rename(&tmp, path) {
        fs::copy(&tmp, path).map_err(|copy_err| {
            format!(
                "Finalize {label} failed (rename: {rename_err}; copy: {copy_err})"
            )
        })?;
        let _ = fs::remove_file(&tmp);
    }
    Ok(true)
}

pub(crate) fn read_layout_app_data(path: &PathBuf) -> Result<AppData, String> {
    let mut agents = if app_layout_agents_path(path).exists() {
        read_json_file::<AgentsFile>(&app_layout_agents_path(path), "agents file")?.agents
    } else {
        AppData::default().agents
    };
    ensure_required_builtin_agents_in_list(&mut agents);

    let runtime = if app_layout_runtime_state_path(path).exists() {
        read_json_file::<RuntimeStateFile>(&app_layout_runtime_state_path(path), "runtime state file")?
    } else {
        RuntimeStateFile::default()
    };

    let mut conversations = Vec::<Conversation>::new();
    let conv_dir = pai_backend::message_store::paths::app_layout_chat_conversations_dir(path);
    if conv_dir.exists() {
        if let Ok(entries) = fs::read_dir(&conv_dir) {
            let mut seen_ids = std::collections::HashSet::<String>::new();
            for entry in entries.flatten() {
                let p = entry.path();
                let conversation_id = if p.extension().and_then(|v| v.to_str()) == Some("json") {
                    p.file_stem()
                        .and_then(|v| v.to_str())
                        .unwrap_or_default()
                        .to_string()
                } else if p.is_dir() {
                    p.file_name()
                        .and_then(|v| v.to_str())
                        .unwrap_or_default()
                        .to_string()
                } else {
                    continue;
                };
                if conversation_id.trim().is_empty() || !seen_ids.insert(conversation_id.clone()) {
                    continue;
                }
                if let Ok(conv) = read_conversation_shard_raw(path, &conversation_id) {
                    conversations.push(conv);
                }
            }
        }
    }

    Ok(AppData {
        version: runtime.version,
        data_migration_version: runtime.data_migration_version,
        message_store_migration_version: runtime.message_store_migration_version,
        agents,
        assistant_department_agent_id: runtime.assistant_department_agent_id,
        user_alias: default_user_alias(),
        response_style_id: runtime.response_style_id,
        pdf_read_mode: runtime.pdf_read_mode,
        background_voice_screenshot_keywords: runtime.background_voice_screenshot_keywords,
        background_voice_screenshot_mode: runtime.background_voice_screenshot_mode,
        instruction_presets: runtime.instruction_presets,
        main_conversation_id: runtime.main_conversation_id,
        pinned_conversation_ids: runtime.pinned_conversation_ids,
        conversation_section_orders: runtime.conversation_section_orders,
        conversations,
        image_text_cache: runtime.image_text_cache,
        remote_im_contacts: runtime.remote_im_contacts,
        remote_im_contact_checkpoints: runtime.remote_im_contact_checkpoints,
        pdf_text_cache: runtime.pdf_text_cache,
        pdf_image_cache: runtime.pdf_image_cache,
        archived_conversations: Vec::new(),
    })
}

// ========== 数据迁移 registry ==========
//
// v2+ 需要显式上下文，避免在只有 app_data 路径时猜测助理空间位置或名称。
pub(crate) struct DataMigrationContext<'a> {
    pub(crate) state: &'a AppState,
    pub(crate) config: &'a AppConfig,
}

#[derive(Debug, Default, Clone, Copy)]
pub(crate) struct DataMigrationStepStats {
    pub(crate) data_changed: bool,
    pub(crate) conversation_writes: usize,
}

pub(crate) struct DataMigrationStep {
    pub(crate) version: u32,
    pub(crate) name: &'static str,
    pub(crate) run: for<'a> fn(&DataMigrationContext<'a>) -> Result<DataMigrationStepStats, String>,
}

pub(crate) fn data_migration_steps() -> Vec<DataMigrationStep> {
    vec![DataMigrationStep {
        version: DATA_MIGRATION_VERSION_V2_ASSISTANT_WORKSPACE_FOR_EMPTY_SHELL_WORKSPACES,
        name: "v2_assistant_workspace_for_empty_shell_workspaces",
        run: migrate_empty_shell_workspaces_to_assistant_workspace,
    }]
}

pub(crate) fn conversation_shell_workspace_path_key(path: &str) -> String {
    let path = path.trim();
    if path.is_empty() {
        String::new()
    } else {
        normalize_terminal_path_for_compare(&PathBuf::from(path))
    }
}

pub(crate) fn legacy_shell_workspace_path_as_main_workspace(
    state: &AppState,
    path: &str,
) -> Option<ShellWorkspaceConfig> {
    let raw = ShellWorkspaceConfig {
        id: "legacy-main-workspace".to_string(),
        name: String::new(),
        path: path.trim().to_string(),
        level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(),
        access: SHELL_WORKSPACE_ACCESS_APPROVAL.to_string(),
        built_in: false,
    };
    let candidate = shell_workspace_resolve_path_candidate(state, &raw)?;
    let canonical = candidate.canonicalize().ok()?;
    if !canonical.is_dir() {
        return None;
    }
    normalize_conversation_shell_workspaces(
        state,
        &[ShellWorkspaceConfig {
            path: terminal_path_for_user(&canonical),
            ..raw
        }],
    )
    .into_iter()
    .next()
}

pub(crate) fn state_write_conversation_shell_workspace_metadata_direct(
    state: &AppState,
    conversation_id: &str,
    shell_workspaces: Vec<ShellWorkspaceConfig>,
) -> Result<bool, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Ok(false);
    }
    let mutation_gate = conversation_mutation_gate(&state.data_path, conversation_id)?;
    let _guard = mutation_gate.lock().map_err(|err| {
        named_lock_error("conversation_mutation_gate", file!(), line!(), module_path!(), &err)
    })?;
    let conversation_meta = state_read_conversation_metadata_cached(state, conversation_id)?;
    let mut conversation =
        conversation_service_v2().build_conversation_snapshot_from_meta(&conversation_meta, Vec::new());
    let original_path = conversation.shell_workspace_path.clone();
    let original_workspaces = conversation.shell_workspaces.clone();
    conversation.shell_workspace_path = None;
    conversation.shell_workspaces = shell_workspaces;
    if conversation.shell_workspace_path == original_path
        && conversation.shell_workspaces == original_workspaces
    {
        return Ok(false);
    }
    let mut updated_meta = message_store::ConversationShardMeta::from_conversation(&conversation);
    updated_meta.preserve_message_derived_fields_from(&conversation_meta);
    write_conversation_meta_shard_from_meta(&state.data_path, &updated_meta)?;
    let _ = state_mark_conversation_metadata_direct_persisted(state, conversation_id)?;
    Ok(true)
}

pub(crate) fn shell_workspaces_for_empty_conversation_workspace_migration(
    state: &AppState,
    config: &AppConfig,
    conversation: &Conversation,
) -> Option<Vec<ShellWorkspaceConfig>> {
    let normalized = normalize_conversation_shell_workspaces(state, &conversation.shell_workspaces);
    if !normalized.is_empty() {
        return None;
    }
    if let Some(legacy_workspace) = conversation
        .shell_workspace_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|path| legacy_shell_workspace_path_as_main_workspace(state, path))
    {
        return Some(vec![legacy_workspace]);
    }
    Some(vec![assistant_workspace_as_conversation_main_workspace(
        state, config,
    )])
}

pub(crate) fn migrate_empty_shell_workspaces_to_assistant_workspace(
    context: &DataMigrationContext<'_>,
) -> Result<DataMigrationStepStats, String> {
    let chat_index = collect_chat_index_items_from_storage(&context.state.data_path)?;
    let mut stats = DataMigrationStepStats::default();
    for item in chat_index {
        let conversation_id = item.id.trim();
        if conversation_id.is_empty() {
            continue;
        }
        let conversation_meta =
            match state_read_conversation_metadata_cached(context.state, conversation_id) {
                Ok(meta) => meta,
                Err(err) => {
                    runtime_log_warn(format!(
                        "[应用数据迁移] 跳过，任务=v2补齐会话工作区，conversation_id={}，error={}",
                        conversation_id, err
                    ));
                    continue;
                }
            };
        let conversation = conversation_service_v2()
            .build_conversation_snapshot_from_meta(&conversation_meta, Vec::new());
        if !conversation_visible_in_foreground_lists(&conversation)
            || !conversation_is_local_normal_chat(&conversation)
            || !conversation_is_unarchived(&conversation)
        {
            continue;
        }
        let Some(shell_workspaces) = shell_workspaces_for_empty_conversation_workspace_migration(
            context.state,
            context.config,
            &conversation,
        ) else {
            continue;
        };
        if state_write_conversation_shell_workspace_metadata_direct(
            context.state,
            conversation_id,
            shell_workspaces,
        )? {
            stats.data_changed = true;
            stats.conversation_writes += 1;
        }
    }
    Ok(stats)
}

pub(crate) fn run_app_data_migrations_with_state(
    state: &AppState,
    config: &AppConfig,
) -> Result<bool, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    if runtime.data_migration_version >= DATA_MIGRATION_CURRENT_VERSION {
        return Ok(false);
    }
    let migration_version_before = runtime.data_migration_version;
    let mut any_data_changed = false;
    for step in data_migration_steps() {
        if runtime.data_migration_version >= step.version {
            continue;
        }
        let started = std::time::Instant::now();
        let stats = (step.run)(&DataMigrationContext { state, config })?;
        runtime.data_migration_version = step.version;
        any_data_changed |= stats.data_changed;
        runtime_log_info(format!(
            "[应用数据迁移] 完成，任务={}，migration_version_before={}，migration_version_after={}，data_changed={}，conversation_writes={}，duration_ms={}",
            step.name,
            migration_version_before,
            runtime.data_migration_version,
            stats.data_changed,
            stats.conversation_writes,
            started.elapsed().as_millis()
        ));
    }
    if runtime.data_migration_version < DATA_MIGRATION_CURRENT_VERSION {
        runtime.data_migration_version = DATA_MIGRATION_CURRENT_VERSION;
    }
    state_write_runtime_state_cached(state, &runtime)?;
    Ok(any_data_changed || migration_version_before != runtime.data_migration_version)
}

pub(crate) fn assistant_workspace_label_sync_target_keys(
    state: &AppState,
    previous_config: &AppConfig,
    next_config: &AppConfig,
) -> std::collections::HashSet<String> {
    let mut previous = previous_config.clone();
    let mut next = next_config.clone();
    let _ = ensure_default_shell_workspace_in_config(&mut previous, state);
    let _ = ensure_default_shell_workspace_in_config(&mut next, state);
    [
        assistant_workspace_as_conversation_main_workspace(state, &previous),
        assistant_workspace_as_conversation_main_workspace(state, &next),
    ]
    .into_iter()
    .map(|workspace| conversation_shell_workspace_path_key(&workspace.path))
    .filter(|key| !key.is_empty())
    .collect()
}

pub(crate) fn sync_assistant_workspace_label_for_unarchived_conversations(
    state: &AppState,
    previous_config: &AppConfig,
    next_config: &AppConfig,
) -> Result<usize, String> {
    let target_keys =
        assistant_workspace_label_sync_target_keys(state, previous_config, next_config);
    if target_keys.is_empty() {
        return Ok(0);
    }
    let assistant_workspace =
        assistant_workspace_as_conversation_main_workspace(state, next_config);
    let chat_index = collect_chat_index_items_from_storage(&state.data_path)?;
    let mut changed = 0usize;
    for item in chat_index {
        let conversation_id = item.id.trim();
        if conversation_id.is_empty() {
            continue;
        }
        let conversation_meta = match state_read_conversation_metadata_cached(state, conversation_id)
        {
            Ok(meta) => meta,
            Err(err) => {
                runtime_log_warn(format!(
                    "[终端工作空间] 跳过，任务=同步助理空间会话标签，conversation_id={}，error={}",
                    conversation_id, err
                ));
                continue;
            }
        };
        let conversation = conversation_service_v2()
            .build_conversation_snapshot_from_meta(&conversation_meta, Vec::new());
        if !conversation_visible_in_foreground_lists(&conversation)
            || !conversation_is_local_normal_chat(&conversation)
            || !conversation_is_unarchived(&conversation)
        {
            continue;
        }
        let mut workspaces =
            normalize_conversation_shell_workspaces(state, &conversation.shell_workspaces);
        if workspaces.is_empty() {
            workspaces = vec![assistant_workspace.clone()];
        } else {
            let main_index = workspaces
                .iter()
                .position(|workspace| {
                    normalize_shell_workspace_level_text(&workspace.level)
                        == SHELL_WORKSPACE_LEVEL_MAIN
                })
                .unwrap_or(0);
            let key = conversation_shell_workspace_path_key(&workspaces[main_index].path);
            if !target_keys.contains(&key) {
                continue;
            }
            let mut synced = workspaces[main_index].clone();
            synced.name = assistant_workspace.name.clone();
            synced.path = assistant_workspace.path.clone();
            synced.level = SHELL_WORKSPACE_LEVEL_MAIN.to_string();
            synced.built_in = false;
            if normalize_shell_workspace_access_text(&synced.access).is_empty() {
                synced.access = assistant_workspace.access.clone();
            }
            workspaces[main_index] = synced;
        }
        if state_write_conversation_shell_workspace_metadata_direct(
            state,
            conversation_id,
            workspaces,
        )? {
            changed += 1;
        }
    }
    if changed > 0 {
        runtime_log_info(format!(
            "[终端工作空间] 完成，任务=同步助理空间会话标签，conversation_count={}",
            changed
        ));
    }
    Ok(changed)
}

pub(crate) fn read_app_data(path: &PathBuf) -> Result<AppData, String> {
    let mut parsed = read_layout_app_data(path)?;
    parsed.version = APP_DATA_SCHEMA_VERSION;
    let migration_version_before = parsed.data_migration_version;
    let run_v1_baseline_migrations =
        migration_version_before < DATA_MIGRATION_VERSION_V1_BASELINE;
    let builtin_agents_filled = ensure_required_builtin_agents(&mut parsed);
    let conversation_metadata_filled = if run_v1_baseline_migrations {
        fill_missing_conversation_metadata(&mut parsed)
    } else {
        false
    };
    let avatar_paths_migrated = if run_v1_baseline_migrations {
        migrate_agent_avatar_paths(path, &mut parsed)
    } else {
        false
    };
    let merged_archives = if run_v1_baseline_migrations {
        migrate_app_data_archives_into_conversations(path, &mut parsed)?
    } else {
        false
    };
    let migrated = if run_v1_baseline_migrations {
        migrate_app_data_inline_media_to_refs(path, &mut parsed)
    } else {
        false
    };
    let main_conversation_marker_changed = if run_v1_baseline_migrations {
        normalize_main_conversation_marker(&mut parsed, "")
    } else {
        false
    };
    let mut tool_review_legacy_cleaned = false;
    if run_v1_baseline_migrations {
        for conversation in parsed.conversations.iter_mut() {
            if tool_review_cleanup_legacy_artifacts(path, conversation)? {
                tool_review_legacy_cleaned = true;
            }
        }
    }
    let data_migration_version_recorded = if parsed.data_migration_version < DATA_MIGRATION_VERSION_V1_BASELINE {
        parsed.data_migration_version = DATA_MIGRATION_VERSION_V1_BASELINE;
        true
    } else {
        false
    };
    if conversation_metadata_filled
        || builtin_agents_filled
        || avatar_paths_migrated
        || merged_archives
        || migrated
        || tool_review_legacy_cleaned
        || main_conversation_marker_changed
        || !app_layout_exists(path)
    {
        #[allow(deprecated)]
        let started = std::time::Instant::now();
        let stats = write_app_data_with_stats(path, &parsed)?;
        runtime_log_debug(format!(
            "[应用数据读入迁移] 完成，任务=读入后兼容写回，触发条件=read_app_data，migration_version_before={}，migration_version_after={}，run_v1_baseline_migrations={}，data_migration_version_recorded={}，builtin_agents_filled={}，conversation_metadata_filled={}，avatar_paths_migrated={}，merged_archives={}，inline_media_migrated={}，tool_review_legacy_cleaned={}，main_conversation_marker_changed={}，layout_missing={}，agents_written={}，runtime_written={}，conversation_writes={}，conversation_deletes={}，duration_ms={}",
            migration_version_before,
            parsed.data_migration_version,
            run_v1_baseline_migrations,
            data_migration_version_recorded,
            builtin_agents_filled,
            conversation_metadata_filled,
            avatar_paths_migrated,
            merged_archives,
            migrated,
            tool_review_legacy_cleaned,
            main_conversation_marker_changed,
            !app_layout_exists(path),
            stats.agents_written,
            stats.runtime_written,
            stats.conversation_writes,
            stats.conversation_deletes,
            started.elapsed().as_millis()
        ));
    } else if data_migration_version_recorded {
        let mut runtime = build_runtime_state_file(&parsed);
        if app_layout_runtime_state_path(path).exists() {
            if let Ok(existing_runtime) =
                read_json_file::<RuntimeStateFile>(&app_layout_runtime_state_path(path), "runtime state file")
            {
                runtime.data_migration_version = runtime
                    .data_migration_version
                    .max(existing_runtime.data_migration_version);
                runtime.message_store_migration_version = runtime
                    .message_store_migration_version
                    .max(existing_runtime.message_store_migration_version);
            }
        }
        let runtime_written = write_runtime_state_shard(path, &runtime)?;
        runtime_log_debug(format!(
            "[应用数据读入迁移] 完成，任务=记录迁移版本，触发条件=read_app_data，migration_version_before={}，migration_version_after={}，run_v1_baseline_migrations={}，runtime_written={}，conversation_writes=0",
            migration_version_before,
            parsed.data_migration_version,
            run_v1_baseline_migrations,
            runtime_written
        ));
    }
    Ok(parsed)
}

pub(crate) fn normalize_conversation_runtime_volatile_fields(conversation: &mut Conversation) {
    let _ = fill_missing_conversation_message_speaker_agent_ids(conversation);
    let _ = cleanup_legacy_summary_context_messages(conversation);
}

// AppData 聚合写入需要保留，作为兼容/迁移/全量导入导出入口。
// 但业务热路径禁止直接依赖它，应该优先走分片写入：
// agents / runtime_state / conversation:<id>
pub(crate) fn write_app_data_with_stats(path: &PathBuf, data: &AppData) -> Result<AppDataWriteStats, String> {
    let agents = build_agents_file(&data.agents);
    let mut runtime = build_runtime_state_file(data);
    if app_layout_runtime_state_path(path).exists() {
        if let Ok(existing_runtime) =
            read_json_file::<RuntimeStateFile>(&app_layout_runtime_state_path(path), "runtime state file")
        {
            runtime.data_migration_version = runtime
                .data_migration_version
                .max(existing_runtime.data_migration_version);
            runtime.message_store_migration_version = runtime
                .message_store_migration_version
                .max(existing_runtime.message_store_migration_version);
        }
    }

    fs::create_dir_all(app_layout_config_dir(path))
        .map_err(|err| format!("Create config layout dir failed: {err}"))?;
    fs::create_dir_all(app_layout_state_dir(path))
        .map_err(|err| format!("Create state layout dir failed: {err}"))?;
    fs::create_dir_all(pai_backend::message_store::paths::app_layout_chat_dir(path))
        .map_err(|err| format!("Create chat layout dir failed: {err}"))?;
    fs::create_dir_all(pai_backend::message_store::paths::app_layout_chat_conversations_dir(path))
        .map_err(|err| format!("Create chat conversations dir failed: {err}"))?;
    fs::create_dir_all(app_layout_backups_dir(path))
        .map_err(|err| format!("Create backups dir failed: {err}"))?;

    let mut stats = AppDataWriteStats::default();

    stats.agents_written = write_json_file_atomic_if_changed(
        &app_layout_agents_path(path),
        &agents,
        "agents file",
    )?;
    stats.runtime_written = write_json_file_atomic_if_changed(
        &app_layout_runtime_state_path(path),
        &runtime,
        "runtime state file",
    )?;
    let mut expected_ids = std::collections::HashSet::<String>::new();
    let mut system_notification_in_input = false;
    for conv in &data.conversations {
        let mut conversation = conv.clone();
        if conversation.id.trim() == SYSTEM_NOTIFICATION_CONVERSATION_ID {
            system_notification_in_input = true;
            let _ = normalize_system_notification_conversation(&mut conversation);
        }
        expected_ids.insert(conversation.id.clone());
        if write_conversation_shard(path, &conversation)? {
            stats.conversation_writes += 1;
        }
    }
    expected_ids.insert(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    if !system_notification_in_input && ensure_system_notification_conversation_shard(path)? {
        stats.conversation_writes += 1;
    }
    if let Ok(entries) = fs::read_dir(pai_backend::message_store::paths::app_layout_chat_conversations_dir(path)) {
        for entry in entries.flatten() {
            let p = entry.path();
            let shard_id = if p.extension().and_then(|v| v.to_str()) == Some("json") {
                p.file_stem()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default()
                    .to_string()
            } else if p.is_dir() {
                p.file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or_default()
                    .to_string()
            } else {
                continue;
            };
            if !expected_ids.contains(&shard_id) {
                match delete_conversation_shard(path, &shard_id) {
                    Ok(true) => {
                        stats.conversation_deletes += 1;
                    }
                    Ok(false) => {}
                    Err(err) => runtime_log_warn(format!(
                        "[应用数据写入] 状态=失败，任务=清理孤立会话分片，conversation_id={}，path={}，error={}",
                        shard_id,
                        p.display(),
                        err
                    )),
                }
            }
        }
    }
    Ok(stats)
}

/// Compatibility-only full AppData writer.
///
/// New production code must not add new call sites to this function. Prefer shard APIs:
/// `write_agents_shard`, `write_runtime_state_shard`, `write_conversation_shard`,
/// and their cached state wrappers.
///
/// Migration timeline:
/// - New code: forbidden immediately.
/// - Existing compatibility / migration / import-export flows: temporarily allowed.
/// - After compatibility-only callers are fully isolated, reevaluate final removal.
#[deprecated(
    note = "兼容层专用的全量 AppData 写入器；新代码禁止调用，请改用 agents/runtime_state/chat_index/conversation 分片写入 API。"
)]
pub(crate) fn write_app_data(path: &PathBuf, data: &AppData) -> Result<(), String> {
    let started = std::time::Instant::now();
    let stats = write_app_data_with_stats(path, data)?;
    runtime_log_debug(format!(
        "[应用数据写入] 任务=应用数据写入，状态=完成，触发=兼容层全量写入，agents_written={}，runtime_written={}，conversation_writes={}，conversation_deletes={}，duration_ms={}",
        stats.agents_written,
        stats.runtime_written,
        stats.conversation_writes,
        stats.conversation_deletes,
        started.elapsed().as_millis()
    ));
    Ok(())
}

#[cfg(test)]
pub(crate) mod conversation_section_orders_runtime_tests {

    #[test]
    fn build_runtime_state_file_should_preserve_conversation_section_orders() {
        let mut data = AppData::default();
        data.conversation_section_orders = ConversationSectionOrders {
            local: vec!["pinned".to_string(), "workspace:alpha".to_string()],
            contact: vec!["recent".to_string(), "channel:demo".to_string()],
        };

        let runtime = build_runtime_state_file(&data);

        assert_eq!(runtime.conversation_section_orders, data.conversation_section_orders);
    }

    #[test]
    fn apply_runtime_state_to_app_data_should_restore_conversation_section_orders() {
        let mut data = AppData::default();
        let runtime = RuntimeStateFile {
            conversation_section_orders: ConversationSectionOrders {
                local: vec!["workspace:beta".to_string()],
                contact: vec!["channel:test".to_string()],
            },
            ..RuntimeStateFile::default()
        };

        apply_runtime_state_to_app_data(&mut data, &runtime);

        assert_eq!(data.conversation_section_orders, runtime.conversation_section_orders);
    }
}
