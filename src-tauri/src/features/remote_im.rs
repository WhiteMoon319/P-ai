#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImEnqueueInput {
    channel_id: String,
    platform: RemoteImPlatform,
    im_name: String,
    remote_contact_type: String,
    remote_contact_id: String,
    #[serde(default)]
    remote_contact_name: Option<String>,
    sender_id: String,
    sender_name: String,
    #[serde(default)]
    sender_avatar_url: Option<String>,
    #[serde(default)]
    platform_message_id: Option<String>,
    #[serde(default)]
    dingtalk_session_webhook: Option<String>,
    #[serde(default)]
    dingtalk_session_webhook_expired_time: Option<i64>,
    #[serde(default)]
    activate_assistant: Option<bool>,
    session: SessionSelector,
    payload: ChatInputPayload,
}

const FAST_REQUEST_KIND_REMOTE_IM_REPLY_DECISION: &str = "remote_im_reply_decision";

fn provider_meta_string(meta: &Option<Value>, key: &str) -> Option<String> {
    meta.as_ref()
        .and_then(|value| value.get(key))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn provider_meta_i64(meta: &Option<Value>, key: &str) -> Option<i64> {
    let value = meta.as_ref().and_then(|item| item.get(key))?;
    if let Some(v) = value.as_i64() {
        return Some(v);
    }
    value
        .as_str()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .and_then(|raw| raw.parse::<i64>().ok())
}

fn resolve_dingtalk_session_webhook(input: &RemoteImEnqueueInput) -> Option<String> {
    let direct = input
        .dingtalk_session_webhook
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string);
    if direct.is_some() {
        return direct;
    }

    provider_meta_string(&input.payload.provider_meta, "sessionWebhook")
        .or_else(|| provider_meta_string(&input.payload.provider_meta, "dingtalkSessionWebhook"))
}

