use serde::{Deserialize, Serialize};

use crate::core::domain::types_chat::{AgentProfile, ChatMessage, Conversation};
use crate::core::domain::types_config::AppConfig;
use crate::core::domain::types_foundation::{
    default_background_voice_screenshot_keywords, default_background_voice_screenshot_mode,
    default_pdf_read_mode, default_response_style_id, RequestFormat,
};
use crate::core::domain::types_storage::{AppData, PromptCommandPreset};
use serde_json::Value;

#[derive(Clone)]
pub struct CodexRuntimeAuth {
    pub provider_id: String,
    pub auth_mode: String,
    pub local_auth_path: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub account_id: Option<String>,
    pub email: Option<String>,
    pub expires_at_ms: Option<i64>,
}

impl std::fmt::Debug for CodexRuntimeAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexRuntimeAuth")
            .field("provider_id", &self.provider_id)
            .field("auth_mode", &self.auth_mode)
            .field("local_auth_path", &self.local_auth_path)
            .field("access_token", &"<redacted>")
            .field("refresh_token", &self.refresh_token.as_ref().map(|_| "<redacted>"))
            .field("account_id", &self.account_id.as_ref().map(|_| "<redacted>"))
            .field("email", &self.email.as_ref().map(|_| "<redacted>"))
            .field("expires_at_ms", &self.expires_at_ms.map(|_| "<redacted>"))
            .finish()
    }
}

#[derive(Debug, Clone)]
pub struct ResolvedApiConfig {
    pub provider_id: Option<String>,
    pub provider_api_keys: Vec<String>,
    pub provider_key_cursor: usize,
    pub request_format: RequestFormat,
    pub allow_concurrent_requests: bool,
    pub max_concurrent_requests: Option<u32>,
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    pub reasoning_effort: Option<String>,
    pub temperature: Option<f64>,
    pub max_output_tokens: Option<u32>,
    pub prompt_cache_key: Option<String>,
    pub extra_headers: Vec<(String, String)>,
    pub codex_auth: Option<CodexRuntimeAuth>,
    pub codex_custom_api_key: Option<String>,
}

pub struct ProviderRequestGate {
    pub limit: usize,
    pub semaphore: std::sync::Arc<tokio::sync::Semaphore>,
}

#[derive(Debug, Clone)]
pub struct PreparedBinaryPayload {
    pub mime: String,
    pub content: String,
    pub saved_path: Option<String>,
    pub label: String,
}

