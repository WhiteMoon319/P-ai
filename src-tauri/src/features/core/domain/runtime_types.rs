#[derive(Clone)]
pub(crate) struct CodexRuntimeAuth {
    pub(crate) provider_id: String,
    pub(crate) auth_mode: String,
    pub(crate) local_auth_path: String,
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) account_id: Option<String>,
    pub(crate) email: Option<String>,
    pub(crate) expires_at_ms: Option<i64>,
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
pub(crate) struct ResolvedApiConfig {
    pub(crate) provider_id: Option<String>,
    pub(crate) provider_api_keys: Vec<String>,
    pub(crate) provider_key_cursor: usize,
    pub(crate) request_format: RequestFormat,
    pub(crate) allow_concurrent_requests: bool,
    pub(crate) max_concurrent_requests: Option<u32>,
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: Option<String>,
    pub(crate) temperature: Option<f64>,
    pub(crate) max_output_tokens: Option<u32>,
    pub(crate) prompt_cache_key: Option<String>,
    pub(crate) extra_headers: Vec<(String, String)>,
    pub(crate) codex_auth: Option<CodexRuntimeAuth>,
    pub(crate) codex_custom_api_key: Option<String>,
}

pub(crate) struct ProviderRequestGate {
    pub(crate) limit: usize,
    pub(crate) semaphore: std::sync::Arc<tokio::sync::Semaphore>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedBinaryPayload {
    pub(crate) mime: String,
    pub(crate) content: String,
    pub(crate) saved_path: Option<String>,
    pub(crate) label: String,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedHistoryMessage {
    pub(crate) role: String,
    pub(crate) text: String,
    pub(crate) extra_text_blocks: Vec<String>,
    pub(crate) user_time_text: Option<String>,
    pub(crate) images: Vec<PreparedBinaryPayload>,
    pub(crate) audios: Vec<PreparedBinaryPayload>,
    pub(crate) tool_calls: Option<Vec<Value>>,
    pub(crate) tool_call_id: Option<String>,
    pub(crate) reasoning_content: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PreparedPrompt {
    pub(crate) preamble: String,
    pub(crate) history_messages: Vec<PreparedHistoryMessage>,
    pub(crate) latest_user_text: String,
    pub(crate) latest_user_meta_text: String,
    pub(crate) latest_user_extra_text: String,
    pub(crate) latest_user_extra_blocks: Vec<String>,
    pub(crate) latest_images: Vec<PreparedBinaryPayload>,
    pub(crate) latest_audios: Vec<PreparedBinaryPayload>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingAppDataPersist {
    pub(crate) seq: u64,
    pub(crate) data: AppData,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingConversationPersist {
    pub(crate) seq: u64,
    pub(crate) conversations: std::collections::HashMap<String, Conversation>,
    pub(crate) metadata_conversation_ids: std::collections::HashSet<String>,
    pub(crate) deleted_conversation_ids: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ConversationDirCacheSignature {
    pub(crate) file_count: u64,
    pub(crate) total_size: u64,
    pub(crate) latest_file_name: String,
    pub(crate) latest_modified: Option<std::time::SystemTime>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct AppDataCacheSignature {
    pub(crate) agents_len: u64,
    pub(crate) agents_modified: Option<std::time::SystemTime>,
    pub(crate) runtime_len: u64,
    pub(crate) runtime_modified: Option<std::time::SystemTime>,
    pub(crate) conversations: ConversationDirCacheSignature,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteImPresenceState {
    Away,
    Present,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteImWorkState {
    Idle,
    Busy,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImReplyDelegateRuntime {
    pub(crate) delegate_id: String,
    pub(crate) contact_id: String,
    pub(crate) conversation_id: String,
    pub(crate) trigger_message_id: String,
    pub(crate) started_at: String,
    /// 委托启动瞬间冻结的在场 block 快照；之后绝不从全局当前 block 重读。
    pub(crate) prompt_snapshot_messages: Vec<ChatMessage>,
    pub(crate) guidance_messages: std::collections::VecDeque<ChatMessage>,
    /// 已消费的引导会累积到后续委托轮次的私有提示词中，但不写回初始快照。
    pub(crate) consumed_guidance_messages: Vec<ChatMessage>,
    /// 取消后禁止排队任务或已返回的模型结果继续写入联系人会话。
    pub(crate) cancelled: bool,
    /// 已开始终结，不再接收秘书引导。
    pub(crate) terminal: bool,
    pub(crate) session_agent_id: String,
    pub(crate) inspection_generation: Option<u64>,
    pub(crate) group_reply_focus: bool,
    pub(crate) group_reply_max_chars: Option<u32>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImContactRuntimeState {
    pub(crate) presence_state: RemoteImPresenceState,
    pub(crate) last_presence_at: Option<String>,
    // 旧的 Busy / has_pending 仅为兼容旧调度收尾路径保留；远程应答委托不使用它们排队。
    pub(crate) work_state: RemoteImWorkState,
    pub(crate) has_pending: bool,
    pub(crate) last_success_reply_at: Option<String>,
    pub(crate) mute_until: Option<String>,
    pub(crate) consecutive_no_reply_count: u32,
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

pub(crate) fn normalize_prepared_prompt_extra_blocks(blocks: &[String]) -> Vec<String> {
    blocks
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

pub(crate) fn prepared_prompt_latest_user_extra_blocks(prepared: &PreparedPrompt) -> Vec<String> {
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

pub(crate) fn prepared_prompt_set_latest_user_extra_blocks(
    prepared: &mut PreparedPrompt,
    blocks: Vec<String>,
) {
    let normalized = normalize_prepared_prompt_extra_blocks(&blocks);
    prepared.latest_user_extra_text = normalized.join("\n\n");
    prepared.latest_user_extra_blocks = normalized;
}

pub(crate) fn prepared_prompt_append_latest_user_extra_blocks(
    prepared: &mut PreparedPrompt,
    blocks: &[String],
) {
    let mut merged = prepared_prompt_latest_user_extra_blocks(prepared);
    merged.extend(normalize_prepared_prompt_extra_blocks(blocks));
    prepared_prompt_set_latest_user_extra_blocks(prepared, merged);
}

pub(crate) fn prepared_prompt_append_latest_user_extra_block(
    prepared: &mut PreparedPrompt,
    block: impl AsRef<str>,
) {
    let trimmed = block.as_ref().trim();
    if trimmed.is_empty() {
        return;
    }
    prepared_prompt_append_latest_user_extra_blocks(prepared, &[trimmed.to_string()]);
}

pub(crate) fn prepared_prompt_prepend_latest_user_extra_block(
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

pub(crate) fn prepared_prompt_latest_user_text_blocks(prepared: &PreparedPrompt) -> Vec<String> {
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
pub(crate) struct ChatSettings {
    #[serde(alias = "selectedAgentId", alias = "selected_agent_id")]
    pub(crate) assistant_department_agent_id: String,
    pub(crate) user_alias: String,
    #[serde(default = "default_response_style_id")]
    pub(crate) response_style_id: String,
    #[serde(default = "default_pdf_read_mode")]
    pub(crate) pdf_read_mode: String,
    #[serde(default = "default_background_voice_screenshot_keywords")]
    pub(crate) background_voice_screenshot_keywords: String,
    #[serde(default = "default_background_voice_screenshot_mode")]
    pub(crate) background_voice_screenshot_mode: String,
    #[serde(default)]
    pub(crate) instruction_presets: Vec<PromptCommandPreset>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppBootstrapSnapshot {
    pub(crate) config: AppConfig,
    pub(crate) agents: Vec<AgentProfile>,
    pub(crate) chat_settings: ChatSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationApiSettings {
    #[serde(alias = "chatApiConfigId", alias = "chat_api_config_id")]
    pub(crate) assistant_department_api_config_id: String,
    #[serde(default)]
    pub(crate) vision_api_config_id: Option<String>,
    #[serde(default)]
    pub(crate) tool_review_api_config_id: Option<String>,
    #[serde(default)]
    pub(crate) stt_api_config_id: Option<String>,
    #[serde(default)]
    pub(crate) stt_auto_send: bool,
}