fn resolve_dingtalk_session_webhook_expired_time(input: &RemoteImEnqueueInput) -> Option<i64> {
    input.dingtalk_session_webhook_expired_time
        .or_else(|| provider_meta_i64(&input.payload.provider_meta, "sessionWebhookExpiredTime"))
        .or_else(|| {
            provider_meta_i64(
                &input.payload.provider_meta,
                "dingtalkSessionWebhookExpiredTime",
            )
        })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImEnqueueResult {
    event_id: String,
    conversation_id: String,
    activate_assistant: bool,
    contact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactAllowSendUpdateInput {
    contact_id: String,
    allow_send: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactAllowSendFilesUpdateInput {
    contact_id: String,
    allow_send_files: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactAllowReceiveUpdateInput {
    contact_id: String,
    allow_receive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactActivationUpdateInput {
    contact_id: String,
    activation_mode: String,
    #[serde(default)]
    activation_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_mute_keywords")]
    mute_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_unmute_keywords")]
    unmute_keywords: Vec<String>,
    #[serde(default = "default_remote_im_contact_patience_seconds")]
    patience_seconds: u64,
    #[serde(default = "default_remote_im_contact_mute_duration_seconds")]
    mute_duration_seconds: u64,
    #[serde(default)]
    activation_cooldown_seconds: u64,
    #[serde(default = "default_remote_im_contact_response_strategy")]
    response_strategy: String,
    #[serde(default = "default_remote_im_contact_response_guidance")]
    response_guidance: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactRemarkUpdateInput {
    contact_id: String,
    #[serde(default)]
    remark_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactRouteModeUpdateInput {
    contact_id: String,
    route_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactDepartmentBindingUpdateInput {
    contact_id: String,
    #[serde(default)]
    department_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactProcessingModeUpdateInput {
    contact_id: String,
    processing_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactDeleteInput {
    contact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactLogsInput {
    contact_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactWorkspaceUpdateInput {
    contact_id: String,
    #[serde(default)]
    shell_workspaces: Vec<ShellWorkspaceConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactConversationSummary {
    contact_id: String,
    conversation_id: String,
    title: String,
    updated_at: String,
    last_message_at: Option<String>,
    message_count: usize,
    channel_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_name: Option<String>,
    channel_enabled: bool,
    platform: RemoteImPlatform,
    contact_display_name: String,
    bound_department_id: Option<String>,
    bound_agent_id: Option<String>,
    processing_mode: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    preview_messages: Vec<ConversationPreviewMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactConversationMessagesInput {
    contact_id: String,
}

pub(crate) fn remote_im_channel_by_id<'a>(
    config: &'a AppConfig,
    channel_id: &str,
) -> Option<&'a RemoteImChannelConfig> {
    config
        .remote_im_channels
        .iter()
        .find(|channel| channel.id == channel_id)
}

fn remote_im_upsert_contact_for_inbound(
    runtime: &mut RuntimeStateFile,
    channel: &RemoteImChannelConfig,
    input: &RemoteImEnqueueInput,
    now: &str,
) -> String {
    let default_allow_receive = remote_im_resolve_inbound_activate(channel, input.activate_assistant);
    if let Some(contact) = runtime.remote_im_contacts.iter_mut().find(|item| {
        item.channel_id == input.channel_id
            && item.remote_contact_type == input.remote_contact_type.trim()
            && item.remote_contact_id == input.remote_contact_id
    }) {
        if let Some(name) = input
            .remote_contact_name
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            contact.remote_contact_name = name.to_string();
        }
        if let Some(avatar_url) = input
            .sender_avatar_url
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
        {
            contact.avatar_url = avatar_url.to_string();
        }
        if matches!(input.platform, RemoteImPlatform::Dingtalk) {
            let session_webhook = resolve_dingtalk_session_webhook(input);
            if session_webhook.is_some() {
                contact.dingtalk_session_webhook = session_webhook;
            }
            let expired_time = resolve_dingtalk_session_webhook_expired_time(input);
            if expired_time.is_some() {
                contact.dingtalk_session_webhook_expired_time = expired_time;
            }
        }
        contact.last_message_at = Some(now.to_string());
        return contact.id.clone();
    }

    let contact_id = Uuid::new_v4().to_string();
    runtime.remote_im_contacts.push(RemoteImContact {
        id: contact_id.clone(),
        channel_id: input.channel_id.clone(),
        platform: input.platform.clone(),
        remote_contact_type: input.remote_contact_type.trim().to_string(),
        remote_contact_id: input.remote_contact_id.trim().to_string(),
        remote_contact_name: input
            .remote_contact_name
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
        avatar_url: input
            .sender_avatar_url
            .as_deref()
            .map(str::trim)
            .unwrap_or("")
            .to_string(),
        remark_name: String::new(),
        allow_send: default_allow_receive,
        allow_send_files: false,
        allow_receive: default_allow_receive,
        activation_mode: "never".to_string(),
        activation_keywords: Vec::new(),
        mute_keywords: default_remote_im_contact_mute_keywords(),
        unmute_keywords: default_remote_im_contact_unmute_keywords(),
        patience_seconds: default_remote_im_contact_patience_seconds(),
        mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
        activation_cooldown_seconds: 0,
        route_mode: "dedicated_contact_conversation".to_string(),
        bound_department_id: Some(REMOTE_CUSTOMER_SERVICE_DEPARTMENT_ID.to_string()),
        bound_agent_id: None,
        bound_conversation_id: None,
        processing_mode: "continuous".to_string(),
        response_strategy: default_remote_im_contact_response_strategy_for_type(
            input.remote_contact_type.as_str(),
        ),
        response_guidance: default_remote_im_contact_response_guidance(),
        last_activated_at: None,
        last_message_at: Some(now.to_string()),
        dingtalk_session_webhook: if matches!(input.platform, RemoteImPlatform::Dingtalk) {
            resolve_dingtalk_session_webhook(input)
        } else {
            None
        },
        dingtalk_session_webhook_expired_time: if matches!(input.platform, RemoteImPlatform::Dingtalk)
        {
            resolve_dingtalk_session_webhook_expired_time(input)
        } else {
            None
        },
        onebot_group_members: Vec::new(),
        shell_workspaces: Vec::new(),
    });
    contact_id
}

fn normalize_contact_activation_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "always" | "keyword" => value.trim().to_ascii_lowercase(),
        "never" => "never".to_string(),
        _ => "never".to_string(),
    }
}

fn normalize_contact_keyword_list(values: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for value in values {
        for segment in value.split(|ch| matches!(ch, ',' | '，' | '\n' | '\r')) {
            let trimmed = segment.trim();
            if trimmed.is_empty() {
                continue;
            }
            if !seen.insert(trimmed.to_string()) {
                continue;
            }
            out.push(trimmed.to_string());
        }
    }
    out
}

fn normalize_contact_activation_keywords(values: &[String]) -> Vec<String> {
    normalize_contact_keyword_list(values)
}

fn normalize_contact_route_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "dedicated_contact_conversation" => "dedicated_contact_conversation".to_string(),
        _ => "dedicated_contact_conversation".to_string(),
    }
}

fn normalize_contact_processing_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "qa" => "qa".to_string(),
        _ => "continuous".to_string(),
    }
}

fn normalize_contact_response_strategy(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "smart_judge" => "smart_judge".to_string(),
        _ => "always_reply".to_string(),
    }
}

fn remote_im_contact_is_private(contact: &RemoteImContact) -> bool {
    contact
        .remote_contact_type
        .trim()
        .eq_ignore_ascii_case("private")
}

fn default_remote_im_contact_response_strategy_for_type(remote_contact_type: &str) -> String {
    if remote_contact_type.trim().eq_ignore_ascii_case("private") {
        "always_reply".to_string()
    } else {
        "smart_judge".to_string()
    }
}

fn effective_remote_im_contact_response_strategy(contact: &RemoteImContact) -> String {
    if remote_im_contact_is_private(contact) {
        "always_reply".to_string()
    } else {
        normalize_contact_response_strategy(&contact.response_strategy)
    }
}

fn normalize_contact_response_guidance(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_remote_im_contact_response_guidance()
    } else {
        trimmed.to_string()
    }
}

include!("remote_im/reply_debounce.rs");

include!("remote_im/reply_delegate.rs");

fn remote_im_contact_runtime_state_mut<'a>(
    states: &'a mut std::collections::HashMap<String, RemoteImContactRuntimeState>,
    contact_id: &str,
) -> &'a mut RemoteImContactRuntimeState {
    states
        .entry(contact_id.to_string())
        .or_insert_with(RemoteImContactRuntimeState::default)
}

fn remote_im_contact_checkpoint_mut_in_list<'a>(
    checkpoints: &'a mut Vec<RemoteImContactCheckpoint>,
    contact_id: &str,
) -> &'a mut RemoteImContactCheckpoint {
    if let Some(index) = checkpoints
        .iter()
        .position(|item| item.contact_id == contact_id)
    {
        return &mut checkpoints[index];
    }
    checkpoints.push(RemoteImContactCheckpoint {
        contact_id: contact_id.to_string(),
        ..RemoteImContactCheckpoint::default()
    });
    let last_index = checkpoints.len().saturating_sub(1);
    &mut checkpoints[last_index]
}