#[derive(Debug, Clone)]
pub struct PreparedHistoryMessage {
    pub role: String,
    pub text: String,
    pub extra_text_blocks: Vec<String>,
    pub user_time_text: Option<String>,
    pub images: Vec<PreparedBinaryPayload>,
    pub audios: Vec<PreparedBinaryPayload>,
    pub tool_calls: Option<Vec<Value>>,
    pub tool_call_id: Option<String>,
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PreparedPrompt {
    pub preamble: String,
    pub history_messages: Vec<PreparedHistoryMessage>,
    pub latest_user_text: String,
    pub latest_user_meta_text: String,
    pub latest_user_extra_text: String,
    pub latest_user_extra_blocks: Vec<String>,
    pub latest_images: Vec<PreparedBinaryPayload>,
    pub latest_audios: Vec<PreparedBinaryPayload>,
}

#[derive(Debug, Clone)]
pub struct PendingAppDataPersist {
    pub seq: u64,
    pub data: AppData,
}

#[derive(Debug, Clone)]
pub struct PendingConversationPersist {
    pub seq: u64,
    pub conversations: std::collections::HashMap<String, Conversation>,
    pub metadata_conversation_ids: std::collections::HashSet<String>,
    pub deleted_conversation_ids: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ConversationDirCacheSignature {
    pub file_count: u64,
    pub total_size: u64,
    pub latest_file_name: String,
    pub latest_modified: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AppDataCacheSignature {
    pub agents_len: u64,
    pub agents_modified: Option<std::time::SystemTime>,
    pub runtime_len: u64,
    pub runtime_modified: Option<std::time::SystemTime>,
    pub conversations: ConversationDirCacheSignature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteImPresenceState {
    Away,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteImWorkState {
    Idle,
    Busy,
}

#[derive(Debug, Clone)]
pub struct RemoteImReplyDelegateRuntime {
    pub delegate_id: String,
    pub contact_id: String,
    pub conversation_id: String,
    pub trigger_message_id: String,
    pub started_at: String,
    /// 委托启动瞬间冻结的在场 block 快照；之后绝不从全局当前 block 重读。
    pub prompt_snapshot_messages: Vec<ChatMessage>,
    pub guidance_messages: std::collections::VecDeque<ChatMessage>,
    /// 已消费的引导会累积到后续委托轮次的私有提示词中，但不写回初始快照。
    pub consumed_guidance_messages: Vec<ChatMessage>,
    /// 取消后禁止排队任务或已返回的模型结果继续写入联系人会话。
    pub cancelled: bool,
    /// 已开始终结，不再接收秘书引导。
    pub terminal: bool,
    pub session_agent_id: String,
    pub inspection_generation: Option<u64>,
    pub group_reply_focus: bool,
    pub group_reply_max_chars: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct RemoteImContactRuntimeState {
    pub presence_state: RemoteImPresenceState,
    pub last_presence_at: Option<String>,
    // 旧的 Busy / has_pending 仅为兼容旧调度收尾路径保留；远程应答委托不使用它们排队。
    pub work_state: RemoteImWorkState,
    pub has_pending: bool,
    pub last_success_reply_at: Option<String>,
    pub mute_until: Option<String>,
    pub consecutive_no_reply_count: u32,
}

impl Default for RemoteImContactRuntimeState {
    fn default() -> Self {
        Self {
            presence_state: RemoteImPresenceState::Away,
            last_presence_at: None,
            work_state: RemoteImWorkState::Idle,
            has_pending: false,
            last_success_reply_at: None,
            mute_until: None,
            consecutive_no_reply_count: 0,
        }
    }
}

pub fn normalize_prepared_prompt_extra_blocks(blocks: &[String]) -> Vec<String> {
    blocks
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub fn prepared_prompt_latest_user_extra_blocks(prepared: &PreparedPrompt) -> Vec<String> {
    let normalized = normalize_prepared_prompt_extra_blocks(&prepared.latest_user_extra_blocks);
    if !normalized.is_empty() {
        return normalized;
    }
    let fallback = prepared.latest_user_extra_text.trim();
    if fallback.is_empty() {
        Vec::new()
    } else {
        vec![fallback.to_string()]
    }
}

pub fn prepared_prompt_set_latest_user_extra_blocks(
    prepared: &mut PreparedPrompt,
    blocks: Vec<String>,
) {
    let normalized = normalize_prepared_prompt_extra_blocks(&blocks);
    prepared.latest_user_extra_text = normalized.join("\n\n");
    prepared.latest_user_extra_blocks = normalized;
}

pub fn prepared_prompt_append_latest_user_extra_blocks(
    prepared: &mut PreparedPrompt,
    blocks: &[String],
) {
    let mut merged = prepared_prompt_latest_user_extra_blocks(prepared);
    merged.extend(normalize_prepared_prompt_extra_blocks(blocks));
    prepared_prompt_set_latest_user_extra_blocks(prepared, merged);
}

pub fn prepared_prompt_append_latest_user_extra_block(
    prepared: &mut PreparedPrompt,
    block: impl AsRef<str>,
) {
    let trimmed = block.as_ref().trim();
    if trimmed.is_empty() {
        return;
    }
    prepared_prompt_append_latest_user_extra_blocks(prepared, &[trimmed.to_string()]);
}

pub fn prepared_prompt_prepend_latest_user_extra_block(
    prepared: &mut PreparedPrompt,
    block: impl AsRef<str>,
) {
    let trimmed = block.as_ref().trim();
    if trimmed.is_empty() {
        return;
    }
    let mut merged = vec![trimmed.to_string()];
    merged.extend(prepared_prompt_latest_user_extra_blocks(prepared));
    prepared_prompt_set_latest_user_extra_blocks(prepared, merged);
}

pub fn prepared_prompt_latest_user_text_blocks(prepared: &PreparedPrompt) -> Vec<String> {
    let mut blocks = Vec::<String>::new();
    let extra_blocks = prepared_prompt_latest_user_extra_blocks(prepared);
    blocks.extend(
        extra_blocks
            .iter()
            .filter(|block| block.starts_with("[系统提醒]"))
            .cloned(),
    );
    for text in [
        prepared.latest_user_meta_text.trim(),
        prepared.latest_user_text.trim(),
    ] {
        if !text.is_empty() {
            blocks.push(text.to_string());
        }
    }
    blocks.extend(
        extra_blocks
            .into_iter()
            .filter(|block| !block.starts_with("[系统提醒]")),
    );
    blocks
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSettings {
    #[serde(alias = "selectedAgentId", alias = "selected_agent_id")]
    pub assistant_department_agent_id: String,
    pub user_alias: String,
    #[serde(default = "default_response_style_id")]
    pub response_style_id: String,
    #[serde(default = "default_pdf_read_mode")]
    pub pdf_read_mode: String,
    #[serde(default = "default_background_voice_screenshot_keywords")]
    pub background_voice_screenshot_keywords: String,
    #[serde(default = "default_background_voice_screenshot_mode")]
    pub background_voice_screenshot_mode: String,
    #[serde(default)]
    pub instruction_presets: Vec<PromptCommandPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppBootstrapSnapshot {
    pub config: AppConfig,
    pub agents: Vec<AgentProfile>,
    pub chat_settings: ChatSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationApiSettings {
    #[serde(alias = "chatApiConfigId", alias = "chat_api_config_id")]
    pub assistant_department_api_config_id: String,
    #[serde(default)]
    pub vision_api_config_id: Option<String>,
    #[serde(default)]
    pub tool_review_api_config_id: Option<String>,
    #[serde(default)]
    pub stt_api_config_id: Option<String>,
    #[serde(default)]
    pub stt_auto_send: bool,
}
