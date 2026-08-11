pub(crate) const MEMORY_RECALL_MODE_AUTO: &str = "auto";
pub(crate) const MEMORY_RECALL_MODE_MANUAL: &str = "manual";
pub(crate) const MEMORY_RECALL_MODE_OFF: &str = "off";

pub(crate) fn default_agent_memory_recall_mode() -> String {
    MEMORY_RECALL_MODE_AUTO.to_string()
}

pub(crate) fn normalize_agent_memory_recall_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        MEMORY_RECALL_MODE_MANUAL => MEMORY_RECALL_MODE_MANUAL.to_string(),
        MEMORY_RECALL_MODE_OFF => MEMORY_RECALL_MODE_OFF.to_string(),
        _ => MEMORY_RECALL_MODE_AUTO.to_string(),
    }
}

pub(crate) fn agent_memory_recall_mode(agent: &AgentProfile) -> String {
    normalize_agent_memory_recall_mode(&agent.memory_recall_mode)
}

pub(crate) fn agent_memory_rag_enabled(agent: &AgentProfile) -> bool {
    agent_memory_recall_mode(agent) == MEMORY_RECALL_MODE_AUTO
}

pub(crate) fn agent_memory_recall_enabled(agent: &AgentProfile) -> bool {
    agent_memory_recall_mode(agent) != MEMORY_RECALL_MODE_OFF
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentProfile {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) system_prompt: String,
    #[serde(default = "default_agent_tools")]
    pub(crate) tools: Vec<ApiToolConfig>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[serde(default)]
    pub(crate) avatar_path: Option<String>,
    #[serde(default)]
    pub(crate) avatar_updated_at: Option<String>,
    #[serde(default)]
    pub(crate) is_built_in_user: bool,
    #[serde(default)]
    pub(crate) is_built_in_system: bool,
    #[serde(default)]
    pub(crate) private_memory_enabled: bool,
    #[serde(default = "default_agent_memory_recall_mode")]
    pub(crate) memory_recall_mode: String,
    #[serde(default = "default_main_source")]
    pub(crate) source: String,
    #[serde(default = "default_global_scope")]
    pub(crate) scope: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveAgentsInput {
    pub(crate) agents: Vec<AgentProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub(crate) enum MessagePart {
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reasoning_content: Option<String>,
    },
    Image {
        mime: String,
        #[serde(alias = "bytesBase64")]
        bytes_base64: String,
        name: Option<String>,
        #[serde(default)]
        compressed: bool,
    },
    Audio {
        mime: String,
        #[serde(alias = "bytesBase64")]
        bytes_base64: String,
        name: Option<String>,
        #[serde(default)]
        compressed: bool,
    },
    Attachment {
        path: String,
        mime: String,
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemeAnnotation {
    pub(crate) meme: String,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatMessage {
    pub(crate) id: String,
    pub(crate) role: String,
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) speaker_agent_id: Option<String>,
    pub(crate) parts: Vec<MessagePart>,
    #[serde(default)]
    pub(crate) extra_text_blocks: Vec<String>,
    pub(crate) provider_meta: Option<Value>,
    pub(crate) tool_call: Option<Vec<Value>>,
    pub(crate) mcp_call: Option<Vec<Value>>,
    // 正式 meme 替换语义：把正文中的 token 映射为 markdown 图片引用。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) meme_annotations: Option<Vec<MemeAnnotation>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteImMessageSource {
    pub(crate) channel_id: String,
    pub(crate) platform: RemoteImPlatform,
    pub(crate) im_name: String,
    pub(crate) remote_contact_type: String,
    pub(crate) remote_contact_id: String,
    pub(crate) remote_contact_name: String,
    pub(crate) sender_id: String,
    pub(crate) sender_name: String,
    #[serde(default)]
    pub(crate) sender_avatar_url: Option<String>,
    #[serde(default)]
    pub(crate) platform_message_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteImActivationSource {
    pub(crate) channel_id: String,
    pub(crate) platform: RemoteImPlatform,
    pub(crate) remote_contact_type: String,
    pub(crate) remote_contact_id: String,
    pub(crate) remote_contact_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationTodoItem {
    pub(crate) content: String,
    pub(crate) status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FastRequestTurn {
    #[serde(default)]
    pub(crate) id: String,
    #[serde(default)]
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) request_text: String,
    #[serde(default)]
    pub(crate) response_text: String,
    #[serde(default)]
    pub(crate) success: bool,
    #[serde(default)]
    pub(crate) error: Option<String>,
    #[serde(default)]
    pub(crate) model_name: Option<String>,
    #[serde(default)]
    pub(crate) duration_ms: Option<u64>,
    #[serde(default)]
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationUsageBucket {
    #[serde(default)]
    pub(crate) input_tokens: u64,
    #[serde(default)]
    pub(crate) output_tokens: u64,
    #[serde(default)]
    pub(crate) total_tokens: u64,
    #[serde(default)]
    pub(crate) cache_read_tokens: u64,
    #[serde(default)]
    pub(crate) cache_write_tokens: u64,
    #[serde(default)]
    pub(crate) reasoning_tokens: u64,
}

impl ConversationUsageBucket {
    pub(crate) fn needs_legacy_total_tokens_backfill(&self) -> bool {
        self.total_tokens < self.input_tokens.saturating_add(self.output_tokens)
    }

    pub(crate) fn normalized_legacy_totals(mut self) -> Self {
        let floor_total_tokens = self.input_tokens.saturating_add(self.output_tokens);
        self.total_tokens = self.total_tokens.max(floor_total_tokens);
        self
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.total_tokens == 0
            && self.cache_read_tokens == 0
            && self.cache_write_tokens == 0
            && self.reasoning_tokens == 0
    }

    pub(crate) fn keep_at_least(&mut self, other: &ConversationUsageBucket) {
        self.input_tokens = self.input_tokens.max(other.input_tokens);
        self.output_tokens = self.output_tokens.max(other.output_tokens);
        self.total_tokens = self.total_tokens.max(other.total_tokens);
        self.cache_read_tokens = self.cache_read_tokens.max(other.cache_read_tokens);
        self.cache_write_tokens = self.cache_write_tokens.max(other.cache_write_tokens);
        self.reasoning_tokens = self.reasoning_tokens.max(other.reasoning_tokens);
    }

    pub(crate) fn saturating_add_assign(&mut self, other: &ConversationUsageBucket) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.total_tokens = self.total_tokens.saturating_add(other.total_tokens);
        self.cache_read_tokens = self.cache_read_tokens.saturating_add(other.cache_read_tokens);
        self.cache_write_tokens = self.cache_write_tokens.saturating_add(other.cache_write_tokens);
        self.reasoning_tokens = self.reasoning_tokens.saturating_add(other.reasoning_tokens);
    }

    pub(crate) fn saturating_sub_from_totals(
        total_input_tokens: u64,
        total_output_tokens: u64,
        total_total_tokens: u64,
        total_cache_read_tokens: u64,
        total_cache_write_tokens: u64,
        total_reasoning_tokens: u64,
        used: &ConversationUsageBucket,
    ) -> ConversationUsageBucket {
        ConversationUsageBucket {
            input_tokens: total_input_tokens.saturating_sub(used.input_tokens),
            output_tokens: total_output_tokens.saturating_sub(used.output_tokens),
            total_tokens: total_total_tokens.saturating_sub(used.total_tokens),
            cache_read_tokens: total_cache_read_tokens.saturating_sub(used.cache_read_tokens),
            cache_write_tokens: total_cache_write_tokens.saturating_sub(used.cache_write_tokens),
            reasoning_tokens: total_reasoning_tokens.saturating_sub(used.reasoning_tokens),
        }
    }
}

pub(crate) fn conversation_provider_model_usage_is_empty(
    value: &std::collections::BTreeMap<String, std::collections::BTreeMap<String, ConversationUsageBucket>>,
) -> bool {
    value.is_empty()
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationCumulativeUsage {
    #[serde(default)]
    pub(crate) input_tokens: u64,
    #[serde(default)]
    pub(crate) output_tokens: u64,
    #[serde(default)]
    pub(crate) total_tokens: u64,
    #[serde(default)]
    pub(crate) cache_read_tokens: u64,
    #[serde(default)]
    pub(crate) cache_write_tokens: u64,
    #[serde(default)]
    pub(crate) reasoning_tokens: u64,
    #[serde(default, skip_serializing_if = "conversation_provider_model_usage_is_empty")]
    pub(crate) by_provider_model:
        std::collections::BTreeMap<String, std::collections::BTreeMap<String, ConversationUsageBucket>>,
}

impl ConversationCumulativeUsage {
    pub(crate) fn needs_legacy_total_tokens_backfill(&self) -> bool {
        self.total_tokens < self.input_tokens.saturating_add(self.output_tokens)
            || self
                .by_provider_model
                .values()
                .any(|models| models.values().any(ConversationUsageBucket::needs_legacy_total_tokens_backfill))
    }

    pub(crate) fn normalized_legacy_totals(mut self) -> Self {
        let floor_total_tokens = self.input_tokens.saturating_add(self.output_tokens);
        self.total_tokens = self.total_tokens.max(floor_total_tokens);
        self.by_provider_model = self
            .by_provider_model
            .into_iter()
            .map(|(provider_key, models)| {
                let normalized_models = models
                    .into_iter()
                    .map(|(model_name, bucket)| {
                        (model_name, bucket.normalized_legacy_totals())
                    })
                    .collect();
                (provider_key, normalized_models)
            })
            .collect();
        self
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.input_tokens == 0
            && self.output_tokens == 0
            && self.total_tokens == 0
            && self.cache_read_tokens == 0
            && self.cache_write_tokens == 0
            && self.reasoning_tokens == 0
            && self.by_provider_model.is_empty()
    }

    pub(crate) fn keep_at_least(&mut self, other: &ConversationCumulativeUsage) {
        self.input_tokens = self.input_tokens.max(other.input_tokens);
        self.output_tokens = self.output_tokens.max(other.output_tokens);
        self.total_tokens = self.total_tokens.max(other.total_tokens);
        self.cache_read_tokens = self.cache_read_tokens.max(other.cache_read_tokens);
        self.cache_write_tokens = self.cache_write_tokens.max(other.cache_write_tokens);
        self.reasoning_tokens = self.reasoning_tokens.max(other.reasoning_tokens);
        for (provider_key, other_models) in &other.by_provider_model {
            let target_models = self
                .by_provider_model
                .entry(provider_key.clone())
                .or_default();
            for (model_name, other_bucket) in other_models {
                target_models
                    .entry(model_name.clone())
                    .or_default()
                    .keep_at_least(other_bucket);
            }
        }
    }

    pub(crate) fn detailed_usage_sum(&self) -> ConversationUsageBucket {
        let mut sum = ConversationUsageBucket::default();
        for models in self.by_provider_model.values() {
            for bucket in models.values() {
                sum.saturating_add_assign(bucket);
            }
        }
        sum
    }

    pub(crate) fn legacy_remainder(&self) -> ConversationUsageBucket {
        ConversationUsageBucket::saturating_sub_from_totals(
            self.input_tokens,
            self.output_tokens,
            self.total_tokens,
            self.cache_read_tokens,
            self.cache_write_tokens,
            self.reasoning_tokens,
            &self.detailed_usage_sum(),
        )
    }
}

pub(crate) fn conversation_cumulative_usage_add_provider_usage(
    target: &mut ConversationCumulativeUsage,
    provider_key: Option<&str>,
    model_name: Option<&str>,
    usage: &Value,
) -> bool {
    fn read_u64(usage: &Value, keys: &[&str]) -> u64 {
        keys.iter()
            .find_map(|key| {
                let value = usage.get(*key)?;
                value
                    .as_u64()
                    .or_else(|| value.as_i64().and_then(|item| u64::try_from(item).ok()))
            })
            .unwrap_or(0)
    }

    let input_tokens = read_u64(usage, &["promptTokens", "prompt_tokens"]);
    let output_tokens = read_u64(usage, &["completionTokens", "completion_tokens"]);
    let total_tokens = read_u64(usage, &["totalTokens", "total_tokens"])
        .max(input_tokens.saturating_add(output_tokens));
    let delta = ConversationUsageBucket {
        input_tokens,
        output_tokens,
        total_tokens,
        cache_read_tokens: read_u64(usage, &["cachedTokens", "cached_tokens"]),
        cache_write_tokens: read_u64(
            usage,
            &["cacheCreationTokens", "cache_creation_tokens"],
        )
        .saturating_add(read_u64(
            usage,
            &["cacheCreation5mTokens", "cache_creation_5m_tokens"],
        ))
        .saturating_add(read_u64(
            usage,
            &["cacheCreation1hTokens", "cache_creation_1h_tokens"],
        )),
        reasoning_tokens: read_u64(usage, &["reasoningTokens", "reasoning_tokens"]),
    };
    if delta.is_empty() {
        return false;
    }
    target.input_tokens = target.input_tokens.saturating_add(delta.input_tokens);
    target.output_tokens = target.output_tokens.saturating_add(delta.output_tokens);
    target.total_tokens = target.total_tokens.saturating_add(delta.total_tokens);
    target.cache_read_tokens = target
        .cache_read_tokens
        .saturating_add(delta.cache_read_tokens);
    target.cache_write_tokens = target
        .cache_write_tokens
        .saturating_add(delta.cache_write_tokens);
    target.reasoning_tokens = target
        .reasoning_tokens
        .saturating_add(delta.reasoning_tokens);
    let normalized_provider_key = provider_key
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let normalized_model_name = model_name
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let (Some(provider_key), Some(model_name)) = (normalized_provider_key, normalized_model_name)
    {
        let models = target
            .by_provider_model
            .entry(provider_key.to_string())
            .or_default();
        models
            .entry(model_name.to_string())
            .or_default()
            .saturating_add_assign(&delta);
    }
    true
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationGoalState {
    pub(crate) goal_id: String,
    pub(crate) status: String,
    pub(crate) objective: String,
    pub(crate) started_at: String,
    #[serde(default)]
    pub(crate) ended_at: Option<String>,
    #[serde(default)]
    pub(crate) usage_start: ConversationCumulativeUsage,
    #[serde(default)]
    pub(crate) usage_end: Option<ConversationCumulativeUsage>,
}

pub(crate) fn conversation_goal_is_active(goal: &ConversationGoalState) -> bool {
    goal.status.trim() == "active"
}

pub(crate) fn conversation_cumulative_usage_weighted_tokens(
    cumulative_usage: &ConversationCumulativeUsage,
) -> u64 {
    let weighted = (cumulative_usage.output_tokens as f64 * 2.0)
        + (cumulative_usage.cache_read_tokens as f64 * 0.02)
        + cumulative_usage.cache_write_tokens as f64;
    if !weighted.is_finite() || weighted <= 0.0 {
        0
    } else if weighted >= u64::MAX as f64 {
        u64::MAX
    } else {
        weighted.round() as u64
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Conversation {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) agent_id: String,
    #[serde(default)]
    pub(crate) department_id: String,
    #[serde(default)]
    pub(crate) bound_conversation_id: Option<String>,
    #[serde(default)]
    pub(crate) parent_conversation_id: Option<String>,
    #[serde(default)]
    pub(crate) child_conversation_ids: Vec<String>,
    #[serde(default)]
    pub(crate) fork_message_cursor: Option<String>,
    #[serde(default)]
    pub(crate) unread_count: usize,
    #[serde(default)]
    pub(crate) conversation_kind: String,
    #[serde(default)]
    pub(crate) root_conversation_id: Option<String>,
    #[serde(default)]
    pub(crate) delegate_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[serde(default)]
    pub(crate) last_user_at: Option<String>,
    pub(crate) last_assistant_at: Option<String>,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) summary: String,
    #[serde(default)]
    pub(crate) user_profile_snapshot: String,
    #[serde(default)]
    pub(crate) shell_workspace_path: Option<String>,
    #[serde(default)]
    pub(crate) shell_workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default)]
    pub(crate) shell_autonomous_mode: bool,
    #[serde(default = "default_shell_work_mode")]
    pub(crate) shell_work_mode: String,
    #[serde(default)]
    pub(crate) archived_at: Option<String>,
    pub(crate) messages: Vec<ChatMessage>,
    #[serde(default)]
    pub(crate) fast_request_turns: Vec<FastRequestTurn>,
    #[serde(default)]
    pub(crate) current_todos: Vec<ConversationTodoItem>,
    #[serde(default)]
    pub(crate) memory_recall_table: Vec<String>,
    #[serde(default)]
    pub(crate) plan_mode_enabled: bool,
    #[serde(default)]
    pub(crate) preferred_api_config_id: Option<String>,
    #[serde(default)]
    pub(crate) auto_push_remote_contact_id: Option<String>,
    #[serde(default, alias = "usageSummary")]
    pub(crate) cumulative_usage: ConversationCumulativeUsage,
    #[serde(default)]
    pub(crate) active_goal: Option<ConversationGoalState>,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImConversationAssistantContext {
    pub(crate) department_id: String,
    pub(crate) department_name: String,
    pub(crate) agent_id: String,
    pub(crate) agent_name: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ConversationRuntimeSlot {
    pub(crate) state: MainSessionState,
    pub(crate) pending_queue: std::collections::VecDeque<ChatPendingEvent>,
    pub(crate) stream_cache: ConversationStreamRuntimeCache,
    pub(crate) active_remote_im_activation_sources: Vec<RemoteImActivationSource>,
    pub(crate) active_remote_im_assistant_context: Option<RemoteImConversationAssistantContext>,
    pub(crate) plan_mode_enabled: bool,
    pub(crate) last_activity_at: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ConversationStreamRuntimeCache {
    pub(crate) activation_id: String,
    pub(crate) request_id: String,
    pub(crate) department_id: String,
    pub(crate) agent_id: String,
    pub(crate) assistant_text: String,
    pub(crate) activity_reasoning_text: String,
    pub(crate) tool_status_text: String,
    pub(crate) tool_status_state: String,
    pub(crate) stream_blocks: Vec<AssistantStreamBlock>,
    pub(crate) started_at: String,
    pub(crate) started_at_ms: u64,
    pub(crate) updated_at: String,
    pub(crate) persisted_assistant_message_id: String,
    // 上下文用量随流式缓存下发：工具执行期间落盘用量后写入，
    // 前端随每个 delta 事件的 stream_cache 拿到最新准确占用率，
    // 切屏恢复时也直接来自缓存，无需旁路广播。
    pub(crate) context_usage_ratio: f64,
    pub(crate) context_usage_percent: u32,
    pub(crate) effective_prompt_tokens: u64,
    pub(crate) context_window_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantStreamToolBlock {
    pub(crate) tool_call_id: String,
    pub(crate) name: String,
    pub(crate) args_text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) result_text: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssistantStreamBlock {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) reasoning: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub(crate) text: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tools: Vec<AssistantStreamToolBlock>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub(crate) pending_text_break: bool,
}

impl Default for ConversationRuntimeSlot {
    fn default() -> Self {
        Self {
            state: MainSessionState::Idle,
            pending_queue: std::collections::VecDeque::new(),
            stream_cache: ConversationStreamRuntimeCache::default(),
            active_remote_im_activation_sources: Vec::new(),
            active_remote_im_assistant_context: None,
            plan_mode_enabled: false,
            last_activity_at: String::new(),
        }
    }
}

#[cfg(test)]
mod conversation_usage_tests {
    use super::*;

    #[test]
    fn conversation_cumulative_usage_should_record_provider_model_breakdown() {
        let mut usage = ConversationCumulativeUsage::default();
        let payload = serde_json::json!({
            "promptTokens": 120,
            "completionTokens": 45,
            "cachedTokens": 20,
            "cacheCreationTokens": 7
        });

        let changed = conversation_cumulative_usage_add_provider_usage(
            &mut usage,
            Some("openai"),
            Some("gpt-5"),
            &payload,
        );

        assert!(changed);
        assert_eq!(usage.input_tokens, 120);
        assert_eq!(usage.output_tokens, 45);
        assert_eq!(usage.cache_read_tokens, 20);
        assert_eq!(usage.cache_write_tokens, 7);
        assert_eq!(
            usage
                .by_provider_model
                .get("openai")
                .and_then(|models| models.get("gpt-5"))
                .cloned(),
            Some(ConversationUsageBucket {
                input_tokens: 120,
                output_tokens: 45,
                total_tokens: 165,
                cache_read_tokens: 20,
                cache_write_tokens: 7,
                reasoning_tokens: 0,
            })
        );
    }

    #[test]
    fn conversation_cumulative_usage_legacy_remainder_should_subtract_breakdown_sum() {
        let mut usage = ConversationCumulativeUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_read_tokens: 50,
            cache_write_tokens: 25,
            ..ConversationCumulativeUsage::default()
        };
        usage.by_provider_model.insert(
            "openai".to_string(),
            std::collections::BTreeMap::from([(
                "gpt-5".to_string(),
                ConversationUsageBucket {
                    input_tokens: 120,
                    output_tokens: 60,
                    total_tokens: 180,
                    cache_read_tokens: 20,
                    cache_write_tokens: 5,
                    reasoning_tokens: 0,
                },
            )]),
        );

        let remainder = usage.legacy_remainder();

        assert_eq!(remainder.input_tokens, 80);
        assert_eq!(remainder.output_tokens, 40);
        assert_eq!(remainder.cache_read_tokens, 30);
        assert_eq!(remainder.cache_write_tokens, 20);
    }
}

#[derive(Debug, Clone)]
pub(crate) struct DelegateRuntimeThread {
    pub(crate) delegate_id: String,
    pub(crate) root_conversation_id: String,
    pub(crate) target_agent_id: String,
    pub(crate) title: String,
    pub(crate) call_stack: Vec<String>,
    pub(crate) parent_chat_session_key: Option<String>,
    pub(crate) archived_at: Option<String>,
    pub(crate) conversation: Conversation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationArchive {
    pub(crate) archive_id: String,
    pub(crate) archived_at: String,
    pub(crate) reason: String,
    pub(crate) source_conversation: Conversation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveSummary {
    pub(crate) archive_id: String,
    pub(crate) archived_at: String,
    pub(crate) title: String,
    pub(crate) message_count: usize,
    pub(crate) api_config_id: String,
    pub(crate) agent_id: String,
}