fn remote_im_contact_by_source_in_runtime<'a>(
    contacts: &'a [RemoteImContact],
    source: &RemoteImMessageSource,
) -> Option<&'a RemoteImContact> {
    contacts.iter().find(|item| {
        item.channel_id == source.channel_id
            && item.remote_contact_type == source.remote_contact_type
            && item.remote_contact_id == source.remote_contact_id
    })
}

fn remote_im_contact_by_activation_source_in_runtime<'a>(
    contacts: &'a [RemoteImContact],
    source: &RemoteImActivationSource,
) -> Option<&'a RemoteImContact> {
    contacts.iter().find(|item| {
        item.channel_id == source.channel_id
            && item.remote_contact_type == source.remote_contact_type
            && item.remote_contact_id == source.remote_contact_id
    })
}

fn remote_im_contact_matches_reply_target(
    source: &RemoteImActivationSource,
    target: &RemoteImReplyTarget,
) -> bool {
    source.channel_id.trim() == target.channel_id.trim()
        && source.remote_contact_id.trim() == target.contact_id.trim()
}

fn remote_im_text_contains_keyword(text: &str, keyword: &str) -> bool {
    text.to_ascii_lowercase()
        .contains(&keyword.to_ascii_lowercase())
}

fn remote_im_find_matched_keyword<'a>(text: &str, keywords: &'a [String]) -> Option<&'a str> {
    keywords
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .find(|keyword| remote_im_text_contains_keyword(text, keyword))
}

