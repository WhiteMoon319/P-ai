#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BinaryPart {
    pub(crate) mime: String,
    pub(crate) bytes_base64: String,
    #[serde(default)]
    pub(crate) saved_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatInputPayload {
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) display_text: Option<String>,
    #[serde(default)]
    pub(crate) parts: Option<Vec<ChatIngressPart>>,
    pub(crate) images: Option<Vec<BinaryPart>>,
    pub(crate) audios: Option<Vec<BinaryPart>>,
    #[serde(default)]
    pub(crate) attachments: Option<Vec<AttachmentMetaInput>>,
    pub(crate) model: Option<String>,
    #[serde(default)]
    pub(crate) extra_text_blocks: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) mentions: Option<Vec<UserMentionTargetInput>>,
    #[serde(default)]
    pub(crate) provider_meta: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "type"
)]
pub(crate) enum ChatIngressPart {
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
pub(crate) struct UserMentionTargetInput {
    pub(crate) agent_id: String,
    #[serde(default)]
    pub(crate) agent_name: Option<String>,
    pub(crate) department_id: String,
    #[serde(default)]
    pub(crate) department_name: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttachmentMetaInput {
    pub(crate) file_name: String,
    #[serde(default, alias = "relativePath")]
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendChatRequest {
    pub(crate) payload: ChatInputPayload,
    #[serde(default)]
    pub(crate) session: Option<SessionSelector>,
    #[serde(default)]
    pub(crate) speaker_agent_id: Option<String>,
    #[serde(default)]
    pub(crate) trace_id: Option<String>,
    #[serde(default)]
    pub(crate) assistant_message_id: Option<String>,
    #[serde(default)]
    pub(crate) oldest_queue_created_at: Option<String>,
    #[serde(default)]
    pub(crate) remote_im_activation_sources: Vec<RemoteImActivationSource>,
    #[serde(default)]
    pub(crate) runtime_context: Option<RuntimeContext>,
    #[serde(default)]
    pub(crate) trigger_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopChatRequest {
    pub(crate) session: SessionSelector,
    #[serde(default)]
    pub(crate) partial_assistant_text: String,
    #[serde(default)]
    pub(crate) partial_stream_blocks: Vec<AssistantStreamBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteImReplyTarget {
    pub(crate) channel_id: String,
    pub(crate) contact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SubmitChatResult {
    pub(crate) accepted: bool,
    pub(crate) duplicate: bool,
    pub(crate) event_id: String,
    pub(crate) conversation_id: String,
    pub(crate) trace_id: String,
    pub(crate) ingress: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) user_message_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) assistant_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SendChatResult {
    pub(crate) conversation_id: String,
    pub(crate) latest_user_text: String,
    pub(crate) assistant_text: String,
    #[serde(default)]
    pub(crate) final_response_text: String,
    pub(crate) archived_before_send: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) assistant_message: Option<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) provider_prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) estimated_prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) effective_prompt_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) effective_prompt_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) context_window_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) max_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) context_usage_percent: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) remote_im_reply_decision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) remote_im_reply_target: Option<RemoteImReplyTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopChatResult {
    pub(crate) aborted: bool,
    pub(crate) persisted: bool,
    pub(crate) conversation_id: Option<String>,
    #[serde(default)]
    pub(crate) assistant_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) assistant_message: Option<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionSelector {
    pub(crate) api_config_id: Option<String>,
    #[serde(default)]
    pub(crate) department_id: Option<String>,
    pub(crate) agent_id: String,
    #[serde(default)]
    pub(crate) conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeContext {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) request_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) dispatch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) origin_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) target_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) root_conversation_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) executor_agent_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) executor_department_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) model_config_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) event_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) dispatch_reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) trusted_prompt_usage: Option<TrustedPromptUsage>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) bound_remote_im_activation_source: Option<RemoteImActivationSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) remote_im_reply_delegate_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) remote_im_reply_trigger_message_id: Option<String>,
    /// 仅在进程内传递，不能序列化进任务或持久化状态。
    #[serde(skip)]
    pub(crate) remote_im_reply_prompt_snapshot_messages: Option<Vec<ChatMessage>>,
    /// 压缩保留消息：仅进程内传递；压缩完成后置为 ready，新调度 bootstrap 才能消费。
    #[serde(skip)]
    pub(crate) compaction_preserved_messages: Option<CompactionPreservedMessages>,
    #[serde(skip)]
    pub(crate) compaction_preserved_messages_ready: bool,
    #[serde(default)]
    pub(crate) remote_im_dynamic_boundary: bool,
    /// 远程应答委托多轮执行时，禁止 core_send 在每一轮结束后立即外发。
    #[serde(default)]
    pub(crate) remote_im_defer_auto_send: bool,
}

