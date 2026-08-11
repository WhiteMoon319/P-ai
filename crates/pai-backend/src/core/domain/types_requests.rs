use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::core::domain::types_chat::{
    AssistantStreamBlock, ChatMessage, ConversationStreamRuntimeCacheSnapshot,
    RemoteImActivationSource,
};
use crate::core::domain::types_config::{default_codex_auth_mode, default_codex_local_auth_path};
use crate::core::domain::types_foundation::RequestFormat;
use uuid::Uuid;

/// 轻量 token 估算（字符级启发式，与 src-tauri conversation.rs 回退逻辑一致；
/// 不依赖 tokenizer 缓存，pai-backend 纯逻辑侧使用）。
pub fn estimated_tokens_for_text(text: &str) -> f64 {
    let mut zh_chars = 0usize;
    let mut other_chars = 0usize;
    for ch in text.chars() {
        if ch.is_whitespace() {
            continue;
        }
        if ('\u{4e00}'..='\u{9fff}').contains(&ch)
            || ('\u{3400}'..='\u{4dbf}').contains(&ch)
            || ('\u{f900}'..='\u{faff}').contains(&ch)
        {
            zh_chars += 1;
        } else {
            other_chars += 1;
        }
    }
    (zh_chars as f64) + (other_chars as f64) / 3.5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BinaryPart {
    pub mime: String,
    pub bytes_base64: String,
    #[serde(default)]
    pub saved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatInputPayload {
    pub text: Option<String>,
    #[serde(default)]
    pub display_text: Option<String>,
    #[serde(default)]
    pub parts: Option<Vec<ChatIngressPart>>,
    pub images: Option<Vec<BinaryPart>>,
    pub audios: Option<Vec<BinaryPart>>,
    #[serde(default)]
    pub attachments: Option<Vec<AttachmentMetaInput>>,
    pub model: Option<String>,
    #[serde(default)]
    pub extra_text_blocks: Option<Vec<String>>,
    #[serde(default)]
    pub mentions: Option<Vec<UserMentionTargetInput>>,
    #[serde(default)]
    pub provider_meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub enum ChatIngressPart {
    Text {
        text: String,
    },
    Attachment {
        #[serde(default)]
        path: Option<String>,
        #[serde(default)]
        bytes_base64: Option<String>,
        #[serde(default)]
        mime: String,
        #[serde(default)]
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserMentionTargetInput {
    pub agent_id: String,
    #[serde(default)]
    pub agent_name: Option<String>,
    pub department_id: String,
    #[serde(default)]
    pub department_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachmentMetaInput {
    pub file_name: String,
    #[serde(default, alias = "relativePath")]
    pub path: String,
    #[serde(default)]
    pub mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatRequest {
    pub payload: ChatInputPayload,
    #[serde(default)]
    pub session: Option<SessionSelector>,
    #[serde(default)]
    pub speaker_agent_id: Option<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub assistant_message_id: Option<String>,
    #[serde(default)]
    pub oldest_queue_created_at: Option<String>,
    #[serde(default)]
    pub remote_im_activation_sources: Vec<RemoteImActivationSource>,
    #[serde(default)]
    pub runtime_context: Option<RuntimeContext>,
    #[serde(default)]
    pub trigger_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopChatRequest {
    pub session: SessionSelector,
    #[serde(default)]
    pub partial_assistant_text: String,
    #[serde(default)]
    pub partial_stream_blocks: Vec<AssistantStreamBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImReplyTarget {
    pub channel_id: String,
    pub contact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmitChatResult {
    pub accepted: bool,
    pub duplicate: bool,
    pub event_id: String,
    pub conversation_id: String,
    pub trace_id: String,
    pub ingress: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SendChatResult {
    pub conversation_id: String,
    pub latest_user_text: String,
    pub assistant_text: String,
    #[serde(default)]
    pub final_response_text: String,
    pub archived_before_send: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_message: Option<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_prompt_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_window_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_usage_percent: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_im_reply_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_im_reply_target: Option<RemoteImReplyTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopChatResult {
    pub aborted: bool,
    pub persisted: bool,
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub assistant_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assistant_message: Option<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSelector {
    pub api_config_id: Option<String>,
    #[serde(default)]
    pub department_id: Option<String>,
    pub agent_id: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub origin_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub root_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor_department_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_config_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dispatch_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trusted_prompt_usage: Option<TrustedPromptUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bound_remote_im_activation_source: Option<RemoteImActivationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_im_reply_delegate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_im_reply_trigger_message_id: Option<String>,
    /// 仅在进程内传递，不能序列化进任务或持久化状态。
    #[serde(skip)]
    pub remote_im_reply_prompt_snapshot_messages: Option<Vec<ChatMessage>>,
    /// 压缩保留消息：仅进程内传递；压缩完成后置为 ready，新调度 bootstrap 才能消费。
    #[serde(skip)]
    pub compaction_preserved_messages: Option<CompactionPreservedMessages>,
    #[serde(skip)]
    pub compaction_preserved_messages_ready: bool,
    #[serde(default)]
    pub remote_im_dynamic_boundary: bool,
    /// 远程应答委托多轮执行时，禁止 core_send 在每一轮结束后立即外发。
    #[serde(default)]
    pub remote_im_defer_auto_send: bool,
}

/// 压缩保留消息：一轮已完成但未写入旧段的 assistant 正文/思维链/工具事件。
#[derive(Debug, Clone, PartialEq)]
pub struct CompactionPreservedMessages {
    pub assistant_text: String,
    pub activity_reasoning_text: String,
    pub tool_history_events: Vec<Value>,
}

impl CompactionPreservedMessages {
    pub fn new(
        assistant_text: impl Into<String>,
        activity_reasoning_text: impl Into<String>,
        tool_history_events: Vec<Value>,
    ) -> Self {
        Self {
            assistant_text: assistant_text.into(),
            activity_reasoning_text: activity_reasoning_text.into(),
            tool_history_events,
        }
    }

    /// 复用现有 `estimated_tokens_for_text`，只估本组消息本身。
    pub fn token_usage(&self) -> u64 {
        let mut total = 0.0f64;
        total += estimated_tokens_for_text(self.assistant_text.trim());
        // 与 prepare 估算一致：reasoning 不计入 prompt 输入。
        for event in &self.tool_history_events {
            let role = event
                .get("role")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default();
            if role.eq_ignore_ascii_case("assistant") {
                if let Some(content) = event.get("content").and_then(Value::as_str) {
                    total += estimated_tokens_for_text(content);
                }
                if let Some(calls) = event.get("tool_calls").and_then(Value::as_array) {
                    for call in calls {
                        if let Some(function) = call.get("function") {
                            if let Some(name) = function.get("name").and_then(Value::as_str) {
                                total += estimated_tokens_for_text(name);
                            }
                            if let Some(arguments) =
                                function.get("arguments").and_then(Value::as_str)
                            {
                                total += estimated_tokens_for_text(arguments);
                            }
                        }
                    }
                }
            } else if role.eq_ignore_ascii_case("tool") {
                if let Some(content) = event.get("content").and_then(Value::as_str) {
                    total += estimated_tokens_for_text(content);
                }
            } else if let Some(content) = event.get("content").and_then(Value::as_str) {
                total += estimated_tokens_for_text(content);
            }
            total += 4.0;
        }
        total.ceil().max(0.0).min(u64::MAX as f64) as u64
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TrustedPromptUsage {
    pub effective_prompt_tokens: u64,
    pub context_usage_ratio: f64,
    #[serde(default)]
    pub estimated: bool,
}

pub fn runtime_context_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn runtime_context_new(event_source: &str, dispatch_reason: &str) -> RuntimeContext {
    RuntimeContext {
        event_source: runtime_context_trimmed(Some(event_source)),
        dispatch_reason: runtime_context_trimmed(Some(dispatch_reason)),
        ..RuntimeContext::default()
    }
}

pub fn runtime_context_request_id_or_new(
    runtime_context: Option<&RuntimeContext>,
    trace_id: Option<&str>,
    prefix: &str,
) -> String {
    runtime_context
        .and_then(|value| value.request_id.as_deref())
        .or(trace_id)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{}-{}", prefix.trim(), Uuid::new_v4()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatSnapshot {
    pub conversation_id: String,
    pub latest_user: Option<ChatMessage>,
    pub latest_assistant: Option<ChatMessage>,
    pub active_message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPreview {
    pub preamble: String,
    pub latest_user_text: String,
    pub latest_images: usize,
    pub latest_audios: usize,
    pub request_body_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemPromptPreview {
    pub system_prompt: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RefreshModelsInput {
    pub base_url: String,
    pub api_key: String,
    pub request_format: RequestFormat,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default = "default_codex_auth_mode")]
    pub codex_auth_mode: String,
    #[serde(default = "default_codex_local_auth_path")]
    pub codex_local_auth_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuickGenaiChatInput {
    pub base_url: String,
    pub api_key: String,
    pub request_format: RequestFormat,
    pub model: String,
    pub prompt: String,
    #[serde(default)]
    pub provider_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModelMetadataInput {
    pub request_format: RequestFormat,
    pub model: String,
    #[serde(default)]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchModelMetadataOutput {
    pub found: bool,
    pub matched_model_id: Option<String>,
    pub context_window_tokens: Option<u32>,
    pub max_output_tokens: Option<u32>,
    pub enable_image: Option<bool>,
    pub enable_tools: Option<bool>,
    pub enable_audio: Option<bool>,
    pub enable_video: Option<bool>,
    pub reasoning: Option<bool>,
    #[serde(default)]
    pub reasoning_effort_options: Vec<String>,
    pub documentation_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestEmbeddingConnectionInput {
    pub base_url: String,
    pub api_key: String,
    pub request_format: RequestFormat,
    pub model: String,
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestEmbeddingConnectionResult {
    pub vector_dim: usize,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRerankConnectionInput {
    pub base_url: String,
    pub api_key: String,
    pub request_format: RequestFormat,
    pub model: String,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub documents: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestRerankConnectionResult {
    pub result_count: usize,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestVoiceConnectionInput {
    pub base_url: String,
    pub api_key: String,
    pub request_format: RequestFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestVoiceConnectionResult {
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckToolsStatusInput {
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub api_config_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolLoadStatus {
    pub id: String,
    pub status: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentPermissionCatalogItem {
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DepartmentPermissionCatalog {
    pub builtin_tools: Vec<DepartmentPermissionCatalogItem>,
    pub skills: Vec<DepartmentPermissionCatalogItem>,
    pub mcp_tools: Vec<DepartmentPermissionCatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendToolFunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrontendToolDefinition {
    #[serde(rename = "type")]
    pub kind: String,
    pub function: FrontendToolFunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageTextCacheStats {
    pub entries: usize,
    pub total_chars: usize,
    pub latest_updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIModelListItem {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAIModelListResponse {
    pub data: Vec<OpenAIModelListItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiNativeModelListItem {
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeminiNativeModelListResponse {
    #[serde(default)]
    pub models: Vec<GeminiNativeModelListItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicModelListItem {
    pub id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AnthropicModelListResponse {
    #[serde(default)]
    pub data: Vec<AnthropicModelListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssistantDeltaEvent {
    pub delta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub activation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phase_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, skip_deserializing)]
    pub stream_cache: Option<ConversationStreamRuntimeCacheSnapshot>,
}

/// 流式 delta 通道：Android 原生模式流式走 NATIVE_DELTA_QUEUE 旁路（pollEvents 轮询），
/// 通过会话 id 绑定后 push 原生事件队列；不依赖 tauri crate（阶段 4 迁入 pai-backend）。
#[derive(Clone)]
pub struct DeltaChannel {
    pub _android: std::marker::PhantomData<()>,
    pub conversation_id: Option<String>,
}

impl DeltaChannel {
    pub fn noop() -> Self {
        Self {
            _android: std::marker::PhantomData,
            conversation_id: None,
        }
    }

    /// Android 原生模式：绑定会话 id，send 时把 delta 包装成原生事件队列通知。
    pub fn native_queue(conversation_id: String) -> Self {
        Self {
            _android: std::marker::PhantomData,
            conversation_id: Some(conversation_id),
        }
    }

    pub fn send(&self, event: AssistantDeltaEvent) -> Result<(), String> {
        let Some(conversation_id) = self.conversation_id.as_deref() else {
            return Ok(());
        };
        // 与 dispatch_assistant_delta_to_active_view 的 Android 分支相同格式：
        // push 原生事件队列，Kotlin pollEvents 消费。
        let delta_event = serde_json::json!({
            "delta": event.delta,
            "kind": event.kind,
            "requestId": event.request_id,
            "toolName": event.tool_name,
            "toolStatus": event.tool_status,
            "message": event.message,
        });
        let notification = serde_json::json!({
            "method": "chat.assistantDelta",
            "params": {
                "conversationId": conversation_id.trim(),
                "event": delta_event,
            },
        });
        crate::push_native_delta_event(notification);
        Ok(())
    }
}

pub fn round_completed_delta_event(
    conversation_id: &str,
    request_id: Option<&str>,
    assistant_text: &str,
    assistant_message: Option<&ChatMessage>,
) -> AssistantDeltaEvent {
    let normalized_request_id = request_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let message = serde_json::json!({
        "conversationId": conversation_id.trim(),
        "activationId": normalized_request_id,
        "requestId": normalized_request_id,
        "assistantText": assistant_text,
        "archivedBeforeSend": false,
        "assistantMessage": assistant_message,
    })
    .to_string();
    AssistantDeltaEvent {
        delta: String::new(),
        kind: Some("round_completed".to_string()),
        request_id: normalized_request_id.clone(),
        activation_id: normalized_request_id,
        phase_id: None,
        reason: Some("context_compaction_boundary".to_string()),
        tool_name: None,
        tool_call_id: None,
        tool_status: None,
        tool_args: None,
        message: Some(message),
        stream_cache: None,
    }
}

#[derive(Clone)]
pub struct ActiveChatViewBinding {
    pub window_label: String,
    pub binding_id: String,
    pub conversation_id: String,
    pub delta_channel: DeltaChannel,
}

#[derive(Debug, Clone)]
pub struct ConversationListActivityMark {
    pub activity: String,
    pub failed_message: Option<String>,
    pub completed_at: Option<String>,
}

#[cfg(test)]
mod compaction_preserved_messages_tests {
    use super::*;

    #[test]
    fn compaction_preserved_messages_token_usage_should_be_stable() {
        let group = CompactionPreservedMessages::new(
            "hello",
            "think",
            vec![
                serde_json::json!({
                    "role":"assistant",
                    "content": null,
                    "tool_calls":[{"id":"c1","type":"function","function":{"name":"read_file","arguments":"{\"path\":\"a\"}"}}]
                }),
                serde_json::json!({"role":"tool","tool_call_id":"c1","content":"body"}),
            ],
        );
        let a = group.token_usage();
        let b = group.token_usage();
        assert_eq!(a, b);
        assert!(a > 0);
    }
}