fn remote_im_keyword_matched(contact: &RemoteImContact, message_text: &str) -> bool {
    remote_im_find_matched_keyword(message_text, &contact.activation_keywords).is_some()
}

fn remote_im_resolve_mute_until(now: time::OffsetDateTime, duration_seconds: u64) -> String {
    normalize_time_for_utc_storage(
        now + time::Duration::seconds(duration_seconds.min(i64::MAX as u64) as i64),
    )
    .unwrap_or_else(|_| now_iso())
}

fn remote_im_is_mute_expired(mute_until: &str, now: time::OffsetDateTime) -> bool {
    parse_iso(mute_until).map(|value| value <= now).unwrap_or(true)
}

fn remote_im_should_activate_while_away(
    contact: &RemoteImContact,
    message_text: &str,
) -> (bool, String) {
    match contact.activation_mode.trim().to_ascii_lowercase().as_str() {
        "always" => (true, "away 命中 always，切换为在场".to_string()),
        "keyword" => {
            let matched = remote_im_keyword_matched(contact, message_text);
            if matched {
                (true, "away 命中 keyword，切换为在场".to_string())
            } else {
                (false, "away 未命中 keyword，仅记录消息".to_string())
            }
        }
        _ => (false, "away 命中 never，仅记录消息".to_string()),
    }
}