/// 压缩保留消息：一轮已完成但未写入旧段的 assistant 正文/思维链/工具事件。
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CompactionPreservedMessages {
    pub(crate) assistant_text: String,
    pub(crate) activity_reasoning_text: String,
    pub(crate) tool_history_events: Vec<Value>,
}

impl CompactionPreservedMessages {
    pub(crate) fn new(
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
    pub(crate) fn token_usage(&self) -> u64 {
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
pub(crate) struct TrustedPromptUsage {
    pub(crate) effective_prompt_tokens: u64,
    pub(crate) context_usage_ratio: f64,
    #[serde(default)]
    pub(crate) estimated: bool,
}

pub(crate) fn runtime_context_trimmed(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub(crate) fn runtime_context_new(event_source: &str, dispatch_reason: &str) -> RuntimeContext {
    RuntimeContext {
        event_source: runtime_context_trimmed(Some(event_source)),
        dispatch_reason: runtime_context_trimmed(Some(dispatch_reason)),
        ..RuntimeContext::default()
    }
}

pub(crate) fn runtime_context_request_id_or_new(
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
pub(crate) struct ChatSnapshot {
    pub(crate) conversation_id: String,
    pub(crate) latest_user: Option<ChatMessage>,
    pub(crate) latest_assistant: Option<ChatMessage>,
    pub(crate) active_message_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptPreview {
    pub(crate) preamble: String,
    pub(crate) latest_user_text: String,
    pub(crate) latest_images: usize,
    pub(crate) latest_audios: usize,
    pub(crate) request_body_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SystemPromptPreview {
    pub(crate) system_prompt: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RefreshModelsInput {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) request_format: RequestFormat,
    #[serde(default)]
    pub(crate) provider_id: Option<String>,
    #[serde(default = "default_codex_auth_mode")]
    pub(crate) codex_auth_mode: String,
    #[serde(default = "default_codex_local_auth_path")]
    pub(crate) codex_local_auth_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QuickGenaiChatInput {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) request_format: RequestFormat,
    pub(crate) model: String,
    pub(crate) prompt: String,
    #[serde(default)]
    pub(crate) provider_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchModelMetadataInput {
    pub(crate) request_format: RequestFormat,
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FetchModelMetadataOutput {
    pub(crate) found: bool,
    pub(crate) matched_model_id: Option<String>,
    pub(crate) context_window_tokens: Option<u32>,
    pub(crate) max_output_tokens: Option<u32>,
    pub(crate) enable_image: Option<bool>,
    pub(crate) enable_tools: Option<bool>,
    pub(crate) enable_audio: Option<bool>,
    pub(crate) enable_video: Option<bool>,
    pub(crate) reasoning: Option<bool>,
    #[serde(default)]
    pub(crate) reasoning_effort_options: Vec<String>,
    pub(crate) documentation_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestEmbeddingConnectionInput {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) request_format: RequestFormat,
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestEmbeddingConnectionResult {
    pub(crate) vector_dim: usize,
    pub(crate) elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestRerankConnectionInput {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) request_format: RequestFormat,
    pub(crate) model: String,
    #[serde(default)]
    pub(crate) query: Option<String>,
    #[serde(default)]
    pub(crate) documents: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestRerankConnectionResult {
    pub(crate) result_count: usize,
    pub(crate) elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestVoiceConnectionInput {
    pub(crate) base_url: String,
    pub(crate) api_key: String,
    pub(crate) request_format: RequestFormat,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TestVoiceConnectionResult {
    pub(crate) elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CheckToolsStatusInput {
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
    #[serde(default)]
    pub(crate) api_config_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolLoadStatus {
    pub(crate) id: String,
    pub(crate) status: String,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DepartmentPermissionCatalogItem {
    pub(crate) name: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DepartmentPermissionCatalog {
    pub(crate) builtin_tools: Vec<DepartmentPermissionCatalogItem>,
    pub(crate) skills: Vec<DepartmentPermissionCatalogItem>,
    pub(crate) mcp_tools: Vec<DepartmentPermissionCatalogItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrontendToolFunctionDefinition {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FrontendToolDefinition {
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) function: FrontendToolFunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageTextCacheStats {
    pub(crate) entries: usize,
    pub(crate) total_chars: usize,
    pub(crate) latest_updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct OpenAIModelListItem {
    pub(crate) id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct OpenAIModelListResponse {
    pub(crate) data: Vec<OpenAIModelListItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GeminiNativeModelListItem {
    pub(crate) name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct GeminiNativeModelListResponse {
    #[serde(default)]
    pub(crate) models: Vec<GeminiNativeModelListItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AnthropicModelListItem {
    pub(crate) id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct AnthropicModelListResponse {
    #[serde(default)]
    pub(crate) data: Vec<AnthropicModelListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantDeltaEvent {
    pub(crate) delta: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) activation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) phase_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) tool_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", default, skip_deserializing)]
    pub(crate) stream_cache: Option<ConversationStreamRuntimeCacheSnapshot>,
}

/// 流式 delta 通道抽象：桌面端包装 tauri::ipc::Channel 回显给前端窗口；
/// Android 原生模式流式已走 NATIVE_DELTA_QUEUE 旁路（pollEvents 轮询），此处为 noop 占位，
/// 保证编译期与调用链不依赖 tauri crate。
#[derive(Clone)]
pub(crate) struct DeltaChannel {
    #[cfg(not(target_os = "android"))]
    pub(crate) inner: Option<tauri::ipc::Channel<AssistantDeltaEvent>>,
    #[cfg(target_os = "android")]
    pub(crate) _android: std::marker::PhantomData<()>,
    #[cfg(target_os = "android")]
    pub(crate) conversation_id: Option<String>,
}

impl DeltaChannel {


    #[cfg(target_os = "android")]
    pub(crate) fn noop() -> Self {
        Self {
            _android: std::marker::PhantomData,
            conversation_id: None,
        }
    }

    /// Android 原生模式：绑定会话 id，send 时把 delta 包装成原生事件队列通知。
    #[cfg(target_os = "android")]
    pub(crate) fn native_queue(conversation_id: String) -> Self {
        Self {
            _android: std::marker::PhantomData,
            conversation_id: Some(conversation_id),
        }
    }


    #[cfg(target_os = "android")]
    pub(crate) fn send(&self, event: AssistantDeltaEvent) -> Result<(), String> {
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

pub(crate) fn round_completed_delta_event(
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
pub(crate) struct ActiveChatViewBinding {
    pub(crate) window_label: String,
    pub(crate) binding_id: String,
    pub(crate) conversation_id: String,
    pub(crate) delta_channel: DeltaChannel,
}

#[derive(Debug, Clone)]
pub(crate) struct ConversationListActivityMark {
    pub(crate) activity: String,
    pub(crate) failed_message: Option<String>,
    pub(crate) completed_at: Option<String>,
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
