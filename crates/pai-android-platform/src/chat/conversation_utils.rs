use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use pai_backend::archive_host_selector::{available_non_user_agent, first_available_department_agent};
use pai_backend::core::domain::constants::{
    ASSISTANT_DEPARTMENT_ID, CONVERSATION_KIND_CHAT, CONVERSATION_KIND_DELEGATE, CONVERSATION_KIND_REMOTE_IM_CONTACT,
    CONVERSATION_KIND_SIDE_CHAT, CONVERSATION_KIND_SYSTEM_NOTIFICATION, DEFAULT_AGENT_ID,
    SYSTEM_NOTIFICATION_CONVERSATION_ID, SYSTEM_PERSONA_ID, USER_PERSONA_ID,
};
use pai_backend::core::domain::runtime_types::PreparedBinaryPayload;
use pai_backend::core::domain::types_chat::{
    AgentProfile, ChatMessage, Conversation, ConversationCumulativeUsage, MessagePart,
};
use pai_backend::core::domain::types_config::{
    default_shell_work_mode, ApiConfig, DepartmentConfig,
};
use pai_backend::core::domain::types_requests::{estimated_tokens_for_text, ChatInputPayload};
use pai_backend::core::domain::types_storage::AppData;
use pai_backend::core::time_semantics::now_iso;
use pai_backend::core_provider_utils::truncate_by_chars;
use pai_backend::image_normalizer::{normalize_image_bytes_for_llm_request, LlmRequestNormalizedImage};
use pai_backend::logging::{runtime_log_info, runtime_log_warn};
use pai_backend::memory::store::types::clean_text;
use pai_backend::screenshot_cache::clear_screenshot_artifact_cache;
use serde_json::Value;
use std::path::PathBuf;
use uuid::Uuid;

fn message_attachment_kind(mime: &str) -> &'static str {
    let normalized = mime.trim().to_ascii_lowercase();
    if normalized.starts_with("image/") {
        "image"
    } else if normalized.starts_with("audio/") {
        "audio"
    } else if normalized == "application/pdf" {
        "pdf"
    } else {
        "file"
    }
}


pub fn latest_active_conversation_index(
    data: &AppData,
    _api_config_id: &str,
    _agent_id: &str,
) -> Option<usize> {
    data.conversations
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            conversation_is_unarchived(c) && conversation_visible_in_foreground_lists(c)
        })
        .max_by(|(idx_a, a), (idx_b, b)| {
            let a_updated = a.updated_at.trim();
            let b_updated = b.updated_at.trim();
            let a_created = a.created_at.trim();
            let b_created = b.created_at.trim();
            a_updated
                .cmp(b_updated)
                .then_with(|| a_created.cmp(b_created))
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

pub fn latest_main_conversation_index(data: &AppData, _agent_id: &str) -> Option<usize> {
    data.conversations
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            conversation_is_unarchived(c) && conversation_visible_in_foreground_lists(c)
        })
        .max_by(|(idx_a, a), (idx_b, b)| {
            let a_updated = a.updated_at.trim();
            let b_updated = b.updated_at.trim();
            let a_created = a.created_at.trim();
            let b_created = b.created_at.trim();
            a_updated
                .cmp(b_updated)
                .then_with(|| a_created.cmp(b_created))
                .then_with(|| idx_a.cmp(idx_b))
        })
        .map(|(idx, _)| idx)
}

pub fn system_notification_conversation_title() -> String {
    "P-ai系统".to_string()
}

pub fn normalize_system_notification_conversation(conversation: &mut Conversation) -> bool {
    let mut changed = false;
    if conversation.id.trim() != SYSTEM_NOTIFICATION_CONVERSATION_ID {
        conversation.id = SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string();
        changed = true;
    }
    if conversation.conversation_kind.trim() != CONVERSATION_KIND_SYSTEM_NOTIFICATION {
        conversation.conversation_kind = CONVERSATION_KIND_SYSTEM_NOTIFICATION.to_string();
        changed = true;
    }
    let expected_title = system_notification_conversation_title();
    if conversation.title.trim() != expected_title {
        conversation.title = expected_title;
        changed = true;
    }
    if conversation.department_id.trim().is_empty() {
        conversation.department_id = ASSISTANT_DEPARTMENT_ID.to_string();
        changed = true;
    }
    if conversation.status.trim().is_empty() {
        conversation.status = "active".to_string();
        changed = true;
    }
    changed
}

pub fn conversation_is_system_notification(conversation: &Conversation) -> bool {
    conversation.id.trim() == SYSTEM_NOTIFICATION_CONVERSATION_ID
        || conversation.conversation_kind.trim() == CONVERSATION_KIND_SYSTEM_NOTIFICATION
}

// available_non_user_agent / first_available_department_agent 已迁至
// crates/pai-backend archive_host_selector（阶段 4），通过 crate 根重导出生效。

pub fn resolve_conversation_bound_agent<'a>(
    conversation: &Conversation,
    agents: &'a [AgentProfile],
    departments: &[DepartmentConfig],
) -> Result<&'a AgentProfile, String> {
    let conversation_id = conversation.id.trim();
    let department_id = conversation.department_id.trim();
    let bound_department = if department_id.is_empty() {
        None
    } else {
        Some(
            departments
                .iter()
                .find(|department| department.id.trim() == department_id)
                .ok_or_else(|| {
                    format!(
                        "会话绑定部门不存在: conversation_id={}, department_id={}",
                        conversation_id, department_id
                    )
                })?,
        )
    };
    let bound_agent_id = conversation.agent_id.trim();
    if !bound_agent_id.is_empty() {
        if let Some(agent) = available_non_user_agent(agents, bound_agent_id) {
            return Ok(agent);
        }
        return Err(format!(
            "会话绑定人格不存在或不可用: conversation_id={}, agent_id={}",
            conversation_id, bound_agent_id
        ));
    }

    if let Some(department) = bound_department {
        return first_available_department_agent(department, agents).ok_or_else(|| {
            format!(
                "会话绑定部门没有可用人格: conversation_id={}, department_id={}",
                conversation_id, department_id
            )
        });
    }
    Err(format!(
        "会话缺少有效人格绑定: conversation_id={}, department_id={}",
        conversation_id, department_id
    ))
}