fn remote_im_prepare_enqueue_runtime_state(
    state: &AppState,
    contact: &RemoteImContact,
    message_text: &str,
) -> Result<(bool, String), String> {
    if contact
        .remote_contact_type
        .trim()
        .eq_ignore_ascii_case("private")
    {
        return Ok((
            true,
            "私聊禁用秘书与在场状态，消息直接调度绑定会话".to_string(),
        ));
    }
    let mut runtime_states = lock_remote_im_contact_runtime_states(state)?;
    let runtime = remote_im_contact_runtime_state_mut(&mut runtime_states, &contact.id);
    let previous_presence = runtime.presence_state;
    let previous_work = runtime.work_state;
    let previous_pending = runtime.has_pending;
    let now = now_utc();
    let mut mute_prefix = String::new();
    let supports_mute = contact
        .remote_contact_type
        .trim()
        .eq_ignore_ascii_case("group");
    if !supports_mute {
        runtime.mute_until = None;
    }
    if supports_mute {
        if let Some(mute_until) = runtime.mute_until.clone() {
        if remote_im_is_mute_expired(&mute_until, now) {
            runtime.mute_until = None;
            mute_prefix = format!("闭嘴超时自动解除(截止={mute_until})；");
        }
        }
    }
    if supports_mute {
        if let Some(keyword) = remote_im_find_matched_keyword(message_text, &contact.mute_keywords) {
        let mute_until = remote_im_resolve_mute_until(now, contact.mute_duration_seconds);
        runtime.mute_until = Some(mute_until.clone());
        let reason = format!(
            "{}命中闭嘴词“{}”，进入闭嘴直到 {}，直接拦截后续判定",
            mute_prefix, keyword, mute_until
        );
        runtime_log_info(format!(
            "[远程联系人状态机] 入站判定 完成: contact_id={}, presence={:?}, work={:?}, pending={}, activate_assistant={}, reason={}",
            contact.id,
            runtime.presence_state,
            runtime.work_state,
            runtime.has_pending,
            false,
            reason
        ));
        remote_im_append_channel_log(
            &contact.channel_id,
            "info",
            format!(
                "[联系人状态] 入站判定: contact={}, presence={} -> {}, work={} -> {}, pending={} -> {}, activate={}, reason={}",
                remote_im_contact_log_label(contact),
                remote_im_presence_state_label(previous_presence),
                remote_im_presence_state_label(runtime.presence_state),
                remote_im_work_state_label(previous_work),
                remote_im_work_state_label(runtime.work_state),
                remote_im_yes_no(previous_pending),
                remote_im_yes_no(runtime.has_pending),
                remote_im_yes_no(false),
                reason
            ),
        );
        return Ok((false, reason));
        }
    }
    if supports_mute && runtime.mute_until.is_some() {
        if let Some(keyword) = remote_im_find_matched_keyword(message_text, &contact.unmute_keywords) {
            runtime.mute_until = None;
            mute_prefix.push_str(&format!("命中张嘴词“{}”，解除闭嘴；", keyword));
        } else {
            let mute_until = runtime.mute_until.clone().unwrap_or_default();
            let reason = format!(
                "{}当前仍处于闭嘴期(截止={})，未命中张嘴词，直接拦截后续判定",
                mute_prefix, mute_until
            );
            runtime_log_info(format!(
                "[远程联系人状态机] 入站判定 完成: contact_id={}, presence={:?}, work={:?}, pending={}, activate_assistant={}, reason={}",
                contact.id,
                runtime.presence_state,
                runtime.work_state,
                runtime.has_pending,
                false,
                reason
            ));
            remote_im_append_channel_log(
                &contact.channel_id,
                "info",
                format!(
                    "[联系人状态] 入站判定: contact={}, presence={} -> {}, work={} -> {}, pending={} -> {}, activate={}, reason={}",
                    remote_im_contact_log_label(contact),
                    remote_im_presence_state_label(previous_presence),
                    remote_im_presence_state_label(runtime.presence_state),
                    remote_im_work_state_label(previous_work),
                    remote_im_work_state_label(runtime.work_state),
                    remote_im_yes_no(previous_pending),
                    remote_im_yes_no(runtime.has_pending),
                    remote_im_yes_no(false),
                    reason
                ),
            );
            return Ok((false, reason));
        }
    }
    let (activate_assistant, reason) = match runtime.presence_state {
        RemoteImPresenceState::Away => {
            let (activate, reason) = remote_im_should_activate_while_away(contact, message_text);
            (
                activate,
                format!("{mute_prefix}{reason}；消息先落库，等待秘书决定后才进入在场"),
            )
        }
        RemoteImPresenceState::Present => {
            (
                true,
                format!("{mute_prefix}present，消息先落库后交由秘书决定引导或新建并发委托"),
            )
        }
    };
    runtime_log_info(format!(
        "[远程联系人状态机] 入站判定 完成: contact_id={}, presence={:?}, work={:?}, pending={}, activate_assistant={}, reason={}",
        contact.id,
        runtime.presence_state,
        runtime.work_state,
        runtime.has_pending,
        activate_assistant,
        reason
    ));
    remote_im_append_channel_log(
        &contact.channel_id,
        "info",
        format!(
            "[联系人状态] 入站判定: contact={}, presence={} -> {}, work={} -> {}, pending={} -> {}, activate={}, reason={}",
            remote_im_contact_log_label(contact),
            remote_im_presence_state_label(previous_presence),
            remote_im_presence_state_label(runtime.presence_state),
            remote_im_work_state_label(previous_work),
            remote_im_work_state_label(runtime.work_state),
            remote_im_yes_no(previous_pending),
            remote_im_yes_no(runtime.has_pending),
            remote_im_yes_no(activate_assistant),
            reason
        ),
    );
    Ok((activate_assistant, reason))
}

include!("remote_im/secretary_decision.rs");

include!("remote_im/round_completion.rs");

include!("remote_im/message_routing.rs");

include!("remote_im/inbound_message.rs");

include!("remote_im/contact_commands.rs");