pub fn main_conversation_index(data: &AppData, _agent_id: &str) -> Option<usize> {
    let target_id = data
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())?;
    data.conversations.iter().position(|conversation| {
        conversation.id == target_id
            && conversation_is_unarchived(conversation)
            && conversation_visible_in_foreground_lists(conversation)
    })
}

pub fn normalize_main_conversation_marker(data: &mut AppData, _agent_id: &str) -> bool {
    let fixed_id = SYSTEM_NOTIFICATION_CONVERSATION_ID;
    if let Some(idx) = data.conversations.iter().position(|conversation| {
        conversation.id.trim() == fixed_id
            && conversation_is_unarchived(conversation)
            && conversation_visible_in_foreground_lists(conversation)
    }) {
        let mut changed = normalize_system_notification_conversation(&mut data.conversations[idx]);
        if data.main_conversation_id.as_deref().map(str::trim) != Some(fixed_id) {
            data.main_conversation_id = Some(fixed_id.to_string());
            changed = true;
        }
        return changed;
    }
    if let Some(idx) = data.conversations.iter().position(|conversation| {
        conversation_is_unarchived(conversation)
            && conversation_visible_in_foreground_lists(conversation)
            && conversation_is_system_notification(conversation)
    }) {
        let mut changed = normalize_system_notification_conversation(&mut data.conversations[idx]);
        if data.main_conversation_id.as_deref().map(str::trim) != Some(fixed_id) {
            data.main_conversation_id = Some(fixed_id.to_string());
            changed = true;
        }
        return changed;
    }
    data.conversations.push(build_system_notification_conversation_record());
    data.main_conversation_id = Some(fixed_id.to_string());
    true
}

pub fn normalize_single_active_main_conversation(data: &mut AppData) -> bool {
    let Some(keep_idx) = latest_active_conversation_index(data, "", "")
        .or_else(|| latest_main_conversation_index(data, ""))
    else {
        return false;
    };

    let mut changed = false;
    for (_idx, conversation) in data.conversations.iter_mut().enumerate() {
        if !conversation_visible_in_foreground_lists(conversation) || conversation_is_archived(conversation) {
            continue;
        }
        let target_status = "active";
        if conversation.status.trim() != target_status {
            conversation.status = target_status.to_string();
            changed = true;
        }
    }
    if changed {
        let keep_id = data
            .conversations
            .get(keep_idx)
            .map(|item| item.id.clone())
            .unwrap_or_default();
        runtime_log_info(format!(
            "[会话] 归一化未归档会话激活标记: active_conversation_id={}",
            keep_id
        ));
    }
    changed
}

pub fn conversation_is_delegate(conversation: &Conversation) -> bool {
    conversation.conversation_kind.trim() == CONVERSATION_KIND_DELEGATE
}

pub fn conversation_is_remote_im_contact(conversation: &Conversation) -> bool {
    conversation.conversation_kind.trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
}

pub fn conversation_is_side_chat(conversation: &Conversation) -> bool {
    conversation.conversation_kind.trim() == CONVERSATION_KIND_SIDE_CHAT
}

pub fn increment_conversation_unread_count(conversation: &mut Conversation, count: usize) {
    if count == 0 || conversation_is_remote_im_contact(conversation) {
        return;
    }
    conversation.unread_count = conversation.unread_count.saturating_add(count);
}

pub fn clear_conversation_unread_count(conversation: &mut Conversation) -> bool {
    if conversation.unread_count == 0 {
        return false;
    }
    conversation.unread_count = 0;
    true
}

pub fn conversation_visible_in_foreground_lists(conversation: &Conversation) -> bool {
    // side_chat 仍由普通 Conversation runtime 处理，但只挂在父会话的追问视图中。
    !conversation_is_delegate(conversation)
        && !conversation_is_remote_im_contact(conversation)
        && !conversation_is_side_chat(conversation)
}

pub fn conversation_is_unarchived(conversation: &Conversation) -> bool {
    !conversation_is_archived(conversation)
}

pub fn conversation_is_archived(conversation: &Conversation) -> bool {
    if conversation.status.trim() == "archived" {
        return true;
    }
    conversation
        .archived_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
}

pub const SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION: u64 = 2;
pub const SUMMARY_CONTEXT_TITLE_MAX_CHARS: usize = 20;
pub const SUMMARY_CONTEXT_TITLE_SOURCE_BRANCH: &str = "branch_source";

pub fn conversation_is_local_normal_chat(conversation: &Conversation) -> bool {
    matches!(
        conversation.conversation_kind.trim(),
        CONVERSATION_KIND_CHAT | CONVERSATION_KIND_SIDE_CHAT
    )
        && !conversation_is_system_notification(conversation)
        && !conversation_is_delegate(conversation)
        && !conversation_is_remote_im_contact(conversation)
}

pub fn summary_context_message_kind(message: &ChatMessage) -> Option<&str> {
    let meta = message.provider_meta.as_ref()?;
    meta.get("message_meta")
        .and_then(|value| value.get("kind"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            meta.get("messageMeta")
                .and_then(|value| value.get("kind"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .or_else(|| {
            meta.get("messageKind")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

pub fn is_summary_context_message_kind(kind: &str) -> bool {
    matches!(kind.trim(), "context_compaction" | "summary_context_seed")
}

pub fn normalize_summary_context_title(raw: &str) -> Option<String> {
    let first_line = raw
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default();
    let stripped = first_line
        .trim_matches(|ch| {
            matches!(
                ch,
                '"' | '\''
                    | '`'
                    | '“'
                    | '”'
                    | '‘'
                    | '’'
                    | '《'
                    | '》'
                    | '「'
                    | '」'
                    | '『'
                    | '』'
            )
        })
        .trim_matches(|ch| matches!(ch, '。' | '！' | '？' | '!' | '?' | '，' | ',' | '；' | ';' | '：' | ':' | '、'))
        .trim();
    let cleaned = clean_text(stripped);
    if cleaned.is_empty() {
        return None;
    }
    Some(
        cleaned
            .chars()
            .take(SUMMARY_CONTEXT_TITLE_MAX_CHARS)
            .collect::<String>(),
    )
}

pub fn summary_context_message_title(message: &ChatMessage) -> Option<String> {
    let kind = summary_context_message_kind(message)?;
    if !is_summary_context_message_kind(kind) {
        return None;
    }
    let meta = message.provider_meta.as_ref()?;
    meta.get("message_meta")
        .and_then(|value| value.get("title"))
        .and_then(Value::as_str)
        .and_then(normalize_summary_context_title)
        .or_else(|| {
            meta.get("messageMeta")
                .and_then(|value| value.get("title"))
                .and_then(Value::as_str)
                .and_then(normalize_summary_context_title)
        })
}

pub fn summary_context_message_title_source(message: &ChatMessage) -> Option<&str> {
    let kind = summary_context_message_kind(message)?;
    if !is_summary_context_message_kind(kind) {
        return None;
    }
    let meta = message.provider_meta.as_ref()?;
    meta.get("message_meta")
        .and_then(|value| {
            value
                .get("titleSource")
                .or_else(|| value.get("title_source"))
                .or_else(|| value.get("titleProvenance"))
        })
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            meta.get("messageMeta")
                .and_then(|value| {
                    value
                        .get("titleSource")
                        .or_else(|| value.get("title_source"))
                        .or_else(|| value.get("titleProvenance"))
                })
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
}

pub fn summary_context_message_title_blocks_auto_title(message: &ChatMessage) -> bool {
    if summary_context_message_title(message).is_none() {
        return false;
    }
    !matches!(
        summary_context_message_title_source(message),
        Some(SUMMARY_CONTEXT_TITLE_SOURCE_BRANCH)
    )
}

pub fn conversation_has_auto_title_blocking_summary_title(conversation: &Conversation) -> bool {
    conversation
        .messages
        .iter()
        .rev()
        .find_map(|message| {
            summary_context_message_title(message)
                .map(|_| summary_context_message_title_blocks_auto_title(message))
        })
        .unwrap_or(false)
}

pub fn ensure_summary_context_message_meta_object_mut(
    message: &mut ChatMessage,
) -> Option<&mut serde_json::Map<String, Value>> {
    let provider_meta = message
        .provider_meta
        .get_or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !provider_meta.is_object() {
        *provider_meta = Value::Object(serde_json::Map::new());
    }
    let Some(root) = provider_meta.as_object_mut() else {
        return None;
    };
    let message_meta = root
        .entry("message_meta".to_string())
        .or_insert_with(|| Value::Object(serde_json::Map::new()));
    if !message_meta.is_object() {
        *message_meta = Value::Object(serde_json::Map::new());
    }
    message_meta.as_object_mut()
}

pub fn conversation_update_latest_summary_title(
    conversation: &mut Conversation,
    next_title: Option<&str>,
) -> bool {
    conversation_update_latest_summary_title_with_source(conversation, next_title, None)
}

pub fn conversation_update_latest_summary_title_with_source(
    conversation: &mut Conversation,
    next_title: Option<&str>,
    title_source: Option<&str>,
) -> bool {
    let normalized_title = next_title.and_then(normalize_summary_context_title);
    let normalized_source = title_source.map(str::trim).filter(|value| !value.is_empty());
    let Some(message) = conversation
        .messages
        .iter_mut()
        .rev()
        .find(|message| {
            summary_context_message_kind(message)
                .map(is_summary_context_message_kind)
                .unwrap_or(false)
        })
    else {
        return false;
    };
    let Some(message_meta) = ensure_summary_context_message_meta_object_mut(message) else {
        return false;
    };
    let mut changed = false;
    if message_meta.get("schemaVersion").and_then(Value::as_u64)
        != Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
    {
        message_meta.insert(
            "schemaVersion".to_string(),
            Value::Number(serde_json::Number::from(
                SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION,
            )),
        );
        changed = true;
    }
    let previous_title = message_meta
        .get("title")
        .and_then(Value::as_str)
        .and_then(normalize_summary_context_title);
    match normalized_title {
        Some(title) => {
            if previous_title.as_deref() != Some(title.as_str()) {
                message_meta.insert("title".to_string(), Value::String(title));
                changed = true;
            }
            match normalized_source {
                Some(source) => {
                    if message_meta.get("titleSource").and_then(Value::as_str) != Some(source) {
                        message_meta.insert("titleSource".to_string(), Value::String(source.to_string()));
                        changed = true;
                    }
                    if message_meta.remove("title_source").is_some() {
                        changed = true;
                    }
                    if message_meta.remove("titleProvenance").is_some() {
                        changed = true;
                    }
                    if message_meta.remove("titleProvisional").is_some() {
                        changed = true;
                    }
                }
                None => {
                    if message_meta.remove("titleSource").is_some() {
                        changed = true;
                    }
                    if message_meta.remove("title_source").is_some() {
                        changed = true;
                    }
                    if message_meta.remove("titleProvenance").is_some() {
                        changed = true;
                    }
                    if message_meta.remove("titleProvisional").is_some() {
                        changed = true;
                    }
                }
            }
        }
        None => {
            if message_meta.remove("title").is_some() {
                changed = true;
            }
            if message_meta.remove("titleSource").is_some() {
                changed = true;
            }
            if message_meta.remove("title_source").is_some() {
                changed = true;
            }
            if message_meta.remove("titleProvenance").is_some() {
                changed = true;
            }
            if message_meta.remove("titleProvisional").is_some() {
                changed = true;
            }
        }
    }
    changed
}

pub fn conversation_latest_summary_title(conversation: &Conversation) -> Option<String> {
    conversation
        .messages
        .iter()
        .rev()
        .find_map(summary_context_message_title)
}

pub fn cleanup_legacy_summary_context_messages(conversation: &mut Conversation) -> bool {
    let mut changed = false;
    for message in conversation.messages.iter_mut() {
        let Some(kind) = summary_context_message_kind(message) else {
            continue;
        };
        if !is_summary_context_message_kind(kind) {
            continue;
        }
        let Some(message_meta) = ensure_summary_context_message_meta_object_mut(message) else {
            continue;
        };
        let schema_backfilled = message_meta.get("schemaVersion").and_then(Value::as_u64)
            != Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
        ;
        if schema_backfilled {
            message_meta.insert(
                "schemaVersion".to_string(),
                Value::Number(serde_json::Number::from(
                    SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION,
                )),
            );
            changed = true;
        }
        if schema_backfilled {
            message_meta.insert("title".to_string(), Value::String(String::new()));
            changed = true;
        } else if !message_meta.contains_key("title") {
            message_meta.insert("title".to_string(), Value::String(String::new()));
            changed = true;
        }
    }
    changed
}

pub fn conversation_real_user_messages<'a>(conversation: &'a Conversation) -> Vec<&'a ChatMessage> {
    conversation
        .messages
        .iter()
        .filter(|message| {
            message.role.trim().eq_ignore_ascii_case("user")
                && !is_context_compaction_message(message, "user")
                && message
                    .speaker_agent_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    != Some(SYSTEM_PERSONA_ID)
        })
        .collect::<Vec<_>>()
}

pub fn conversation_real_user_message_texts(conversation: &Conversation) -> Vec<String> {
    conversation_real_user_messages(conversation)
        .into_iter()
        .map(render_message_content_for_model)
        .map(|text| clean_text(text.trim()))
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
}

#[cfg(test)]
pub mod summary_context_title_tests {

    use super::*;

    fn test_chat_message(
        id: &str,
        role: &str,
        speaker_agent_id: Option<&str>,
        text: &str,
        provider_meta: Option<Value>,
    ) -> ChatMessage {
        ChatMessage {
            id: id.to_string(),
            role: role.to_string(),
            created_at: "2026-05-06T10:00:00Z".to_string(),
            speaker_agent_id: speaker_agent_id.map(ToOwned::to_owned),
            parts: vec![MessagePart::Text {
                text: text.to_string(),
                reasoning_content: None,
            }],
            extra_text_blocks: Vec::new(),
            provider_meta,
            tool_call: None,
            mcp_call: None,
        meme_annotations: None,
        }
    }

    fn test_summary_meta(kind: &str, title: Option<&str>, schema_version: Option<u64>) -> Value {
        let mut message_meta = serde_json::Map::new();
        message_meta.insert("kind".to_string(), Value::String(kind.to_string()));
        message_meta.insert("scene".to_string(), Value::String("test".to_string()));
        if let Some(title) = title {
            message_meta.insert("title".to_string(), Value::String(title.to_string()));
        }
        if let Some(schema_version) = schema_version {
            message_meta.insert(
                "schemaVersion".to_string(),
                Value::Number(serde_json::Number::from(schema_version)),
            );
        }
        Value::Object(serde_json::Map::from_iter([(
            "message_meta".to_string(),
            Value::Object(message_meta),
        )]))
    }

    fn test_conversation(messages: Vec<ChatMessage>) -> Conversation {
        Conversation {
            id: "conversation-a".to_string(),
            title: String::new(),
            agent_id: "agent-a".to_string(),
            department_id: "dept-a".to_string(),
            bound_conversation_id: None,
            parent_conversation_id: None,
            child_conversation_ids: Vec::new(),
            fork_message_cursor: None,
            unread_count: 0,
            conversation_kind: CONVERSATION_KIND_CHAT.to_string(),
            root_conversation_id: None,
            delegate_id: None,
            created_at: "2026-05-06T10:00:00Z".to_string(),
            updated_at: "2026-05-06T10:00:00Z".to_string(),
            last_user_at: None,
            last_assistant_at: None,
            status: "active".to_string(),
            summary: String::new(),
            user_profile_snapshot: String::new(),
            shell_workspace_path: None,
            shell_workspaces: Vec::new(),
            shell_autonomous_mode: false,
            shell_work_mode: default_shell_work_mode(),
            archived_at: None,
            messages,
            fast_request_turns: Vec::new(),
            current_todos: Vec::new(),
            memory_recall_table: Vec::new(),
            plan_mode_enabled: false,
            preferred_api_config_id: None,
            auto_push_remote_contact_id: None,
            active_goal: None,
            cumulative_usage: ConversationCumulativeUsage::default(),
        }
    }

    #[test]
    fn side_chat_uses_normal_runtime_rules_but_stays_out_of_foreground_lists() {
        let mut conversation = test_conversation(Vec::new());
        conversation.conversation_kind = CONVERSATION_KIND_SIDE_CHAT.to_string();

        assert!(conversation_is_local_normal_chat(&conversation));
        assert!(!conversation_visible_in_foreground_lists(&conversation));
    }

    #[test]
    fn cleanup_legacy_summary_context_messages_should_backfill_legacy_message_meta() {
        let mut conversation = test_conversation(vec![
            test_chat_message("u1", "user", Some(USER_PERSONA_ID), "正常消息", None),
            test_chat_message(
                "legacy",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "旧压缩",
                Some(test_summary_meta("context_compaction", Some("旧标题"), None)),
            ),
            test_chat_message(
                "seed",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "新摘要",
                Some(test_summary_meta(
                    "summary_context_seed",
                    Some("新标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message("a1", "assistant", Some("agent-a"), "回复", None),
        ]);

        assert!(cleanup_legacy_summary_context_messages(&mut conversation));
        let remaining_ids = conversation
            .messages
            .iter()
            .map(|message| message.id.as_str())
            .collect::<Vec<_>>();
        assert_eq!(remaining_ids, vec!["u1", "legacy", "seed", "a1"]);
        let legacy_meta = conversation
            .messages
            .iter()
            .find(|message| message.id == "legacy")
            .and_then(|message| message.provider_meta.as_ref())
            .and_then(|meta| meta.get("message_meta"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            legacy_meta.get("schemaVersion").and_then(Value::as_u64),
            Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
        );
        assert_eq!(
            legacy_meta.get("title").and_then(Value::as_str),
            Some("")
        );
        assert_eq!(
            conversation_latest_summary_title(&conversation).as_deref(),
            Some("新标题")
        );
    }

    #[test]
    fn conversation_real_user_message_texts_should_skip_summary_context_and_system_user_messages() {
        let conversation = test_conversation(vec![
            test_chat_message(
                "seed",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "不要计入",
                Some(test_summary_meta(
                    "summary_context_seed",
                    Some("摘要标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message("u1", "user", Some(USER_PERSONA_ID), "第一问", None),
            test_chat_message("sys", "user", Some(SYSTEM_PERSONA_ID), "伪造系统用户", None),
            test_chat_message(
                "compact",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "不要计入二",
                Some(test_summary_meta(
                    "context_compaction",
                    Some("压缩标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message("u2", "user", Some(USER_PERSONA_ID), "第二问", None),
            test_chat_message("a1", "assistant", Some("agent-a"), "回复", None),
        ]);

        assert_eq!(
            conversation_real_user_message_texts(&conversation),
            vec!["第一问".to_string(), "第二问".to_string()]
        );
    }

    #[test]
    fn conversation_update_latest_summary_title_should_update_latest_summary_message() {
        let mut conversation = test_conversation(vec![
            test_chat_message(
                "seed",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "摘要",
                Some(test_summary_meta(
                    "summary_context_seed",
                    Some("旧标题"),
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
            test_chat_message(
                "compact",
                "user",
                Some(SYSTEM_PERSONA_ID),
                "压缩",
                Some(test_summary_meta(
                    "context_compaction",
                    None,
                    Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION),
                )),
            ),
        ]);

        assert!(conversation_update_latest_summary_title(
            &mut conversation,
            Some(" “新的标题。” "),
        ));
        assert_eq!(
            conversation_latest_summary_title(&conversation).as_deref(),
            Some("新的标题")
        );
        let latest_meta = conversation
            .messages
            .last()
            .and_then(|message| message.provider_meta.as_ref())
            .and_then(|meta| meta.get("message_meta"))
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            latest_meta
                .get("schemaVersion")
                .and_then(Value::as_u64),
            Some(SUMMARY_CONTEXT_MESSAGE_SCHEMA_VERSION)
        );
        assert_eq!(
            latest_meta
                .get("title")
                .and_then(Value::as_str),
            Some("新的标题")
        );
    }

}

pub fn sanitize_tool_history_events(events: &[Value]) -> Vec<Value> {
    fn assistant_tool_call_ids(event: &Value) -> Vec<String> {
        event
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .flat_map(|item| {
                        ["id", "call_id"]
                            .into_iter()
                            .filter_map(|key| item.get(key).and_then(Value::as_str))
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(ToOwned::to_owned)
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn assistant_with_matched_tool_calls(event: &Value, matched_ids: &[String]) -> Value {
        let mut next = event.clone();
        let Some(object) = next.as_object_mut() else {
            return next;
        };
        let filtered = event
            .get("tool_calls")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter(|item| {
                        ["id", "call_id"].into_iter().any(|key| {
                            item.get(key)
                                .and_then(Value::as_str)
                                .map(str::trim)
                                .is_some_and(|id| matched_ids.iter().any(|matched| matched == id))
                        })
                    })
                    .cloned()
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        object.insert("tool_calls".to_string(), Value::Array(filtered));
        next
    }

    #[derive(Debug, Clone)]
    struct PendingAssistant {
        pub event: Value,
        pub allowed_ids: Vec<String>,
        pub matched_ids: Vec<String>,
        pub output_index: Option<usize>,
        pub legacy_without_ids: bool,
    }

    let mut sanitized = Vec::<Value>::new();
    let mut pending_assistant: Option<PendingAssistant> = None;
    for event in events {
        let role = event
            .get("role")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        match role.as_str() {
            "assistant" => {
                let has_tool_calls = event
                    .get("tool_calls")
                    .and_then(Value::as_array)
                    .map(|items| !items.is_empty())
                    .unwrap_or(false);
                let tool_call_ids = assistant_tool_call_ids(event);
                pending_assistant = if has_tool_calls {
                    Some(PendingAssistant {
                        event: event.clone(),
                        legacy_without_ids: tool_call_ids.is_empty(),
                        allowed_ids: tool_call_ids,
                        matched_ids: Vec::new(),
                        output_index: None,
                    })
                } else {
                    sanitized.push(event.clone());
                    None
                };
            }
            "tool" => {
                if let Some(pending) = pending_assistant.as_mut() {
                    let tool_call_id = event
                        .get("tool_call_id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or_default();
                    let matched_index = pending
                        .allowed_ids
                        .iter()
                        .position(|id| id == tool_call_id);
                    let legacy_without_ids =
                        pending.legacy_without_ids && pending.output_index.is_none();
                    if legacy_without_ids || matched_index.is_some() {
                        if !pending.matched_ids.iter().any(|id| id == tool_call_id) {
                            pending.matched_ids.push(tool_call_id.to_string());
                        }
                        let assistant_event = if pending.legacy_without_ids {
                            pending.event.clone()
                        } else {
                            assistant_with_matched_tool_calls(&pending.event, &pending.matched_ids)
                        };
                        if let Some(index) = pending.output_index {
                            sanitized[index] = assistant_event;
                        } else {
                            pending.output_index = Some(sanitized.len());
                            sanitized.push(assistant_event);
                        }
                        sanitized.push(event.clone());
                        if let Some(index) = matched_index {
                            pending.allowed_ids.remove(index);
                            if pending.allowed_ids.is_empty() {
                                pending_assistant = None;
                            }
                        } else {
                            pending_assistant = None;
                        }
                    }
                }
            }
            _ => {
                pending_assistant = None;
                sanitized.push(event.clone());
            }
        }
    }
    sanitized
}

pub fn build_conversation_record(
    _api_config_id: &str,
    agent_id: &str,
    department_id: &str,
    title: &str,
    conversation_kind: &str,
    root_conversation_id: Option<String>,
    delegate_id: Option<String>,
) -> Conversation {
    let now = now_iso();
    Conversation {
        id: Uuid::new_v4().to_string(),
        title: title.trim().to_string(),
        agent_id: agent_id.to_string(),
        department_id: department_id.trim().to_string(),
        bound_conversation_id: None,
        parent_conversation_id: None,
        child_conversation_ids: Vec::new(),
        fork_message_cursor: None,
        unread_count: 0,
        conversation_kind: conversation_kind.trim().to_string(),
        root_conversation_id,
        delegate_id,
        created_at: now.clone(),
        updated_at: now,
        last_user_at: None,
        last_assistant_at: None,
        status: "active".to_string(),
        summary: String::new(),
        user_profile_snapshot: String::new(),
        shell_workspace_path: None,
        shell_workspaces: Vec::new(),
        shell_autonomous_mode: false,
        shell_work_mode: default_shell_work_mode(),
        archived_at: None,
        messages: Vec::new(),
        fast_request_turns: Vec::new(),
        current_todos: Vec::new(),
        memory_recall_table: Vec::new(),
        plan_mode_enabled: false,
        preferred_api_config_id: None,
        auto_push_remote_contact_id: None,
        cumulative_usage: ConversationCumulativeUsage::default(),
        active_goal: None,
    }
}

pub fn build_system_notification_conversation_record() -> Conversation {
    let mut conversation = build_conversation_record(
        "",
        DEFAULT_AGENT_ID,
        ASSISTANT_DEPARTMENT_ID,
        &system_notification_conversation_title(),
        CONVERSATION_KIND_SYSTEM_NOTIFICATION,
        None,
        None,
    );
    conversation.id = SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string();
    conversation.messages = Vec::new();
    conversation.last_user_at = None;
    conversation.last_assistant_at = None;
    conversation
}

pub fn ensure_active_conversation_index(
    data: &mut AppData,
    api_config_id: &str,
    agent_id: &str,
) -> usize {
    let _ = normalize_main_conversation_marker(data, agent_id);
    let _ = normalize_single_active_main_conversation(data);
    if let Some(idx) = latest_active_conversation_index(data, api_config_id, agent_id) {
        return idx;
    }

    if let Some(idx) = latest_main_conversation_index(data, agent_id) {
        for (_i, conversation) in data.conversations.iter_mut().enumerate() {
            if !conversation_visible_in_foreground_lists(conversation) || conversation_is_archived(conversation) {
                continue;
            }
            conversation.status = "active".to_string();
        }
        return idx;
    }

    let conversation = build_system_notification_conversation_record();

    for item in &mut data.conversations {
        if !conversation_visible_in_foreground_lists(item) || conversation_is_archived(item) {
            continue;
        }
        item.status = "active".to_string();
    }
    data.conversations.push(conversation);
    if data
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    }
    data.conversations.len() - 1
}

pub fn ensure_main_conversation_index(
    data: &mut AppData,
    _api_config_id: &str,
    agent_id: &str,
) -> usize {
    let _ = normalize_main_conversation_marker(data, agent_id);
    if let Some(idx) = main_conversation_index(data, agent_id) {
        return idx;
    }
    let conversation = build_system_notification_conversation_record();
    for item in &mut data.conversations {
        if !conversation_visible_in_foreground_lists(item) || conversation_is_archived(item) {
            continue;
        }
        item.status = "active".to_string();
    }
    data.conversations.push(conversation);
    data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    data.conversations.len() - 1
}

pub fn ensure_active_foreground_conversation_index_atomic(
    data: &mut AppData,
    _data_path: &PathBuf,
    _api_config_id: &str,
    agent_id: &str,
) -> usize {
    let _ = normalize_main_conversation_marker(data, agent_id);
    let _ = normalize_single_active_main_conversation(data);
    if let Some(idx) = main_conversation_index(data, agent_id) {
        for conversation in &mut data.conversations {
            if !conversation_visible_in_foreground_lists(conversation)
                || conversation_is_archived(conversation)
            {
                continue;
            }
            conversation.status = "active".to_string();
        }
        return idx;
    }

    let conversation = build_system_notification_conversation_record();
    for item in &mut data.conversations {
        if !conversation_visible_in_foreground_lists(item) || conversation_is_archived(item) {
            continue;
        }
        item.status = "active".to_string();
    }
    data.conversations.push(conversation);
    if data
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none()
    {
        data.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
    }
    data.conversations.len() - 1
}

pub fn active_foreground_conversation_index_read_only(
    data: &AppData,
    agent_id: &str,
) -> Option<usize> {
    main_conversation_index(data, agent_id)
        .or_else(|| latest_active_conversation_index(data, "", agent_id))
        .or_else(|| latest_main_conversation_index(data, agent_id))
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PromptUsageResolution {
    pub effective_prompt_tokens: u64,
    pub usage_ratio: f64,
    pub estimated_prompt_tokens: Option<u64>,
    pub source: &'static str,
}

#[derive(Debug, Clone)]
pub struct ArchiveDecision {
    pub should_archive: bool,
    pub forced: bool,
    pub reason: String,
    pub usage_ratio: f64,
}

pub fn cached_text_token_bpe() -> Option<&'static tiktoken_rs::CoreBPE> {
    static TOKEN_BPE: std::sync::OnceLock<Option<tiktoken_rs::CoreBPE>> = std::sync::OnceLock::new();
    TOKEN_BPE
        .get_or_init(|| tiktoken_rs::cl100k_base().ok())
        .as_ref()
}

pub fn truncate_text_to_token_limit(text: &str, token_limit: usize) -> String {
    if text.is_empty() || token_limit == 0 {
        return String::new();
    }
    if let Some(bpe) = cached_text_token_bpe() {
        let tokens = bpe.encode_with_special_tokens(text);
        if tokens.len() <= token_limit {
            return text.to_string();
        }
        return bpe
            .decode(tokens[..token_limit].to_vec())
            .unwrap_or_else(|_| truncate_by_chars(text, token_limit));
    }

    let mut end = 0usize;
    for (index, ch) in text.char_indices() {
        let next_end = index.saturating_add(ch.len_utf8());
        if estimated_tokens_for_text(&text[..next_end]).ceil() as usize > token_limit {
            break;
        }
        end = next_end;
    }
    text[..end].to_string()
}

pub fn build_archive_decision_from_usage_ratio(
    usage_ratio: f64,
    _last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> ArchiveDecision {
    if !has_assistant_reply {
        return ArchiveDecision {
            should_archive: false,
            forced: false,
            reason: "no_assistant_reply".to_string(),
            usage_ratio,
        };
    }
    if usage_ratio >= 0.82 {
        return ArchiveDecision {
            should_archive: true,
            forced: true,
            reason: "force_context_usage_82".to_string(),
            usage_ratio,
        };
    }

    ArchiveDecision {
        should_archive: false,
        forced: false,
        reason: "context_usage_below_force_threshold".to_string(),
        usage_ratio,
    }
}

pub fn build_archive_decision_from_estimated_usage_ratio(
    usage_ratio: f64,
    _last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> ArchiveDecision {
    if !has_assistant_reply {
        return ArchiveDecision {
            should_archive: false,
            forced: false,
            reason: "no_assistant_reply".to_string(),
            usage_ratio,
        };
    }
    if usage_ratio >= 0.95 {
        return ArchiveDecision {
            should_archive: true,
            forced: true,
            reason: "force_estimated_context_usage_95".to_string(),
            usage_ratio,
        };
    }

    ArchiveDecision {
        should_archive: false,
        forced: false,
        reason: "estimated_context_usage_below_force_threshold".to_string(),
        usage_ratio,
    }
}

pub fn decide_archive_before_model_request(
    estimated_prompt_tokens: u64,
    context_window_tokens: u32,
    last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> ArchiveDecision {
    let max_tokens = context_window_tokens.max(1) as f64;
    let usage_ratio = (estimated_prompt_tokens as f64 / max_tokens).max(0.0);
    build_archive_decision_from_usage_ratio(usage_ratio, last_user_at, has_assistant_reply)
}

pub fn decide_archive_before_send_with_fallback(
    cached_effective_prompt_tokens: u64,
    cached_usage_ratio: f64,
    estimated_prompt_tokens: Option<u64>,
    context_window_tokens: u32,
    last_user_at: Option<&str>,
    has_assistant_reply: bool,
) -> (ArchiveDecision, &'static str) {
    if cached_effective_prompt_tokens > 0 {
        return (
            decide_archive_before_model_request(
                cached_effective_prompt_tokens,
                context_window_tokens,
                last_user_at,
                has_assistant_reply,
            ),
            "cached_effective_prompt_tokens",
        );
    }
    if cached_usage_ratio.is_finite() && cached_usage_ratio > 0.0 {
        return (
            build_archive_decision_from_usage_ratio(
                cached_usage_ratio.max(0.0),
                last_user_at,
                has_assistant_reply,
            ),
            "cached_usage_ratio",
        );
    }
    (
        build_archive_decision_from_estimated_usage_ratio(
            (estimated_prompt_tokens.unwrap_or(0) as f64
                / context_window_tokens.max(1) as f64)
                .max(0.0),
            last_user_at,
            has_assistant_reply,
        ),
        "estimated_prompt_tokens",
    )
}

pub fn decide_archive_before_send_from_usage(
    usage: &PromptUsageResolution,
    last_user_at: Option<&str>,
    has_assistant_reply: bool,
    current_segment_is_compaction_summary_only: bool,
) -> (ArchiveDecision, &'static str) {
    if current_segment_is_compaction_summary_only {
        return (
            ArchiveDecision {
                should_archive: false,
                forced: false,
                reason: "current_segment_compaction_summary_only".to_string(),
                usage_ratio: usage.usage_ratio,
            },
            "current_segment_compaction_summary_only",
        );
    }
    let decision = match usage.source {
        "cached_effective_prompt_tokens"
        | "cached_usage_ratio"
        | "trusted_prompt_usage"
        | "assistant_message_effective_prompt_tokens"
        | "assistant_message_context_usage_ratio" => {
            build_archive_decision_from_usage_ratio(
                usage.usage_ratio,
                last_user_at,
                has_assistant_reply,
            )
        }
        _ => build_archive_decision_from_estimated_usage_ratio(
            usage.usage_ratio,
            last_user_at,
            has_assistant_reply,
        ),
    };
    (decision, usage.source)
}

pub fn archive_conversation_now(
    data: &mut AppData,
    conversation_id: &str,
    reason: &str,
    summary: &str,
) -> Option<String> {
    let idx = data
        .conversations
        .iter()
        .position(|c| c.id == conversation_id && conversation_is_unarchived(c))?;
    let conv = data.conversations.get_mut(idx)?;
    let previous_status = conv.status.clone();
    let now = now_iso();
    conv.status = "archived".to_string();
    conv.summary = summary.to_string();
    conv.archived_at = Some(now.clone());
    conv.updated_at = now;
    let archive_id = conv.id.clone();
    runtime_log_info(format!(
        "[会话] 已归档: conversation_id={}, previous_status={}, reason=\"{}\", summary=\"{}\"",
        conv.id,
        previous_status,
        reason,
        summary
    ));
    clear_screenshot_artifact_cache();
    Some(archive_id)
}

pub fn normalize_image_for_chat_upload(bytes: &[u8]) -> Result<LlmRequestNormalizedImage, String> {
    normalize_image_bytes_for_llm_request(bytes, None)
}

pub fn normalize_image_base64_for_llm_request(
    mime: &str,
    bytes_base64: &str,
) -> Result<(String, String), String> {
    let raw = B64
        .decode(bytes_base64.trim())
        .map_err(|err| format!("解析图片 base64 失败: {err}"))?;
    let normalized = normalize_image_bytes_for_llm_request(&raw, Some(mime.trim()))?;
    Ok((normalized.mime, B64.encode(normalized.bytes)))
}

pub fn prepared_image_payload_for_llm_request(
    mime: String,
    bytes_base64: String,
    saved_path: Option<String>,
    label: Option<String>,
) -> Option<PreparedBinaryPayload> {
    if mime.trim().eq_ignore_ascii_case("application/pdf") {
        return Some(PreparedBinaryPayload {
            mime,
            content: bytes_base64,
            saved_path,
            label: label.unwrap_or_default(),
        });
    }
    match normalize_image_base64_for_llm_request(&mime, &bytes_base64) {
        Ok((normalized_mime, normalized_base64)) => Some(PreparedBinaryPayload {
            mime: normalized_mime,
            content: normalized_base64,
            saved_path,
            label: label.unwrap_or_default(),
        }),
        Err(err) => {
            runtime_log_warn(format!(
                "[图片规范化] 图片二进制不可用，已跳过该附件并继续文本请求，原因={}，mime={}，base64_len={}，path={}",
                err,
                mime,
                bytes_base64.len(),
                saved_path.as_deref().unwrap_or("未保存")
            ));
            None
        }
    }
}



pub fn render_message_content_for_model(message: &ChatMessage) -> String {
    let mut chunks = Vec::<String>::new();
    for part in &message.parts {
        match part {
            MessagePart::Text { text, .. } => chunks.push(text.clone()),
            MessagePart::Image { mime, .. } => {
                if mime.trim().eq_ignore_ascii_case("application/pdf") {
                    chunks.push("[pdf attached]".to_string());
                } else {
                    chunks.push("[image attached]".to_string());
                }
            }
            MessagePart::Audio { .. } => chunks.push("[audio attached]".to_string()),
            MessagePart::Attachment { mime, .. } => {
                let kind = message_attachment_kind(mime);
                chunks.push(match kind {
                    "image" => "[image attached]".to_string(),
                    "audio" => "[audio attached]".to_string(),
                    "pdf" => "[pdf attached]".to_string(),
                    _ => "[file attached]".to_string(),
                });
            }
        }
    }
    if let Some(meta) = &message.provider_meta {
        if let Some(hidden_prompt_text) = meta
            .get("hiddenPromptText")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            chunks.push(hidden_prompt_text.to_string());
        }
        if let Some(task_trigger) = meta
            .get("taskTrigger")
            .and_then(Value::as_object)
            .filter(|_| {
                meta.get("messageKind")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    == Some("task_trigger")
            })
        {
            let mut lines = Vec::<String>::new();
            if let Some(task_id) = task_trigger
                .get("taskId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("taskId: {}", task_id));
            }
            if let Some(next_run_at_local) = task_trigger
                .get("next_run_at")
                .or_else(|| task_trigger.get("nextRunAt"))
                .or_else(|| task_trigger.get("nextRunAtLocal"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("next_run_at: {}", next_run_at_local));
            }
            if let Some(run_at_local) = task_trigger
                .get("run_at")
                .or_else(|| task_trigger.get("runAt"))
                .or_else(|| task_trigger.get("runAtLocal"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("run_at: {}", run_at_local));
            }
            if let Some(cron_expression) = task_trigger
                .get("cron_expression")
                .or_else(|| task_trigger.get("cronExpression"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("cron_expression: {}", cron_expression));
            }
            if let Some(end_at_local) = task_trigger
                .get("end_at")
                .or_else(|| task_trigger.get("endAt"))
                .or_else(|| task_trigger.get("endAtLocal"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                lines.push(format!("end_at: {}", end_at_local));
            }
            if !lines.is_empty() {
                chunks.push(lines.join("\n"));
            }
        }
        for (index, relative_path) in provider_meta_attachment_relative_paths(meta)
            .iter()
            .enumerate()
        {
            chunks.push(build_attachment_notice_text(index, relative_path));
        }
    }
    chunks.join(" | ")
}

fn provider_meta_attachment_relative_paths(meta: &Value) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let Some(attachments) = meta.get("attachments").and_then(Value::as_array) else {
        return out;
    };
    let mut seen = std::collections::HashSet::<String>::new();
    for item in attachments {
        let relative_path = item
            .get("relativePath")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.replace('\\', "/"));
        let Some(relative_path) = relative_path else {
            continue;
        };
        let mime = item
            .get("mime")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        let dedup_key = format!(
            "{}::{}",
            relative_path.to_ascii_lowercase(),
            mime.to_ascii_lowercase()
        );
        if !seen.insert(dedup_key) {
            continue;
        }
        out.push(relative_path);
    }
    out
}

pub fn provider_meta_message_kind(message: &ChatMessage) -> Option<String> {
    message
        .provider_meta
        .as_ref()?
        .get("message_meta")
        .or_else(|| message.provider_meta.as_ref()?.get("messageMeta"))
        .and_then(Value::as_object)?
        .get("kind")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn is_context_compaction_message(message: &ChatMessage, role: &str) -> bool {
    if role != "user" {
        return false;
    }
    matches!(
        provider_meta_message_kind(message).as_deref(),
        Some("context_compaction") | Some("summary_context_seed")
    )
}

pub fn assistant_space_display_path(relative_path: &str) -> String {
    let trimmed = relative_path.trim().trim_start_matches(['\\', '/']);
    if trimmed.is_empty() {
        "{Assistant Space}".to_string()
    } else {
        format!("{{Assistant Space}}/{}", trimmed.replace('\\', "/"))
    }
}

pub fn build_attachment_notice_text(index: usize, relative_path: &str) -> String {
    format!(
        "[附件#{}]
path: {}",
        index + 1,
        assistant_space_display_path(relative_path)
    )
}
