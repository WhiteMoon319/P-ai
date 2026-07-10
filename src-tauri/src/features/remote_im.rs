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
const FAST_REQUEST_KIND_REMOTE_IM_INTERRUPT_DECISION: &str = "remote_im_interrupt_decision";

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
        response_strategy: default_remote_im_contact_response_strategy(),
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

fn normalize_contact_response_guidance(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        default_remote_im_contact_response_guidance()
    } else {
        trimmed.to_string()
    }
}

fn lock_remote_im_contact_runtime_states(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, std::collections::HashMap<String, RemoteImContactRuntimeState>>, String>
{
    state
        .remote_im_contact_runtime_states
        .lock()
        .map_err(|_| "无法获取远程 IM 联系人运行时状态的锁".to_string())
}

fn lock_remote_im_reply_delegate_runtimes(
    state: &AppState,
) -> Result<
    std::sync::MutexGuard<'_, std::collections::HashMap<String, RemoteImReplyDelegateRuntime>>,
    String,
> {
    state
        .remote_im_reply_delegate_runtimes
        .lock()
        .map_err(|_| "无法获取远程应答委托运行时状态的锁".to_string())
}

fn remote_im_reply_delegate_register(
    state: &AppState,
    contact_id: &str,
    conversation_id: &str,
    trigger_message: &ChatMessage,
    session_info: &ChatSessionInfo,
    force_memory_prompt_snapshot: bool,
) -> Result<String, String> {
    let trigger_message_id = trigger_message.id.trim();
    if trigger_message_id.is_empty() {
        return Err("远程应答委托无法冻结启动快照：触发消息 ID 为空".to_string());
    }
    let mut prompt_snapshot_messages = if force_memory_prompt_snapshot {
        runtime_log_warn(format!(
            "[远程应答委托] 跳过，任务=读取启动 block，reason=dynamic_wake_persistence_failed，conversation_id={}，message_id={}",
            conversation_id, trigger_message_id
        ));
        vec![trigger_message.clone()]
    } else { match conversation_service_v2()
        .get_conversation_prompt_context(state, conversation_id)
    {
        Ok(conversation) => conversation.messages,
        Err(err) => {
            runtime_log_error(format!(
                "[远程应答委托] 失败，任务=读取启动 block，改用触发消息内存快照，conversation_id={}，message_id={}，error={}",
                conversation_id, trigger_message_id, err
            ));
            vec![trigger_message.clone()]
        }
    }};
    let trigger_position = prompt_snapshot_messages
        .iter()
        .position(|message| message.id == trigger_message_id);
    if let Some(trigger_position) = trigger_position {
        // 同批后续事件已经先落库时，快照仍必须止于本委托的触发消息。
        prompt_snapshot_messages.truncate(trigger_position.saturating_add(1));
    } else if prompt_snapshot_messages.len() != 1
        || prompt_snapshot_messages.first().map(|message| message.id.as_str())
            != Some(trigger_message_id)
    {
        runtime_log_error(format!(
            "[远程应答委托] 失败，任务=触发消息不在启动 block，改用触发消息内存快照，conversation_id={}，message_id={}",
            conversation_id, trigger_message_id
        ));
        prompt_snapshot_messages = vec![trigger_message.clone()];
    }
    let root_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let delegate = delegate_store_create_delegate(
        &state.data_path,
        &DelegateCreateInput {
            kind: "remote_im_reply".to_string(),
            conversation_id: conversation_id.to_string(),
            parent_delegate_id: None,
            source_department_id: session_info.department_id.clone(),
            target_department_id: session_info.department_id.clone(),
            source_agent_id: session_info.agent_id.clone(),
            target_agent_id: session_info.agent_id.clone(),
            title: format!("远程应答 · {}", contact_id),
            why: "远程联系人消息触发应答".to_string(),
            goal: "根据冻结上下文回复远程联系人".to_string(),
            todo: "生成并发送远程应答".to_string(),
            notify_assistant_when_done: false,
            call_stack: Vec::new(),
        },
    )?;
    let delegate_id = delegate.delegate_id.clone();
    if let Err(err) = delegate_runtime_thread_create(
        state,
        &delegate,
        root_meta.preferred_api_config_id.as_deref().unwrap_or_default(),
        None,
        None,
    ) {
        let _ = delegate_store_update_status(&state.data_path, &delegate_id, DELEGATE_STATUS_FAILED);
        return Err(format!("创建远程应答委托会话失败: {err}"));
    }
    let runtime = RemoteImReplyDelegateRuntime {
        delegate_id: delegate_id.clone(),
        contact_id: contact_id.to_string(),
        conversation_id: conversation_id.to_string(),
        trigger_message_id: trigger_message_id.to_string(),
        started_at: now_iso(),
        prompt_snapshot_messages,
        guidance_messages: std::collections::VecDeque::new(),
        consumed_guidance_messages: Vec::new(),
        cancelled: false,
        terminal: false,
        session_agent_id: session_info.agent_id.clone(),
    };
    lock_remote_im_reply_delegate_runtimes(state)?.insert(delegate_id.clone(), runtime);
    if let Err(err) = remote_im_reply_delegate_mirror_internal_messages(
        state,
        &delegate_id,
        "frozen_snapshot",
        &remote_im_reply_delegate_prompt_messages(state, &delegate_id)?,
    ) {
        let _ = remote_im_reply_delegate_finish(
            state,
            &delegate_id,
            DELEGATE_STATUS_FAILED,
            "写入远程应答冻结快照失败",
        );
        return Err(err);
    }
    Ok(delegate_id)
}

fn remote_im_reply_delegate_prompt_messages(
    state: &AppState,
    delegate_id: &str,
) -> Result<Vec<ChatMessage>, String> {
    let runtimes = lock_remote_im_reply_delegate_runtimes(state)?;
    let runtime = runtimes
        .get(delegate_id)
        .ok_or_else(|| format!("远程应答委托不存在，delegate_id={delegate_id}"))?;
    if runtime.cancelled || runtime.terminal {
        return Err(format!("远程应答委托已结束，delegate_id={delegate_id}"));
    }
    let mut messages = runtime.prompt_snapshot_messages.clone();
    messages.extend(runtime.consumed_guidance_messages.iter().cloned());
    Ok(messages)
}

fn remote_im_reply_delegate_is_active(state: &AppState, delegate_id: &str) -> bool {
    lock_remote_im_reply_delegate_runtimes(state)
        .ok()
        .and_then(|runtimes| runtimes.get(delegate_id).cloned())
        .map(|runtime| !runtime.cancelled && !runtime.terminal)
        .unwrap_or(false)
}

fn remote_im_reply_delegate_enqueue_guidance(
    state: &AppState,
    delegate_id: &str,
    message: ChatMessage,
) -> Result<(), String> {
    {
        let mut runtimes = lock_remote_im_reply_delegate_runtimes(state)?;
        let runtime = runtimes
            .get_mut(delegate_id)
            .ok_or_else(|| format!("远程应答委托不存在，delegate_id={delegate_id}"))?;
        if runtime.cancelled || runtime.terminal {
            return Err(format!("远程应答委托已结束，delegate_id={delegate_id}"));
        }
        runtime.guidance_messages.push_back(message.clone());
    }
    if let Err(err) = remote_im_reply_delegate_mirror_internal_messages(
        state,
        delegate_id,
        "guidance",
        &[message],
    ) {
        runtime_log_warn(format!(
            "[远程应答委托] 失败，任务=镜像秘书引导，delegate_id={}，error={}",
            delegate_id, err
        ));
    }
    Ok(())
}

/// 在同一把锁内消费引导，或在确认队列为空时注销委托。
/// 这样秘书不会在“最后一次读空”和“删除运行态”之间塞入一条永远不会被消费的消息。
fn remote_im_reply_delegate_take_guidance_or_finish(
    state: &AppState,
    delegate_id: &str,
) -> Result<RemoteImReplyDelegateNext, String> {
    let mut runtimes = lock_remote_im_reply_delegate_runtimes(state)?;
    let completed_runtime = {
        let runtime = runtimes
            .get_mut(delegate_id)
            .ok_or_else(|| format!("远程应答委托不存在，delegate_id={delegate_id}"))?;
        if runtime.cancelled || runtime.terminal {
            return Ok(RemoteImReplyDelegateNext::Ended);
        }
        if runtime.guidance_messages.is_empty() {
            runtime.terminal = true;
            Some(runtime.clone())
        } else {
            let messages = runtime.guidance_messages.drain(..).collect::<Vec<_>>();
            runtime.consumed_guidance_messages.extend(messages.iter().cloned());
            return Ok(RemoteImReplyDelegateNext::Guidance(messages));
        }
    };
    if let Some(runtime) = completed_runtime {
        runtimes.remove(delegate_id);
        Ok(RemoteImReplyDelegateNext::Completed(runtime))
    } else {
        Ok(RemoteImReplyDelegateNext::Ended)
    }
}

enum RemoteImReplyDelegateNext {
    Guidance(Vec<ChatMessage>),
    Completed(RemoteImReplyDelegateRuntime),
    Ended,
}

fn remote_im_reply_delegate_active_ids_for_contact(
    state: &AppState,
    contact_id: &str,
) -> Result<Vec<String>, String> {
    let runtimes = lock_remote_im_reply_delegate_runtimes(state)?
        .values()
        .filter(|runtime| runtime.contact_id == contact_id && !runtime.cancelled && !runtime.terminal)
        .cloned()
        .collect::<Vec<_>>();
    for runtime in &runtimes {
        runtime_log_debug(format!(
            "[远程应答委托] 活跃快照，delegate_id={}，conversation_id={}，trigger_message_id={}，started_at={}",
            runtime.delegate_id,
            runtime.conversation_id,
            runtime.trigger_message_id,
            runtime.started_at
        ));
    }
    Ok(runtimes
        .into_iter()
        .map(|runtime| runtime.delegate_id)
        .collect())
}

fn remote_im_reply_delegate_finish(
    state: &AppState,
    delegate_id: &str,
    status: &str,
    reason: &str,
) -> Result<bool, String> {
    let runtime = {
        let mut runtimes = lock_remote_im_reply_delegate_runtimes(state)?;
        let Some(runtime) = runtimes.get_mut(delegate_id) else {
            return Ok(false);
        };
        if runtime.terminal {
            return Ok(false);
        }
        runtime.terminal = true;
        let runtime = runtime.clone();
        runtimes.remove(delegate_id);
        runtime
    };
    remote_im_reply_delegate_finalize(state, runtime, status, reason)?;
    Ok(true)
}

fn remote_im_reply_delegate_finalize(
    state: &AppState,
    runtime: RemoteImReplyDelegateRuntime,
    status: &str,
    reason: &str,
) -> Result<(), String> {
    let archived_at = now_iso();
    delegate_runtime_thread_archive(state, &runtime.delegate_id, &archived_at)?;
    delegate_store_update_status(&state.data_path, &runtime.delegate_id, status)?;
    if let Err(err) = emit_conversation_delegate_status_updated(
        state,
        &runtime.conversation_id,
        &runtime.delegate_id,
        status,
    ) {
        runtime_log_warn(format!(
            "[远程应答委托] 失败，任务=推送终态，delegate_id={}，status={}，error={}",
            runtime.delegate_id, status, err
        ));
    }
    runtime_log_info(format!(
        "[远程应答委托] 完成，任务=终结，delegate_id={}，status={}，reason={}",
        runtime.delegate_id, status, reason
    ));
    Ok(())
}

fn abort_remote_im_reply_delegate(
    state: &AppState,
    delegate_id: &str,
    reason: &str,
) -> Result<bool, String> {
    let runtime = {
        let mut runtimes = lock_remote_im_reply_delegate_runtimes(state)?;
        let Some(runtime) = runtimes.get_mut(delegate_id) else {
            return Ok(false);
        };
        if runtime.terminal {
            return Ok(false);
        }
        runtime.cancelled = true;
        runtime.terminal = true;
        let runtime = runtime.clone();
        runtimes.remove(delegate_id);
        runtime
    };
    let chat_key = format!("remote-im-reply-delegate::{delegate_id}");
    let aborted_chat = {
        let mut inflight = state
            .inflight_chat_abort_handles
            .lock()
            .map_err(|_| "无法获取远程应答聊天取消句柄".to_string())?;
        if let Some(handle) = inflight.remove(&chat_key) {
            handle.abort();
            true
        } else {
            false
        }
    };
    let tool_key = format!(
        "{}::{}::remote_reply_delegate:{}",
        runtime.session_agent_id, runtime.conversation_id, delegate_id
    );
    let aborted_tool = abort_inflight_tool_abort_handle(state, &tool_key)?;
    remote_im_reply_delegate_finalize(state, runtime, DELEGATE_STATUS_FAILED, reason)?;
    runtime_log_info(format!(
        "[远程应答委托] 完成，任务=取消，delegate_id={}，aborted_chat={}，aborted_tool={}，reason={}",
        delegate_id, aborted_chat, aborted_tool, reason
    ));
    Ok(true)
}

fn remote_im_reply_delegate_mirror_message(
    state: &AppState,
    delegate_id: &str,
    mut message: ChatMessage,
    internal_kind: Option<&str>,
) -> Result<(), String> {
    let Some(mut conversation) = delegate_runtime_thread_conversation_get_any(state, delegate_id)? else {
        return Err(format!("远程应答委托会话不存在，delegate_id={delegate_id}"));
    };
    if conversation.messages.iter().any(|item| item.id == message.id) {
        return Ok(());
    }
    if let Some(kind) = internal_kind {
        let mut meta = message.provider_meta.take().unwrap_or_else(|| serde_json::json!({}));
        if !meta.is_object() {
            meta = serde_json::json!({});
        }
        if let Some(object) = meta.as_object_mut() {
            object.insert("remote_im_delegate_internal".to_string(), serde_json::json!(true));
            object.insert("remote_im_delegate_internal_kind".to_string(), serde_json::json!(kind));
        }
        message.provider_meta = Some(meta);
    }
    conversation.messages.push(message);
    conversation.updated_at = now_iso();
    delegate_runtime_thread_conversation_update(state, delegate_id, conversation)
}

fn remote_im_reply_delegate_mirror_internal_messages(
    state: &AppState,
    delegate_id: &str,
    kind: &str,
    messages: &[ChatMessage],
) -> Result<(), String> {
    for message in messages {
        let mut mirrored = message.clone();
        mirrored.id = format!("remote-im-internal-{}-{}-{}", delegate_id, kind, message.id);
        remote_im_reply_delegate_mirror_message(state, delegate_id, mirrored, Some(kind))?;
    }
    Ok(())
}

fn remote_im_mark_contact_present(
    state: &AppState,
    contact_id: &str,
    reason: &str,
) -> Result<(), String> {
    let mut states = lock_remote_im_contact_runtime_states(state)?;
    let runtime = remote_im_contact_runtime_state_mut(&mut states, contact_id);
    runtime.presence_state = RemoteImPresenceState::Present;
    runtime.last_presence_at = Some(now_iso());
    runtime.consecutive_no_reply_count = 0;
    runtime_log_info(format!(
        "[远程联系人在场] 完成，contact_id={}，reason={}",
        contact_id, reason
    ));
    Ok(())
}

fn remote_im_contact_is_away(state: &AppState, contact_id: &str) -> Result<bool, String> {
    Ok(lock_remote_im_contact_runtime_states(state)?
        .get(contact_id)
        .map(|runtime| runtime.presence_state == RemoteImPresenceState::Away)
        .unwrap_or(true))
}

fn remote_im_schedule_presence_timeout(
    state: &AppState,
    contact_id: &str,
    patience_seconds: u64,
) -> Result<(), String> {
    let expected_presence_at = lock_remote_im_contact_runtime_states(state)?
        .get(contact_id)
        .and_then(|runtime| runtime.last_presence_at.clone())
        .unwrap_or_else(now_iso);
    let state_clone = state.clone();
    let contact_id = contact_id.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(std::time::Duration::from_secs(patience_seconds)).await;
        let Ok(mut states) = lock_remote_im_contact_runtime_states(&state_clone) else {
            return;
        };
        let Some(runtime) = states.get_mut(&contact_id) else {
            return;
        };
        if runtime.presence_state == RemoteImPresenceState::Present
            && runtime.last_presence_at.as_deref() == Some(expected_presence_at.as_str())
        {
            runtime.presence_state = RemoteImPresenceState::Away;
            runtime_log_info(format!(
                "[远程联系人在场] 完成，任务=耐心超时离场，contact_id={}，patience_seconds={}",
                contact_id, patience_seconds
            ));
        }
    });
    Ok(())
}

fn spawn_remote_im_reply_delegate(
    state: &AppState,
    contact_id: &str,
    conversation_id: &str,
    trigger_message: &ChatMessage,
    session_info: &ChatSessionInfo,
    source: RemoteImActivationSource,
    patience_seconds: u64,
    dynamic_boundary: bool,
    force_memory_prompt_snapshot: bool,
) -> Result<String, String> {
    let delegate_id = remote_im_reply_delegate_register(
        state,
        contact_id,
        conversation_id,
        trigger_message,
        session_info,
        force_memory_prompt_snapshot,
    )?;
    let state_clone = state.clone();
    let delegate_id_for_task = delegate_id.clone();
    let conversation_id = conversation_id.to_string();
    let trigger_message_id = trigger_message.id.clone();
    let session_info = session_info.clone();
    let contact_id_for_task = contact_id.to_string();
    tauri::async_runtime::spawn(async move {
        let permit = match state_clone
            .remote_im_reply_delegate_semaphore
            .clone()
            .acquire_owned()
            .await
        {
            Ok(value) => value,
            Err(_) => {
                runtime_log_error(format!(
                    "[远程应答委托] 失败，任务=获取并发槽，delegate_id={}",
                    delegate_id_for_task
                ));
                let _ = remote_im_reply_delegate_finish(
                    &state_clone,
                    &delegate_id_for_task,
                    DELEGATE_STATUS_FAILED,
                    "获取远程应答并发槽失败",
                );
                return;
            }
        };
        if !remote_im_reply_delegate_is_active(&state_clone, &delegate_id_for_task) {
            drop(permit);
            return;
        }
        let channel: tauri::ipc::Channel<AssistantDeltaEvent> =
            tauri::ipc::Channel::new(|_| Ok(()));
        let mut terminal_status = DELEGATE_STATUS_COMPLETED;
        let mut terminal_reason = "远程应答完成";
        loop {
            let prompt_snapshot_messages = match remote_im_reply_delegate_prompt_messages(
                &state_clone,
                &delegate_id_for_task,
            ) {
                Ok(messages) => messages,
                Err(err) => {
                    runtime_log_error(format!(
                        "[远程应答委托] 失败，任务=读取私有提示词快照，delegate_id={}，error={}",
                        delegate_id_for_task, err
                    ));
                    terminal_status = DELEGATE_STATUS_FAILED;
                    terminal_reason = "读取远程应答上下文失败";
                    break;
                }
            };
            let request = SendChatRequest {
                payload: ChatInputPayload {
                    text: None,
                    display_text: None,
                    images: None,
                    audios: None,
                    attachments: None,
                    model: None,
                    extra_text_blocks: None,
                    mentions: None,
                    provider_meta: None,
                },
                session: Some(SessionSelector {
                    api_config_id: None,
                    department_id: Some(session_info.department_id.clone()),
                    agent_id: session_info.agent_id.clone(),
                    conversation_id: Some(conversation_id.clone()),
                }),
                speaker_agent_id: None,
                trace_id: Some(format!("remote-reply-{}", delegate_id_for_task)),
                assistant_message_id: Some(Uuid::new_v4().to_string()),
                oldest_queue_created_at: None,
                remote_im_activation_sources: vec![source.clone()],
                runtime_context: Some(RuntimeContext {
                    event_source: Some("remote_im_reply_delegate".to_string()),
                    dispatch_reason: Some("remote_im_reply_delegate".to_string()),
                    bound_remote_im_activation_source: Some(source.clone()),
                    remote_im_reply_delegate_id: Some(delegate_id_for_task.clone()),
                    remote_im_reply_trigger_message_id: Some(trigger_message_id.to_string()),
                    remote_im_reply_prompt_snapshot_messages: Some(prompt_snapshot_messages),
                    remote_im_dynamic_boundary: dynamic_boundary,
                    ..RuntimeContext::default()
                }),
                trigger_only: true,
            };
            match send_chat_message_inner(request, &state_clone, &channel).await {
                Ok(_) => {
                    let _ = remote_im_mark_contact_present(
                        &state_clone,
                        &contact_id_for_task,
                        "远程应答委托已产生模型回答",
                    );
                    let _ = remote_im_schedule_presence_timeout(
                        &state_clone,
                        &contact_id_for_task,
                        patience_seconds,
                    );
                    runtime_log_info(format!(
                        "[远程应答委托] 完成一轮，delegate_id={}，conversation_id={}",
                        delegate_id_for_task, conversation_id
                    ));
                }
                Err(err) => {
                    runtime_log_error(format!(
                        "[远程应答委托] 失败，delegate_id={}，conversation_id={}，error={}",
                        delegate_id_for_task, conversation_id, err
                    ));
                    terminal_status = DELEGATE_STATUS_FAILED;
                    terminal_reason = "远程应答模型执行失败";
                    break;
                }
            }
            match remote_im_reply_delegate_take_guidance_or_finish(&state_clone, &delegate_id_for_task) {
                Ok(RemoteImReplyDelegateNext::Ended) => break,
                Ok(RemoteImReplyDelegateNext::Completed(runtime)) => {
                    if let Err(err) = remote_im_reply_delegate_finalize(
                        &state_clone,
                        runtime,
                        DELEGATE_STATUS_COMPLETED,
                        "远程应答完成",
                    ) {
                        runtime_log_warn(format!(
                            "[远程应答委托] 失败，任务=终结，delegate_id={}，error={}",
                            delegate_id_for_task, err
                        ));
                    }
                    break;
                }
                Ok(RemoteImReplyDelegateNext::Guidance(messages)) => runtime_log_info(format!(
                    "[远程应答委托] 继续，任务=消费引导，delegate_id={}，message_count={}",
                    delegate_id_for_task,
                    messages.len()
                )),
                Err(err) => {
                    runtime_log_warn(format!(
                        "[远程应答委托] 跳过，任务=读取引导，delegate_id={}，error={}",
                        delegate_id_for_task, err
                    ));
                    terminal_status = DELEGATE_STATUS_FAILED;
                    terminal_reason = "读取远程应答引导失败";
                    break;
                }
            }
        }
        drop(permit);
        if let Err(err) = remote_im_reply_delegate_finish(
            &state_clone,
            &delegate_id_for_task,
            terminal_status,
            terminal_reason,
        ) {
            runtime_log_warn(format!(
                "[远程应答委托] 失败，任务=终结，delegate_id={}，error={}",
                delegate_id_for_task, err
            ));
        }
    });
    Ok(delegate_id)
}

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

fn remote_im_message_group_sender_id(message: &ChatMessage, contact: &RemoteImContact) -> Option<String> {
    if !contact.remote_contact_type.trim().eq_ignore_ascii_case("group") {
        return None;
    }
    if message.role.trim() != "user" {
        return None;
    }
    if message_origin_string(message, "kind") != Some("remote_im")
        || message_origin_string(message, "channel_id") != Some(contact.channel_id.trim())
        || message_origin_string(message, "contact_type") != Some(contact.remote_contact_type.trim())
        || message_origin_string(message, "contact_id") != Some(contact.remote_contact_id.trim())
    {
        return None;
    }
    message_origin_string(message, "sender_id").map(ToOwned::to_owned)
}

fn remote_im_latest_group_sender_id_for_busy_guided(
    state: &AppState,
    conversation_id: &str,
    contact: &RemoteImContact,
) -> Result<Option<String>, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() || !contact.remote_contact_type.trim().eq_ignore_ascii_case("group") {
        return Ok(None);
    }
    let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    ensure_ready_message_store_from_legacy_conversation(state, conversation_id, &paths)?;
    let mut page = message_store::read_ready_message_store_recent_messages_page(&paths, 100)?;
    while let Some(current) = page {
        if let Some(sender_id) = current
            .messages
            .iter()
            .rev()
            .find_map(|message| remote_im_message_group_sender_id(message, contact))
        {
            return Ok(Some(sender_id));
        }
        if !current.has_more {
            return Ok(None);
        }
        let Some(before_message_id) = current.messages.first().map(|message| message.id.as_str()) else {
            return Ok(None);
        };
        page = message_store::read_ready_message_store_messages_before(
            &paths,
            before_message_id,
            100,
        )?;
    }
    Ok(None)
}

fn remote_im_latest_secretary_work_message_for_busy_guided(
    state: &AppState,
    conversation_id: &str,
    contact: &RemoteImContact,
    agents: &[AgentProfile],
    current_assistant: &RemoteImConversationAssistantContext,
) -> Result<Option<RemoteImSecretaryMessageDigest>, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Ok(None);
    }
    let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    ensure_ready_message_store_from_legacy_conversation(state, conversation_id, &paths)?;
    let mut page = message_store::read_ready_message_store_recent_messages_page(&paths, 100)?;
    while let Some(current) = page {
        if let Some(message) = remote_im_collect_secretary_recent_messages(
            &current.messages,
            1,
            contact,
            agents,
            current_assistant,
        )
        .into_iter()
        .next()
        {
            return Ok(Some(message));
        }
        if !current.has_more {
            return Ok(None);
        }
        let Some(before_message_id) = current.messages.first().map(|message| message.id.as_str()) else {
            return Ok(None);
        };
        page = message_store::read_ready_message_store_messages_before(
            &paths,
            before_message_id,
            100,
        )?;
    }
    Ok(None)
}

fn remote_im_busy_guided_same_sender_allowed(
    contact: &RemoteImContact,
    current_sender_id: Option<&str>,
    new_sender_id: &str,
) -> bool {
    if !contact.remote_contact_type.trim().eq_ignore_ascii_case("group") {
        return true;
    }
    let current = current_sender_id.map(str::trim).filter(|value| !value.is_empty());
    let new = new_sender_id.trim();
    current.is_some_and(|value| value == new) && !new.is_empty()
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
    let mut runtime_states = lock_remote_im_contact_runtime_states(state)?;
    let runtime = remote_im_contact_runtime_state_mut(&mut runtime_states, &contact.id);
    let previous_presence = runtime.presence_state;
    let previous_work = runtime.work_state;
    let previous_pending = runtime.has_pending;
    let now = now_utc();
    let mut mute_prefix = String::new();
    if let Some(mute_until) = runtime.mute_until.clone() {
        if remote_im_is_mute_expired(&mute_until, now) {
            runtime.mute_until = None;
            mute_prefix = format!("闭嘴超时自动解除(截止={mute_until})；");
        }
    }
    if let Some(keyword) = remote_im_find_matched_keyword(message_text, &contact.mute_keywords) {
        let mute_until = remote_im_resolve_mute_until(now, contact.mute_duration_seconds);
        runtime.mute_until = Some(mute_until.clone());
        let reason = format!(
            "{}命中闭嘴词“{}”，进入闭嘴直到 {}，直接拦截后续判定",
            mute_prefix, keyword, mute_until
        );
        eprintln!(
            "[远程联系人状态机] 入站判定 完成: contact_id={}, presence={:?}, work={:?}, pending={}, activate_assistant={}, reason={}",
            contact.id,
            runtime.presence_state,
            runtime.work_state,
            runtime.has_pending,
            false,
            reason
        );
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
    if runtime.mute_until.is_some() {
        if let Some(keyword) = remote_im_find_matched_keyword(message_text, &contact.unmute_keywords) {
            runtime.mute_until = None;
            mute_prefix.push_str(&format!("命中张嘴词“{}”，解除闭嘴；", keyword));
        } else {
            let mute_until = runtime.mute_until.clone().unwrap_or_default();
            let reason = format!(
                "{}当前仍处于闭嘴期(截止={})，未命中张嘴词，直接拦截后续判定",
                mute_prefix, mute_until
            );
            eprintln!(
                "[远程联系人状态机] 入站判定 完成: contact_id={}, presence={:?}, work={:?}, pending={}, activate_assistant={}, reason={}",
                contact.id,
                runtime.presence_state,
                runtime.work_state,
                runtime.has_pending,
                false,
                reason
            );
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
    eprintln!(
        "[远程联系人状态机] 入站判定 完成: contact_id={}, presence={:?}, work={:?}, pending={}, activate_assistant={}, reason={}",
        contact.id,
        runtime.presence_state,
        runtime.work_state,
        runtime.has_pending,
        activate_assistant,
        reason
    );
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

fn remote_im_secretary_should_upgrade_guided(
    state: &AppState,
    conversation_id: &str,
) -> Result<bool, String> {
    Ok(matches!(
        get_conversation_runtime_state(state, conversation_id)?,
        MainSessionState::AssistantStreaming
    ))
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImSecretaryMessageDigest {
    time_text: String,
    speaker: String,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImSecretaryDecisionReply {
    #[serde(default, alias = "should_reply")]
    should_reply: bool,
    #[serde(default, alias = "target_delegate_id")]
    target_delegate_id: Option<String>,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone)]
struct RemoteImSecretaryDecision {
    should_reply: bool,
    target_delegate_id: Option<String>,
    reason: String,
    model_name: String,
    emit_log: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImSecretaryGuideDecisionReply {
    #[serde(default, alias = "should_interrupt")]
    should_interrupt: bool,
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Clone)]
struct RemoteImSecretaryGuideDecision {
    should_interrupt: bool,
    reason: String,
    emit_log: bool,
}

fn remote_im_secretary_contact_type_label(contact_type: &str) -> &str {
    match contact_type.trim().to_ascii_lowercase().as_str() {
        "group" => "群聊",
        "private" => "私聊",
        _ => "联系人",
    }
}

fn remote_im_secretary_contact_display_name(contact: &RemoteImContact) -> String {
    let remote_id = contact.remote_contact_id.trim();
    let remark_name = contact.remark_name.trim();
    if !remark_name.is_empty() && remark_name != remote_id {
        return remark_name.to_string();
    }
    let remote_name = contact.remote_contact_name.trim();
    if !remote_name.is_empty() && remote_name != remote_id {
        return remote_name.to_string();
    }
    remote_im_secretary_contact_type_label(&contact.remote_contact_type).to_string()
}

fn remote_im_secretary_context_display_name(name: &str, id: &str, fallback_name: &str) -> String {
    let name = name.trim();
    let id = id.trim();
    if !name.is_empty() && name != id {
        name.to_string()
    } else {
        fallback_name.to_string()
    }
}

fn remote_im_secretary_named_label(
    prefix: &str,
    name: &str,
    id: &str,
    fallback_name: &str,
    include_id: bool,
) -> String {
    let prefix = prefix.trim();
    let name = name.trim();
    let id = id.trim();
    let resolved_name = if !name.is_empty() && name != id {
        name
    } else if !fallback_name.trim().is_empty() {
        fallback_name.trim()
    } else if !prefix.is_empty() {
        prefix
    } else {
        "未知"
    };
    let base_label = if prefix.is_empty() || prefix == resolved_name {
        resolved_name.to_string()
    } else {
        format!("{prefix} {resolved_name}")
    };
    if include_id && !id.is_empty() {
        format!("{base_label}/{id}")
    } else {
        base_label
    }
}

fn remote_im_secretary_current_assistant_context(
    state: &AppState,
    conversation_id: &str,
) -> Result<RemoteImConversationAssistantContext, String> {
    get_conversation_remote_im_assistant_context(state, conversation_id)?
        .ok_or_else(|| format!("缺少当前助理上下文: conversation_id={}", conversation_id.trim()))
}

fn remote_im_resolve_contact_assistant_context(
    state: &AppState,
    contact: &RemoteImContact,
) -> Result<RemoteImConversationAssistantContext, String> {
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let requested_department_id = contact
        .bound_department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("联系人未设置应答部门: {}", contact.id))?;
    let (department_id, agent_id) = resolve_department_agent_pair(
        Some(requested_department_id),
        contact.bound_agent_id.as_deref(),
        &runtime_snapshot.config,
    )?;
    let department = runtime_department_by_id(&runtime_snapshot, &department_id)
        .ok_or_else(|| format!("路由部门不存在: {department_id}"))?;
    let agent = runtime_snapshot
        .agents
        .iter()
        .find(|item| item.id == agent_id)
        .ok_or_else(|| format!("路由人格不存在: {agent_id}"))?;
    let department_name = if department.name.trim().is_empty() {
        department.id.clone()
    } else {
        department.name.trim().to_string()
    };
    let agent_name = if agent.name.trim().is_empty() {
        agent.id.clone()
    } else {
        agent.name.trim().to_string()
    };
    Ok(RemoteImConversationAssistantContext {
        department_id,
        department_name,
        agent_id,
        agent_name,
    })
}

fn remote_im_secretary_message_speaker_label(
    message: &ChatMessage,
    contact: &RemoteImContact,
    agents: &[AgentProfile],
    current_assistant: &RemoteImConversationAssistantContext,
) -> Option<String> {
    match message.role.trim() {
        "assistant" => {
            let speaker_id = message
                .speaker_agent_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(current_assistant.agent_id.as_str());
            let speaker_name = agents
                .iter()
                .find(|agent| agent.id == speaker_id)
                .map(|agent| agent.name.trim().to_string())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| {
                    if speaker_id == current_assistant.agent_id {
                        current_assistant.agent_name.clone()
                    } else if speaker_id.is_empty() {
                        "当前助理".to_string()
                    } else {
                        speaker_id.to_string()
                    }
                });
            Some(remote_im_secretary_named_label(
                "",
                &speaker_name,
                speaker_id,
                "当前助理",
                false,
            ))
        }
        "user" => {
            if let Some(origin) = remote_im_origin_from_message(message) {
                let contact_type = remote_im_origin_string(origin, "contact_type")
                    .unwrap_or(contact.remote_contact_type.as_str());
                if contact_type.eq_ignore_ascii_case("group") {
                    let sender_name = remote_im_origin_string(origin, "sender_name").unwrap_or("");
                    let sender_id = remote_im_origin_string(origin, "sender_id").unwrap_or("");
                    return Some(remote_im_secretary_named_label(
                        "群友",
                        sender_name,
                        sender_id,
                        "群友",
                        true,
                    ));
                }
                let fallback_contact_name = remote_im_secretary_contact_display_name(contact);
                let contact_name = remote_im_origin_string(origin, "contact_name")
                    .unwrap_or(fallback_contact_name.as_str());
                let contact_id = remote_im_origin_string(origin, "contact_id")
                    .unwrap_or(contact.remote_contact_id.as_str());
                return Some(remote_im_secretary_named_label(
                    "",
                    contact_name,
                    contact_id,
                    "联系人",
                    true,
                ));
            }
            let fallback_contact_name = remote_im_secretary_contact_display_name(contact);
            Some(remote_im_secretary_named_label(
                "",
                &fallback_contact_name,
                contact.remote_contact_id.as_str(),
                "联系人",
                true,
            ))
        }
        _ => None,
    }
}

fn remote_im_secretary_truncate_text(text: &str, max_chars: usize) -> String {
    text.chars().take(max_chars).collect::<String>()
}

fn remote_im_secretary_message_time_text(created_at: &str) -> String {
    let time_text = format_utc_storage_time_to_local_relative_label(created_at);
    if time_text.trim().is_empty() {
        "时间未知".to_string()
    } else {
        time_text
    }
}

fn remote_im_secretary_message_line(
    item: &RemoteImSecretaryMessageDigest,
    latest_suffix: &str,
) -> String {
    format!(
        "[{}]({}){}：{}",
        item.speaker, item.time_text, latest_suffix, item.text
    )
}

fn remote_im_secretary_message_digest(
    message: &ChatMessage,
    contact: &RemoteImContact,
    agents: &[AgentProfile],
    current_assistant: &RemoteImConversationAssistantContext,
) -> Option<RemoteImSecretaryMessageDigest> {
    if is_context_compaction_message(message, message.role.trim()) {
        return None;
    }
    let speaker = remote_im_secretary_message_speaker_label(
        message,
        contact,
        agents,
        current_assistant,
    )?;
    let mut chunks = Vec::<String>::new();
    for part in &message.parts {
        match part {
            MessagePart::Text { text, .. } => {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    chunks.push(trimmed.to_string());
                }
            }
            MessagePart::Image { .. } => chunks.push("[图片]".to_string()),
            MessagePart::Audio { .. } => chunks.push("[音频]".to_string()),
        }
    }
    for block in &message.extra_text_blocks {
        let trimmed = block.trim();
        if !trimmed.is_empty() {
            chunks.push(trimmed.to_string());
        }
    }
    if chunks.is_empty() {
        return None;
    }
    Some(RemoteImSecretaryMessageDigest {
        time_text: remote_im_secretary_message_time_text(&message.created_at),
        speaker,
        text: remote_im_secretary_truncate_text(&chunks.join("\n"), 100),
    })
}

fn remote_im_collect_secretary_recent_messages(
    messages: &[ChatMessage],
    limit: usize,
    contact: &RemoteImContact,
    agents: &[AgentProfile],
    current_assistant: &RemoteImConversationAssistantContext,
) -> Vec<RemoteImSecretaryMessageDigest> {
    if limit == 0 {
        return Vec::new();
    }
    let mut selected = Vec::<RemoteImSecretaryMessageDigest>::new();
    for message in messages.iter().rev() {
        if let Some(digest) =
            remote_im_secretary_message_digest(message, contact, agents, current_assistant)
        {
            selected.push(digest);
            if selected.len() >= limit {
                break;
            }
        }
    }
    selected.reverse();
    selected
}

fn remote_im_secretary_messages_to_text(
    messages: &[RemoteImSecretaryMessageDigest],
    mark_latest_last: bool,
) -> String {
    if messages.is_empty() {
        return "（无）".to_string();
    }
    messages
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let latest_suffix = if mark_latest_last && idx + 1 == messages.len() {
                "（最新）"
            } else {
                ""
            };
            remote_im_secretary_message_line(item, latest_suffix)
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn build_remote_im_secretary_prepared_prompt(
    language: &str,
    contact: &RemoteImContact,
    current_assistant: &RemoteImConversationAssistantContext,
    history_messages: &[RemoteImSecretaryMessageDigest],
    new_batch_messages: &[RemoteImSecretaryMessageDigest],
    active_delegate_ids: &[String],
) -> PreparedPrompt {
    let guidance = normalize_contact_response_guidance(&contact.response_guidance);
    let contact_name = remote_im_secretary_contact_display_name(contact);
    let contact_type = remote_im_secretary_contact_type_label(&contact.remote_contact_type);
    let department_name = remote_im_secretary_context_display_name(
        &current_assistant.department_name,
        &current_assistant.department_id,
        "当前部门",
    );
    let agent_name = remote_im_secretary_context_display_name(
        &current_assistant.agent_name,
        &current_assistant.agent_id,
        "当前助理",
    );
    PreparedPrompt {
        preamble: format!(
            "请使用{language}完成远程联系人应答判断。\n\
你是正式处理部门入场前的秘书，只负责判断这一次是否应该回应，不负责代写回复。\n\
你会收到两段内容：最近 7 条已处理历史消息，以及本次未处理新消息。每条消息以 [发言人/ID](本地差异时间标签) 开头，助理消息可能没有 ID；正文只保留了前 100 个字，信息不足时不要过度推断。\n\
“未处理边界”之后的消息按时间从旧到新排列，最后一条就是最新消息，应优先围绕它判断是否需要回应。\n\
请优先遵守“什么时候应该回答”这段规则；如果规则不够，再按常识判断。\n\
如果无法确定，倾向于 shouldReply=true。\n\
只返回一个 JSON 对象，不要输出 Markdown、代码块或额外解释。\n\
JSON 只能包含字段：shouldReply, targetDelegateId, reason。"
        ),
        history_messages: Vec::new(),
        latest_user_text: format!(
            "当前应答部门：\n\
- 名称：{}\n\n\
当前助理：\n\
- 名称：{}\n\n\
当前联系人：\n\
- 名称：{contact_name}\n\
- 类型：{contact_type}\n\n\
什么时候应该回答：\n{guidance}\n\n\
最近 7 条已处理历史消息\n{}\n\n\
================ 未处理边界 ================\n\
以下是本次未处理新消息，按时间从旧到新排列，最后一条是最新消息\n{}\n\n\
当前活跃远程应答委托：
{}

如果新消息应继续某个活跃委托，targetDelegateId 必须填该委托 ID；如果是独立问题或没有活跃委托，targetDelegateId 留空。\n\n请直接输出 JSON。",
            department_name,
            agent_name,
            remote_im_secretary_messages_to_text(history_messages, false),
            remote_im_secretary_messages_to_text(new_batch_messages, true),
            if active_delegate_ids.is_empty() {
                "（无）".to_string()
            } else {
                active_delegate_ids.join("\n")
            },
        ),
        latest_user_meta_text: String::new(),
        latest_user_extra_text: String::new(),
        latest_user_extra_blocks: Vec::new(),
        latest_images: Vec::new(),
        latest_audios: Vec::new(),
    }
}

fn build_remote_im_secretary_guided_goal(
    language: &str,
    contact: &RemoteImContact,
    current_assistant: &RemoteImConversationAssistantContext,
    current_work_message: Option<&RemoteImSecretaryMessageDigest>,
    new_message: &RemoteImSecretaryMessageDigest,
) -> String {
    let guidance = normalize_contact_response_guidance(&contact.response_guidance);
    let contact_name = remote_im_secretary_contact_display_name(contact);
    let contact_type = remote_im_secretary_contact_type_label(&contact.remote_contact_type);
    let department_name = remote_im_secretary_context_display_name(
        &current_assistant.department_name,
        &current_assistant.department_id,
        "当前部门",
    );
    let agent_name = remote_im_secretary_context_display_name(
        &current_assistant.agent_name,
        &current_assistant.agent_id,
        "当前助理",
    );
    let current_work_text = current_work_message
        .map(|item| remote_im_secretary_message_line(item, ""))
        .unwrap_or_else(|| "（无）".to_string());
    let new_message_text = remote_im_secretary_message_line(new_message, "（最新）");
    format!(
        "请使用{language}完成远程联系人忙碌中的引导判断。\n\
你是秘书。助理正在处理当前激活消息，此时又来了新消息。\n\
你只负责判断：是否应该优先让助理先看这条新消息，并把它升级为引导消息。\n\
如果不值得打断当前工作，就返回 shouldInterrupt=false。\n\
如果值得优先插队，就返回 shouldInterrupt=true。\n\
请优先遵守“什么时候应该回答”这段规则；如果规则不够，再按常识判断。\n\
只返回一个 JSON 对象，不要输出 Markdown、代码块或额外解释。\n\
JSON 只能包含字段：shouldInterrupt, reason。\n\n\
当前应答部门：\n- 名称：{}\n\n\
当前助理：\n- 名称：{}\n\n\
当前联系人：\n- 名称：{contact_name}\n- 类型：{contact_type}\n\n\
什么时候应该回答：\n{guidance}\n\n\
助理当前正在处理的激活消息：\n{}\n\n\
刚到的新消息：\n{}\n\n\
请直接输出 JSON。",
        department_name,
        agent_name,
        current_work_text,
        new_message_text,
    )
}

async fn run_remote_im_secretary_guided_decision(
    state: &AppState,
    contact: &RemoteImContact,
    current_assistant: &RemoteImConversationAssistantContext,
    current_work_message: Option<&RemoteImSecretaryMessageDigest>,
    new_message: &RemoteImSecretaryMessageDigest,
) -> Result<RemoteImSecretaryGuideDecision, String> {
    if normalize_contact_response_strategy(&contact.response_strategy) == "always_reply" {
        return Ok(RemoteImSecretaryGuideDecision {
            should_interrupt: true,
            reason: String::new(),
            emit_log: false,
        });
    }

    let review_api_config_id = current_tool_review_api_config_id(state)?
        .ok_or_else(|| "未配置快速模型".to_string())?;
    let app_config = state_read_config_cached(state)?;
    let selected_api = resolve_selected_api_config(&app_config, Some(&review_api_config_id))
        .ok_or_else(|| format!("快速模型配置不存在：{}", review_api_config_id))?;
    if !selected_api.enable_text || !selected_api.request_format.is_chat_text() {
        return Err("快速模型不支持文本对话".to_string());
    }
    let resolved_api = resolve_api_config(&app_config, Some(&review_api_config_id))?;
    let model_name = if selected_api.model.trim().is_empty() {
        resolved_api.model.clone()
    } else {
        selected_api.model.trim().to_string()
    };
    let language = terminal_smart_review_language(&app_config.ui_language);
    let prepared = PreparedPrompt {
        preamble: build_remote_im_secretary_guided_goal(
            language,
            contact,
            current_assistant,
            current_work_message,
            new_message,
        ),
        history_messages: Vec::new(),
        latest_user_text: String::new(),
        latest_user_meta_text: String::new(),
        latest_user_extra_text: String::new(),
        latest_user_extra_blocks: Vec::new(),
        latest_images: Vec::new(),
        latest_audios: Vec::new(),
    };
    let request_text = prepared_prompt_to_fast_request_text(&prepared);
    let record_conversation_id = contact
        .bound_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let execution = invoke_model_with_policy(
        &resolved_api,
        &model_name,
        prepared,
        CallPolicy {
            scene: "Remote IM secretary guided review",
            timeout_secs: Some(12),
            json_only: true,
        },
        Some(state),
    )
    .await;
    push_model_call_log_parts(Some(state), &execution);
    let duration_ms = execution.log_parts.elapsed_ms;
    let reply = match execution.result {
        Ok(reply) => reply,
        Err(err) => {
            if let Some(conversation_id) = record_conversation_id.as_deref() {
                record_fast_request_turn_best_effort(
                    state,
                    conversation_id,
                    build_fast_request_turn(
                        FAST_REQUEST_KIND_REMOTE_IM_INTERRUPT_DECISION,
                        &request_text,
                        "",
                        false,
                        Some(err.clone()),
                        Some(model_name.clone()),
                        Some(duration_ms),
                    ),
                );
            }
            return Err(err);
        }
    };
    let raw_text = if reply.final_response_text.trim().is_empty() {
        reply.assistant_text.trim()
    } else {
        reply.final_response_text.trim()
    };
    let parsed = match serde_json::from_str::<RemoteImSecretaryGuideDecisionReply>(
        remote_im_secretary_extract_json(raw_text),
    )
    {
        Ok(parsed) => {
            if let Some(conversation_id) = record_conversation_id.as_deref() {
                record_fast_request_turn_best_effort(
                    state,
                    conversation_id,
                    build_fast_request_turn(
                        FAST_REQUEST_KIND_REMOTE_IM_INTERRUPT_DECISION,
                        &request_text,
                        raw_text,
                        true,
                        None,
                        Some(model_name.clone()),
                        Some(duration_ms),
                    ),
                );
            }
            parsed
        }
        Err(err) => {
            let message = format!("解析秘书引导 JSON 失败: {err}; raw={}", raw_text.trim());
            if let Some(conversation_id) = record_conversation_id.as_deref() {
                record_fast_request_turn_best_effort(
                    state,
                    conversation_id,
                    build_fast_request_turn(
                        FAST_REQUEST_KIND_REMOTE_IM_INTERRUPT_DECISION,
                        &request_text,
                        raw_text,
                        false,
                        Some(message.clone()),
                        Some(model_name.clone()),
                        Some(duration_ms),
                    ),
                );
            }
            return Err(message);
        }
    };
    runtime_log_debug(format!(
        "[远程联系人秘书] 忙碌引导快速判断完成: contact_id={} model_name={} should_interrupt={}",
        contact.id,
        model_name,
        parsed.should_interrupt
    ));
    Ok(RemoteImSecretaryGuideDecision {
        should_interrupt: parsed.should_interrupt,
        reason: parsed.reason.trim().to_string(),
        emit_log: true,
    })
}
fn remote_im_secretary_extract_json(raw: &str) -> &str {
    let trimmed = raw.trim();
    if let Some(stripped) = trimmed.strip_prefix("```json") {
        return stripped.trim().trim_end_matches("```").trim();
    }
    if let Some(stripped) = trimmed.strip_prefix("```") {
        return stripped.trim().trim_end_matches("```").trim();
    }
    trimmed
}

fn remote_im_resolve_secretary_contact(
    state: &AppState,
    activated_sources: &[RemoteImActivationSource],
) -> Result<Option<RemoteImContact>, String> {
    let Some(source) = activated_sources.first() else {
        return Ok(None);
    };
    if activated_sources.len() > 1 {
        runtime_log_warn(format!(
            "[远程联系人秘书] 本轮激活联系人超过 1 个，跳过秘书判断: source_count={}",
            activated_sources.len()
        ));
        return Ok(None);
    }
    let runtime = state_read_runtime_state_cached(state)?;
    Ok(remote_im_contact_by_activation_source_in_runtime(&runtime.remote_im_contacts, source).cloned())
}

async fn run_remote_im_secretary_decision(
    state: &AppState,
    contact: &RemoteImContact,
    current_assistant: &RemoteImConversationAssistantContext,
    history_messages: &[RemoteImSecretaryMessageDigest],
    new_batch_messages: &[RemoteImSecretaryMessageDigest],
    active_delegate_ids: &[String],
) -> Result<RemoteImSecretaryDecision, String> {
    if normalize_contact_response_strategy(&contact.response_strategy) == "always_reply" {
        return Ok(RemoteImSecretaryDecision {
            should_reply: true,
            target_delegate_id: None,
            reason: String::new(),
            model_name: String::new(),
            emit_log: false,
        });
    }

    let review_api_config_id = current_tool_review_api_config_id(state)?
        .ok_or_else(|| "未配置快速模型".to_string())?;
    let app_config = state_read_config_cached(state)?;
    let selected_api = resolve_selected_api_config(&app_config, Some(&review_api_config_id))
        .ok_or_else(|| format!("快速模型配置不存在：{}", review_api_config_id))?;
    if !selected_api.enable_text || !selected_api.request_format.is_chat_text() {
        return Err("快速模型不支持文本对话".to_string());
    }
    let resolved_api = resolve_api_config(&app_config, Some(&review_api_config_id))?;
    let model_name = if selected_api.model.trim().is_empty() {
        resolved_api.model.clone()
    } else {
        selected_api.model.trim().to_string()
    };
    let language = terminal_smart_review_language(&app_config.ui_language);
    let prepared = build_remote_im_secretary_prepared_prompt(
        language,
        contact,
        current_assistant,
        history_messages,
        new_batch_messages,
        active_delegate_ids,
    );
    let request_text = prepared_prompt_to_fast_request_text(&prepared);
    let record_conversation_id = contact
        .bound_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let execution = invoke_model_with_policy(
        &resolved_api,
        &model_name,
        prepared,
        CallPolicy {
            scene: "Remote IM secretary review",
            timeout_secs: Some(12),
            json_only: true,
        },
        Some(state),
    )
    .await;
    push_model_call_log_parts(Some(state), &execution);
    let duration_ms = execution.log_parts.elapsed_ms;
    let reply = match execution.result {
        Ok(reply) => reply,
        Err(err) => {
            if let Some(conversation_id) = record_conversation_id.as_deref() {
                record_fast_request_turn_best_effort(
                    state,
                    conversation_id,
                    build_fast_request_turn(
                        FAST_REQUEST_KIND_REMOTE_IM_REPLY_DECISION,
                        &request_text,
                        "",
                        false,
                        Some(err.clone()),
                        Some(model_name.clone()),
                        Some(duration_ms),
                    ),
                );
            }
            return Err(err);
        }
    };
    let raw_text = if reply.final_response_text.trim().is_empty() {
        reply.assistant_text.trim()
    } else {
        reply.final_response_text.trim()
    };
    let parsed = match serde_json::from_str::<RemoteImSecretaryDecisionReply>(
        remote_im_secretary_extract_json(raw_text),
    )
    {
        Ok(parsed) => {
            if let Some(conversation_id) = record_conversation_id.as_deref() {
                record_fast_request_turn_best_effort(
                    state,
                    conversation_id,
                    build_fast_request_turn(
                        FAST_REQUEST_KIND_REMOTE_IM_REPLY_DECISION,
                        &request_text,
                        raw_text,
                        true,
                        None,
                        Some(model_name.clone()),
                        Some(duration_ms),
                    ),
                );
            }
            parsed
        }
        Err(err) => {
            let message = format!("解析秘书 JSON 失败: {err}; raw={}", raw_text.trim());
            if let Some(conversation_id) = record_conversation_id.as_deref() {
                record_fast_request_turn_best_effort(
                    state,
                    conversation_id,
                    build_fast_request_turn(
                        FAST_REQUEST_KIND_REMOTE_IM_REPLY_DECISION,
                        &request_text,
                        raw_text,
                        false,
                        Some(message.clone()),
                        Some(model_name.clone()),
                        Some(duration_ms),
                    ),
                );
            }
            return Err(message);
        }
    };
    Ok(RemoteImSecretaryDecision {
        should_reply: parsed.should_reply,
        target_delegate_id: parsed
            .target_delegate_id
            .map(|value| value.trim().to_string())
            .filter(|value| active_delegate_ids.iter().any(|item| item == value)),
        reason: parsed.reason.trim().to_string(),
        model_name,
        emit_log: true,
    })
}

fn remote_im_event_latest_message_id(event: &ChatPendingEvent) -> Option<String> {
    event.messages.last().map(|message| message.id.clone())
}

fn remote_im_update_checkpoint_latest_seen_in_list(
    checkpoints: &mut Vec<RemoteImContactCheckpoint>,
    contact_id: &str,
    message_id: Option<&str>,
    now: &str,
) {
    let checkpoint = remote_im_contact_checkpoint_mut_in_list(checkpoints, contact_id);
    remote_im_update_checkpoint_latest_seen_in_checkpoint(checkpoint, message_id, now);
}

fn remote_im_update_checkpoint_latest_seen_in_checkpoint(
    checkpoint: &mut RemoteImContactCheckpoint,
    message_id: Option<&str>,
    now: &str,
) {
    checkpoint.latest_seen_message_id = message_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .or(checkpoint.latest_seen_message_id.clone());
    checkpoint.updated_at = Some(now.to_string());
}

fn remote_im_handle_persisted_event_after_history_flush_runtime(
    state: &AppState,
    contacts: &[RemoteImContact],
    checkpoints: &mut Vec<RemoteImContactCheckpoint>,
    conversation: &mut Conversation,
    event: &ChatPendingEvent,
    now: &str,
    activated_contacts_in_batch: &mut std::collections::HashSet<String>,
) -> Result<bool, String> {
    let Some(sender) = event.sender_info.as_ref() else {
        return Ok(false);
    };
    let Some(contact) = remote_im_contact_by_source_in_runtime(contacts, sender).cloned() else {
        return Ok(false);
    };
    let latest_message_id = remote_im_event_latest_message_id(event);
    remote_im_update_checkpoint_latest_seen_in_list(
        checkpoints,
        &contact.id,
        latest_message_id.as_deref(),
        now,
    );
    if !event.activate_assistant {
        remote_im_append_channel_log(
            &contact.channel_id,
            "info",
            format!(
                "[联系人状态] 历史落地: contact={}, conversation_id={}, activate=否, reason=event_gate_blocked",
                remote_im_contact_log_label(&contact),
                conversation.id
            ),
        );
        return Ok(false);
    }

    let should_activate;
    let (
        previous_presence,
        previous_work,
        previous_pending,
        current_presence,
        current_work,
        current_pending,
        state_reason,
    ) = {
        let mut runtime_states = lock_remote_im_contact_runtime_states(state)?;
        let runtime = remote_im_contact_runtime_state_mut(&mut runtime_states, &contact.id);
        let previous_presence = runtime.presence_state;
        let previous_work = runtime.work_state;
        let previous_pending = runtime.has_pending;
        let state_reason = match runtime.presence_state {
            RemoteImPresenceState::Away => "persisted_await_secretary_wake".to_string(),
            RemoteImPresenceState::Present => "persisted_await_secretary_present".to_string(),
        };
        should_activate = true;
        (
            previous_presence,
            previous_work,
            previous_pending,
            runtime.presence_state,
            runtime.work_state,
            runtime.has_pending,
            state_reason,
        )
    };

    if should_activate {
        // 每个已落库远程事件都必须单独交给秘书判断；不能因同一批前一条
        // 消息已经激活而跳过后续消息。
        activated_contacts_in_batch.insert(format!("{}:{}", contact.id, event.id));
        eprintln!(
            "[远程联系人状态机] 激活调度 开始: contact_id={}, conversation_id={}",
            contact.id, conversation.id
        );
    }
    remote_im_append_channel_log(
        &contact.channel_id,
        "info",
        format!(
            "[联系人状态] 历史落地: contact={}, conversation_id={}, presence={} -> {}, work={} -> {}, pending={} -> {}, activate={}, reason={}",
            remote_im_contact_log_label(&contact),
            conversation.id,
            remote_im_presence_state_label(previous_presence),
            remote_im_presence_state_label(current_presence),
            remote_im_work_state_label(previous_work),
            remote_im_work_state_label(current_work),
            remote_im_yes_no(previous_pending),
            remote_im_yes_no(current_pending),
            remote_im_yes_no(should_activate),
            state_reason
        ),
    );
    Ok(should_activate)
}

fn remote_im_finalize_round_completion(
    state: &AppState,
    activated_sources: &[RemoteImActivationSource],
    reply_decision: Option<&str>,
    reply_target: Option<&RemoteImReplyTarget>,
    failed_error: Option<&str>,
    finished_at: &str,
) -> Result<Vec<RemoteImActivationSource>, String> {
    if activated_sources.is_empty() {
        return Ok(Vec::new());
    }
    let runtime = state_read_runtime_state_cached(state)?;
    let mut runtime_states = lock_remote_im_contact_runtime_states(state)?;
    let mut follow_up_sources = Vec::<RemoteImActivationSource>::new();
    for source in activated_sources {
        let Some(contact) =
            remote_im_contact_by_activation_source_in_runtime(&runtime.remote_im_contacts, source)
        else {
            continue;
        };
        let runtime = remote_im_contact_runtime_state_mut(&mut runtime_states, &contact.id);
        let previous_presence = runtime.presence_state;
        let previous_work = runtime.work_state;
        let previous_pending = runtime.has_pending;
        let previous_no_reply_count = runtime.consecutive_no_reply_count;
        runtime.work_state = RemoteImWorkState::Idle;
        let decision_label = match reply_decision.unwrap_or("").trim() {
            "" => "send_async",
            value => value,
        };
        if let Some(error) = failed_error {
            eprintln!(
                "[远程联系人状态机] 轮次结束 失败: contact_id={}, presence={:?}->{:?}, pending={}, error={}",
                contact.id,
                previous_presence,
                runtime.presence_state,
                previous_pending,
                error
            );
            remote_im_append_channel_log(
                &contact.channel_id,
                "warn",
                format!(
                    "[联系人状态] 轮次收尾失败: contact={}, decision={}, presence={} -> {}, work={} -> {}, pending={} -> {}, error={}",
                    remote_im_contact_log_label(contact),
                    decision_label,
                    remote_im_presence_state_label(previous_presence),
                    remote_im_presence_state_label(runtime.presence_state),
                    remote_im_work_state_label(previous_work),
                    remote_im_work_state_label(runtime.work_state),
                    remote_im_yes_no(previous_pending),
                    remote_im_yes_no(runtime.has_pending),
                    error
                ),
            );
            continue;
        }
        let should_follow_up_after_round = previous_pending;
        match decision_label {
            "reply" | "send_files" | "send" | "reply_async" => {
                let target_matched = reply_target
                    .map(|target| remote_im_contact_matches_reply_target(source, target))
                    .unwrap_or(activated_sources.len() == 1);
                runtime.presence_state = RemoteImPresenceState::Present;
                runtime.consecutive_no_reply_count = 0;
                if target_matched {
                    runtime.last_success_reply_at = Some(finished_at.to_string());
                }
            }
            "no_reply" => {
                runtime.consecutive_no_reply_count =
                    runtime.consecutive_no_reply_count.saturating_add(1);
                if runtime.has_pending {
                    runtime.presence_state = RemoteImPresenceState::Present;
                } else if runtime.consecutive_no_reply_count >= 2 {
                    runtime.presence_state = RemoteImPresenceState::Away;
                } else if let Some(last_success_at) = runtime.last_success_reply_at.as_deref() {
                    let elapsed_seconds = parse_iso(last_success_at)
                        .map(|last| (now_utc() - last).whole_seconds().max(0) as u64)
                        .unwrap_or_default();
                    if elapsed_seconds > contact.patience_seconds {
                        runtime.presence_state = RemoteImPresenceState::Away;
                    } else {
                        runtime.presence_state = RemoteImPresenceState::Present;
                    }
                } else {
                    runtime.presence_state = RemoteImPresenceState::Present;
                }
            }
            "send_async" | "" => {
                runtime.presence_state = RemoteImPresenceState::Present;
                runtime.consecutive_no_reply_count = 0;
            }
            _ => {}
        }
        if should_follow_up_after_round {
            runtime.has_pending = false;
            runtime.presence_state = RemoteImPresenceState::Present;
            follow_up_sources.push(source.clone());
        }
        eprintln!(
            "[远程联系人状态机] 轮次结束 完成: contact_id={}, decision={}, presence={:?}->{:?}, pending={}->{}, no_reply_count={}->{}, follow_up={}, last_success_reply_at={}",
            contact.id,
            decision_label,
            previous_presence,
            runtime.presence_state,
            previous_pending,
            runtime.has_pending,
            previous_no_reply_count,
            runtime.consecutive_no_reply_count,
            should_follow_up_after_round,
            runtime.last_success_reply_at.as_deref().unwrap_or("")
        );
        remote_im_append_channel_log(
            &contact.channel_id,
            "info",
            format!(
                "[联系人状态] 轮次结束: contact={}, decision={}, presence={} -> {}, work={} -> {}, pending={} -> {}, no_reply_count={} -> {}, follow_up={}, last_success_reply_at={}",
                remote_im_contact_log_label(contact),
                decision_label,
                remote_im_presence_state_label(previous_presence),
                remote_im_presence_state_label(runtime.presence_state),
                remote_im_work_state_label(previous_work),
                remote_im_work_state_label(runtime.work_state),
                remote_im_yes_no(previous_pending),
                remote_im_yes_no(runtime.has_pending),
                previous_no_reply_count,
                runtime.consecutive_no_reply_count,
                remote_im_yes_no(should_follow_up_after_round),
                runtime.last_success_reply_at.as_deref().unwrap_or("")
            ),
        );
    }
    Ok(follow_up_sources)
}

fn remote_im_finalize_async_send_result(
    state: &AppState,
    source: &RemoteImActivationSource,
    send_ok: bool,
    now: &str,
    error: Option<&str>,
) -> Result<(), String> {
    let runtime = state_read_runtime_state_cached(state)?;
    let Some(contact) =
        remote_im_contact_by_activation_source_in_runtime(&runtime.remote_im_contacts, source)
    else {
        return Ok(());
    };
    let mut runtime_states = lock_remote_im_contact_runtime_states(state)?;
    let runtime = remote_im_contact_runtime_state_mut(&mut runtime_states, &contact.id);
    let previous_presence = runtime.presence_state;
    let previous_no_reply_count = runtime.consecutive_no_reply_count;
    runtime.presence_state = RemoteImPresenceState::Present;
    runtime.consecutive_no_reply_count = 0;
    if send_ok {
        runtime.last_success_reply_at = Some(now.to_string());
    }
    eprintln!(
        "[远程联系人状态机] 异步发送{}: contact_id={}, last_success_reply_at={}, error={}",
        if send_ok { "完成" } else { "失败" },
        contact.id,
        runtime.last_success_reply_at.as_deref().unwrap_or(""),
        error.unwrap_or("")
    );
    remote_im_append_channel_log(
        &contact.channel_id,
        if send_ok { "info" } else { "warn" },
        format!(
            "[联系人状态] 异步发送收尾: contact={}, result={}, presence={} -> {}, no_reply_count={} -> {}, last_success_reply_at={}, error={}",
            remote_im_contact_log_label(&contact),
            if send_ok { "成功" } else { "失败" },
            remote_im_presence_state_label(previous_presence),
            remote_im_presence_state_label(runtime.presence_state),
            previous_no_reply_count,
            runtime.consecutive_no_reply_count,
            runtime.last_success_reply_at.as_deref().unwrap_or(""),
            error.unwrap_or("")
        ),
    );
    Ok(())
}

fn remote_im_contact_display_name(contact: &RemoteImContact) -> String {
    let remark = contact.remark_name.trim();
    if !remark.is_empty() {
        return remark.to_string();
    }
    let remote_name = contact.remote_contact_name.trim();
    if !remote_name.is_empty() {
        return remote_name.to_string();
    }
    contact.remote_contact_id.trim().to_string()
}

#[derive(Debug, Clone)]
struct RemoteImOutboundContentDigest {
    text_preview: String,
    text_count: usize,
    image_count: usize,
    file_count: usize,
    other_count: usize,
}

fn remote_im_presence_state_label(state: RemoteImPresenceState) -> &'static str {
    match state {
        RemoteImPresenceState::Away => "离场",
        RemoteImPresenceState::Present => "在场",
    }
}

fn remote_im_work_state_label(state: RemoteImWorkState) -> &'static str {
    match state {
        RemoteImWorkState::Idle => "空闲",
        RemoteImWorkState::Busy => "忙碌",
    }
}

fn remote_im_yes_no(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}

fn remote_im_preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "（无文本）".to_string();
    }
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    format!(
        "{}...",
        normalized.chars().take(max_chars).collect::<String>()
    )
}

fn remote_im_contact_log_label(contact: &RemoteImContact) -> String {
    format!(
        "{}[{}:{}]",
        remote_im_contact_display_name(contact),
        contact.remote_contact_type.trim(),
        contact.remote_contact_id.trim()
    )
}

fn remote_im_contact_log_marker(contact: &RemoteImContact) -> String {
    format!(
        "[{}:{}]",
        contact.remote_contact_type.trim(),
        contact.remote_contact_id.trim()
    )
}

fn remote_im_contact_downloads_segment(value: &str, fallback: &str) -> String {
    let sanitized = sanitize_download_file_name(value);
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn remote_im_contact_downloads_subdir_parts(
    channel_id: &str,
    contact_type: &str,
    contact_id: &str,
) -> String {
    format!(
        "contacts/{}/{}/{}/downloads",
        remote_im_contact_downloads_segment(channel_id, "unknown-channel"),
        remote_im_contact_downloads_segment(contact_type, "unknown-type"),
        remote_im_contact_downloads_segment(contact_id, "unknown-contact")
    )
}

fn remote_im_contact_downloads_subdir(contact: &RemoteImContact) -> String {
    remote_im_contact_downloads_subdir_parts(
        &contact.channel_id,
        &contact.remote_contact_type,
        &contact.remote_contact_id,
    )
}

fn remote_im_contact_downloads_relative_dir(contact: &RemoteImContact) -> String {
    format!("downloads/{}", remote_im_contact_downloads_subdir(contact))
}

fn remote_im_activation_source_log_label(source: &RemoteImActivationSource) -> String {
    let display_name = source.remote_contact_name.trim();
    let name = if display_name.is_empty() {
        source.remote_contact_id.trim()
    } else {
        display_name
    };
    format!(
        "{}[{}:{}]",
        name,
        source.remote_contact_type.trim(),
        source.remote_contact_id.trim()
    )
}

fn remote_im_outbound_content_digest(content: &[Value]) -> RemoteImOutboundContentDigest {
    let mut text_count = 0usize;
    let mut image_count = 0usize;
    let mut file_count = 0usize;
    let mut other_count = 0usize;
    let mut text_fragments = Vec::<String>::new();
    let mut asset_names = Vec::<String>::new();
    for item in content {
        match item.get("type").and_then(Value::as_str).unwrap_or("") {
            "text" => {
                text_count += 1;
                let text = item
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("");
                if !text.is_empty() {
                    text_fragments.push(text.to_string());
                }
            }
            "image" => {
                image_count += 1;
                if let Some(name) = item
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    asset_names.push(name.to_string());
                }
            }
            "file" => {
                file_count += 1;
                if let Some(name) = item
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    asset_names.push(name.to_string());
                }
            }
            _ => {
                other_count += 1;
            }
        }
    }
    let preview_source = if !text_fragments.is_empty() {
        text_fragments.join(" / ")
    } else if !asset_names.is_empty() {
        asset_names.join(", ")
    } else if image_count + file_count + other_count > 0 {
        format!("附件 {} 个", image_count + file_count + other_count)
    } else {
        String::new()
    };
    RemoteImOutboundContentDigest {
        text_preview: remote_im_preview_text(&preview_source, 100),
        text_count,
        image_count,
        file_count,
        other_count,
    }
}

#[cfg(not(test))]
fn remote_im_append_channel_log(channel_id: &str, level: &str, message: String) {
    let channel_id = channel_id.trim().to_string();
    let level = level.trim().to_string();
    let message = message.trim().to_string();
    if channel_id.is_empty() || level.is_empty() || message.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        onebot_v11_ws_manager()
            .add_log(&channel_id, &level, &message)
            .await;
    });
}

#[cfg(test)]
fn remote_im_append_channel_log(channel_id: &str, level: &str, message: String) {
    let _ = (channel_id, level, message);
}

#[cfg(not(test))]
async fn remote_im_append_channel_log_async(channel_id: &str, level: &str, message: String) {
    let channel_id = channel_id.trim().to_string();
    let level = level.trim().to_string();
    let message = message.trim().to_string();
    if channel_id.is_empty() || level.is_empty() || message.is_empty() {
        return;
    }
    onebot_v11_ws_manager()
        .add_log(&channel_id, &level, &message)
        .await;
}

#[cfg(test)]
async fn remote_im_append_channel_log_async(channel_id: &str, level: &str, message: String) {
    let _ = (channel_id, level, message);
}

fn remote_im_resolve_contact_log_query(
    state: &AppState,
    contact_id: &str,
) -> Result<(String, String), String> {
    let normalized_contact_id = contact_id.trim();
    if normalized_contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter()
        .find(|item| item.id == normalized_contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{normalized_contact_id}"))?;
    Ok((
        contact.channel_id.trim().to_string(),
        remote_im_contact_log_marker(contact),
    ))
}

fn remote_im_filter_channel_logs_for_contact(
    logs: Vec<ChannelLogEntry>,
    contact_marker: &str,
) -> Vec<ChannelLogEntry> {
    let normalized_marker = contact_marker.trim();
    if normalized_marker.is_empty() {
        return Vec::new();
    }
    logs.into_iter()
        .filter(|entry| entry.message.contains(normalized_marker))
        .collect()
}

fn remote_im_resolve_effective_route_mode(
    _config: &AppConfig,
    _contact: &RemoteImContact,
) -> String {
    "dedicated_contact_conversation".to_string()
}

fn remote_im_contact_conversation_title(contact: &RemoteImContact) -> String {
    format!("联系人 · {}", remote_im_contact_display_name(contact))
}

fn remote_im_contact_conversation_key_parts(
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
) -> String {
    format!(
        "remote_im_contact:{}:{}:{}",
        channel_id.trim(),
        remote_contact_type.trim().to_ascii_lowercase(),
        remote_contact_id.trim()
    )
}

fn remote_im_contact_conversation_key(contact: &RemoteImContact) -> String {
    remote_im_contact_conversation_key_parts(
        &contact.channel_id,
        &contact.remote_contact_type,
        &contact.remote_contact_id,
    )
}

fn remote_im_set_sender_origin_meta(
    input: &RemoteImEnqueueInput,
    conversation_id: &str,
    contact_record_id: &str,
) -> Value {
    serde_json::json!({
        "origin": {
            "kind": "remote_im",
            "channel_id": input.channel_id,
            "platform": input.platform,
            "im_name": input.im_name,
            "contact_type": input.remote_contact_type,
            "contact_id": input.remote_contact_id,
            "contact_name": input.remote_contact_name,
            "contact_record_id": contact_record_id,
            "sender_id": input.sender_id,
            "sender_name": input.sender_name,
            "sender_avatar_url": input.sender_avatar_url,
            "platform_message_id": input.platform_message_id,
            "conversation_id": conversation_id
        }
    })
}

fn remote_im_resolve_inbound_activate(
    channel: &RemoteImChannelConfig,
    message_flag: Option<bool>,
) -> bool {
    message_flag.unwrap_or(channel.activate_assistant)
}

fn origin_value_string<'a>(origin: &'a Value, key: &str) -> Option<&'a str> {
    origin
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn message_origin_string<'a>(message: &'a ChatMessage, key: &str) -> Option<&'a str> {
    let origin = message.provider_meta.as_ref()?.get("origin")?;
    origin_value_string(origin, key)
}

fn message_has_remote_im_platform_message(
    message: &ChatMessage,
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
    platform_message_id: &str,
) -> bool {
    message_origin_string(message, "kind") == Some("remote_im")
        && message_origin_string(message, "channel_id") == Some(channel_id)
        && message_origin_string(message, "contact_type") == Some(remote_contact_type)
        && message_origin_string(message, "contact_id") == Some(remote_contact_id)
        && message_origin_string(message, "platform_message_id") == Some(platform_message_id)
}

fn ready_store_has_remote_im_platform_message(
    state: &AppState,
    conversation_id: &str,
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
    platform_message_id: &str,
) -> Result<bool, String> {
    let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    let Some(page) = message_store::read_ready_message_store_block_page(&paths, None)? else {
        return Ok(false);
    };
    for block in page.blocks.into_iter().rev() {
        let Some(block_page) =
            message_store::read_ready_message_store_block_page(&paths, Some(block.block_id))?
        else {
            continue;
        };
        if block_page.messages.iter().any(|message| {
            message_has_remote_im_platform_message(
                message,
                channel_id,
                remote_contact_type,
                remote_contact_id,
                platform_message_id,
            )
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn pending_event_has_remote_im_platform_message(
    event: &ChatPendingEvent,
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
    platform_message_id: &str,
) -> bool {
    event.sender_info.as_ref().is_some_and(|sender| {
        sender.channel_id.trim() == channel_id
            && sender.remote_contact_type.trim() == remote_contact_type
            && sender.remote_contact_id.trim() == remote_contact_id
            && sender.platform_message_id.as_deref().map(str::trim) == Some(platform_message_id)
    })
}

fn remote_im_is_duplicate_platform_message(
    state: &AppState,
    conversation_id: &str,
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
    platform_message_id: &str,
) -> Result<bool, String> {
    if ready_store_has_remote_im_platform_message(
        state,
        conversation_id,
        channel_id,
        remote_contact_type,
        remote_contact_id,
        platform_message_id,
    )? {
        return Ok(true);
    }

    let slots = lock_conversation_runtime_slots(state)?;
    Ok(slots.values().any(|slot| {
        slot.pending_queue.iter().any(|event| {
            event.conversation_id == conversation_id
                && pending_event_has_remote_im_platform_message(
                    event,
                    channel_id,
                    remote_contact_type,
                    remote_contact_id,
                    platform_message_id,
                )
        })
    }))
}

struct ValidatedEnqueueInput {
    text: String,
    images: Vec<BinaryPart>,
    audios: Vec<BinaryPart>,
    attachments: Vec<AttachmentMetaInput>,
    channel: RemoteImChannelConfig,
}

fn validate_images(channel: &RemoteImChannelConfig, input: &RemoteImEnqueueInput) -> Vec<BinaryPart> {
    if channel.receive_files {
        input.payload.images.clone().unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn validate_audios(channel: &RemoteImChannelConfig, input: &RemoteImEnqueueInput) -> Vec<BinaryPart> {
    if channel.receive_files {
        input.payload.audios.clone().unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn validate_attachments(
    channel: &RemoteImChannelConfig,
    input: &RemoteImEnqueueInput,
) -> Vec<AttachmentMetaInput> {
    if channel.receive_files {
        input.payload.attachments.clone().unwrap_or_default()
    } else {
        Vec::new()
    }
}

fn resolve_channel_config(
    input: &RemoteImEnqueueInput,
    config: &AppConfig,
) -> Result<(String, RemoteImChannelConfig), String> {
    let channel_id = input.channel_id.trim().to_string();
    if channel_id.is_empty() {
        return Err("channel_id 不能为空".to_string());
    }
    let channel = remote_im_channel_by_id(config, &channel_id)
        .ok_or_else(|| format!("远程IM渠道不存在: {channel_id}"))?
        .clone();
    if !channel.enabled {
        return Err(format!("远程IM渠道未启用: {channel_id}"));
    }
    Ok((channel_id, channel))
}

fn resolve_department_agent_pair(
    requested_department_id: Option<&str>,
    requested_agent_id: Option<&str>,
    config: &AppConfig,
) -> Result<(String, String), String> {
    let requested_department_id = requested_department_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let requested_agent_id = requested_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_default();
    let department = if let Some(department_id) = requested_department_id.as_deref() {
        department_by_id(config, department_id)
            .ok_or_else(|| format!("路由部门不存在: {department_id}"))?
    } else {
        let agent_id = if !requested_agent_id.is_empty() {
            requested_agent_id.clone()
        } else {
            assistant_department_agent_id(config)
                .ok_or_else(|| "路由信息不完整（缺少 agentId）".to_string())?
        };
        department_for_agent_id(config, &agent_id)
            .or_else(|| assistant_department(config))
            .ok_or_else(|| "路由部门不存在".to_string())?
    };
    let agent_id = if !requested_agent_id.is_empty() {
        requested_agent_id
    } else if requested_department_id.is_some() {
        department
            .agent_ids
            .iter()
            .map(|id| id.trim())
            .find(|id| !id.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| format!("部门没有可用人格：{}", department.id))?
    } else {
        assistant_department_agent_id(config)
            .ok_or_else(|| "路由信息不完整（缺少 agentId）".to_string())?
    };
    if !department
        .agent_ids
        .iter()
        .any(|id| id.trim() == agent_id)
    {
        return Err(format!(
            "agentId 与部门不匹配: agentId={}, departmentId={}",
            agent_id, department.id
        ));
    }
    department_primary_chat_api_config_id(config, department)
        .ok_or_else(|| format!("部门模型未配置或不可用于聊天: {}", department.id))?;
    Ok((department.id.clone(), agent_id))
}

fn validate_enqueue_input(
    input: &RemoteImEnqueueInput,
    config: &AppConfig,
) -> Result<ValidatedEnqueueInput, String> {
    let text = input.payload.text.as_deref().unwrap_or("").trim().to_string();
    let (_channel_id, channel) = resolve_channel_config(input, config)?;
    let images = validate_images(&channel, input);
    let audios = validate_audios(&channel, input);
    let attachments = validate_attachments(&channel, input);
    if text.is_empty() && images.is_empty() && audios.is_empty() && attachments.is_empty() {
        return Err("远程IM消息内容为空".to_string());
    }

    Ok(ValidatedEnqueueInput {
        text,
        images,
        audios,
        attachments,
        channel,
    })
}

fn ensure_remote_im_contact_conversation_id(
    state: &AppState,
    contact: &mut RemoteImContact,
) -> Result<String, String> {
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let binding_pair = match resolve_department_agent_pair(
        contact.bound_department_id.as_deref(),
        contact.bound_agent_id.as_deref(),
        &runtime_snapshot.config,
    ) {
        Ok(pair) => Some(pair),
        Err(err) => {
            runtime_log_warn(format!(
                "[远程IM] 跳过，任务=同步联系人会话绑定，contact_id={}，原因={}",
                contact.id, err
            ));
            None
        }
    };
    if let Some((department_id, agent_id)) = binding_pair.as_ref() {
        contact.bound_department_id = Some(department_id.clone());
        contact.bound_agent_id = Some(agent_id.clone());
    }
    if let Some(bound_conversation_id) = contact
        .bound_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|conversation_id| {
            conversation_service_v2()
                .get_conversation_meta(state, conversation_id)
                .ok()
                .filter(|conversation_meta| {
                    remote_im_meta_is_reusable_active_contact_conversation(conversation_meta)
                })
                .map(|conversation_meta| conversation_meta.id.to_string())
        })
    {
        contact.bound_conversation_id = Some(bound_conversation_id.clone());
        if let Some((department_id, agent_id)) = binding_pair.as_ref() {
            sync_remote_im_contact_conversation_binding(
                state,
                contact,
                &bound_conversation_id,
                department_id,
                agent_id,
            )?;
        }
        return Ok(bound_conversation_id);
    }

    let target_key = remote_im_contact_conversation_key(contact);
    if let Some(found_id) = state_read_chat_index_cached(state)?
        .conversations
        .iter()
        .filter_map(|item| conversation_service_v2().get_conversation_meta(state, item.id.as_str()).ok())
        .find(|conversation_meta| {
            remote_im_meta_is_reusable_active_contact_conversation(conversation_meta)
                && conversation_meta.root_conversation_id.as_deref() == Some(target_key.as_str())
        })
        .map(|conversation_meta| conversation_meta.id.to_string())
    {
        contact.bound_conversation_id = Some(found_id.clone());
        if let Some((department_id, agent_id)) = binding_pair.as_ref() {
            sync_remote_im_contact_conversation_binding(
                state,
                contact,
                &found_id,
                department_id,
                agent_id,
            )?;
        }
        return Ok(found_id);
    }

    let (department_id, agent_id) = binding_pair.unwrap_or_default();
    let conversation = conversation_service_v2().create_remote_im_contact_conversation(
        state,
        &remote_im_contact_conversation_title(contact),
        &department_id,
        &agent_id,
        &target_key,
    )?;
    let conversation_id = conversation.id.clone();
    contact.bound_conversation_id = Some(conversation_id.clone());
    Ok(conversation_id)
}

fn sync_remote_im_contact_conversation_binding(
    state: &AppState,
    contact: &RemoteImContact,
    conversation_id: &str,
    department_id: &str,
    agent_id: &str,
) -> Result<(), String> {
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    if conversation_meta.status.trim() == "archived"
        || conversation_meta
            .archived_at
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
        || conversation_meta.conversation_kind.trim() != CONVERSATION_KIND_REMOTE_IM_CONTACT
    {
        return Ok(());
    }
    let target_key = remote_im_contact_conversation_key(contact);
    let department_changed = conversation_meta.department_id.trim() != department_id;
    let agent_changed = conversation_meta.agent_id.trim() != agent_id;
    let root_changed = conversation_meta.root_conversation_id.as_deref() != Some(target_key.as_str());
    if department_changed || agent_changed || root_changed {
        conversation_service_v2().set_routing(
            state,
            conversation_id,
            Some(department_id),
            Some(agent_id),
            Some(Some(target_key)),
            None,
        )?;
    }
    let preferred_api_changed = conversation_meta
        .preferred_api_config_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    if preferred_api_changed {
        conversation_service_v2().set_preferred_api_config_id(
            state,
            conversation_id,
            None,
        )?;
    }
    Ok(())
}

fn remote_im_meta_is_reusable_active_contact_conversation(
    conversation_meta: &ConversationMetaView,
) -> bool {
    conversation_meta.status.trim() != "archived"
        && conversation_meta
            .archived_at
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none()
        && conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_REMOTE_IM_CONTACT
}

fn resolve_contact_session_target(
    state: &AppState,
    _runtime: &mut RuntimeStateFile,
    contact: &mut RemoteImContact,
) -> Result<(String, String, String), String> {
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let effective_route_mode =
        remote_im_resolve_effective_route_mode(&runtime_snapshot.config, contact);
    contact.route_mode = effective_route_mode.clone();

    let (department_id, agent_id) = resolve_department_agent_pair(
        contact.bound_department_id.as_deref(),
        contact.bound_agent_id.as_deref(),
        &runtime_snapshot.config,
    )?;
    let conversation_id = ensure_remote_im_contact_conversation_id(state, contact)?;
    Ok((department_id, agent_id, conversation_id))
}

fn build_chat_message_from_input(
    input: &RemoteImEnqueueInput,
    conversation_id: &str,
    contact: &RemoteImContact,
    now: &str,
    text: &str,
    images: &[BinaryPart],
    audios: &[BinaryPart],
    attachments: &[AttachmentMetaInput],
    data_path: &PathBuf,
) -> ChatMessage {
    let mut parts = Vec::<MessagePart>::new();
    let contact_id = contact.id.trim();
    let downloads_subdir = remote_im_contact_downloads_subdir(contact);
    if !text.is_empty() {
        parts.push(MessagePart::Text {
            text: text.to_string(),
                reasoning_content: None,
            });
    }
    for img in images {
        let bytes_base64 =
            externalize_stored_binary_base64_in_downloads_subdir(
                data_path,
                &downloads_subdir,
                &img.mime,
                &img.bytes_base64,
            )
                .unwrap_or_else(|err| {
                    eprintln!(
                        "[远程IM] 入站图片外置化失败，保留原始内容: conversation_id={}，contact_id={}，mime={}，bytes_len={}，error={}",
                        conversation_id,
                        contact_id,
                        img.mime,
                        img.bytes_base64.len(),
                        err
                    );
                    img.bytes_base64.clone()
                });
        parts.push(MessagePart::Image {
            mime: img.mime.clone(),
            bytes_base64,
            name: None,
            compressed: false,
        });
    }
    for audio in audios {
        let bytes_base64 =
            externalize_stored_binary_base64_in_downloads_subdir(
                data_path,
                &downloads_subdir,
                &audio.mime,
                &audio.bytes_base64,
            )
                .unwrap_or_else(|err| {
                    eprintln!(
                        "[远程IM] 入站音频外置化失败，保留原始内容: conversation_id={}，contact_id={}，mime={}，bytes_len={}，error={}",
                        conversation_id,
                        contact_id,
                        audio.mime,
                        audio.bytes_base64.len(),
                        err
                    );
                    audio.bytes_base64.clone()
                });
        parts.push(MessagePart::Audio {
            mime: audio.mime.clone(),
            bytes_base64,
            name: None,
            compressed: false,
        });
    }

    let origin_meta = remote_im_set_sender_origin_meta(input, conversation_id, contact_id);
    let mut base_meta = input
        .payload
        .provider_meta
        .clone()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(base_obj) = base_meta.as_object_mut() {
        base_obj.insert("origin".to_string(), origin_meta["origin"].clone());
    } else {
        base_meta = origin_meta;
    }
    let attachment_meta = normalize_payload_attachments(Some(&attachments.to_vec()));
    let merged_meta = merge_provider_meta_with_attachments(Some(base_meta), &attachment_meta);

    ChatMessage {
        id: Uuid::new_v4().to_string(),
        role: "user".to_string(),
        created_at: now.to_string(),
        speaker_agent_id: None,
        parts,
        extra_text_blocks: input.payload.extra_text_blocks.clone().unwrap_or_default(),
        provider_meta: merged_meta,
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    }
}

fn create_pending_event(
    event_id: String,
    conversation_id: String,
    messages: Vec<ChatMessage>,
    activate_assistant: bool,
    session_info: ChatSessionInfo,
    sender_info: RemoteImMessageSource,
) -> ChatPendingEvent {
    ChatPendingEvent {
        id: event_id,
        conversation_id,
        created_at: now_iso(),
        source: ChatEventSource::RemoteIm,
        queue_mode: ChatQueueMode::Normal,
        messages,
        activate_assistant,
        assistant_message_id: None,
        session_info,
        runtime_context: None,
        sender_info: Some(sender_info),
    }
}

#[tauri::command]
fn remote_im_list_channels(state: State<'_, AppState>) -> Result<Vec<RemoteImChannelConfig>, String> {
    let config = state_read_config_cached(&state)?;
    Ok(config.remote_im_channels)
}

#[tauri::command]
fn remote_im_list_contacts(state: State<'_, AppState>) -> Result<Vec<RemoteImContact>, String> {
    let runtime = state_read_runtime_state_cached(&state)?;
    let mut contacts = runtime.remote_im_contacts;
    contacts.sort_by(|a, b| {
        a.channel_id
            .cmp(&b.channel_id)
            .then_with(|| b.last_message_at.cmp(&a.last_message_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    Ok(contacts)
}

#[tauri::command]
fn remote_im_update_contact_allow_send(
    input: RemoteImContactAllowSendUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send = input.allow_send;
    contact.allow_receive = input.allow_send;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_allow_send_files(
    input: RemoteImContactAllowSendFilesUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send_files = input.allow_send_files;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_allow_receive(
    input: RemoteImContactAllowReceiveUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_receive = input.allow_receive;
    contact.allow_send = input.allow_receive;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_activation(
    input: RemoteImContactActivationUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.activation_mode = normalize_contact_activation_mode(&input.activation_mode);
    contact.activation_keywords = normalize_contact_activation_keywords(&input.activation_keywords);
    contact.mute_keywords = normalize_contact_keyword_list(&input.mute_keywords);
    contact.unmute_keywords = normalize_contact_keyword_list(&input.unmute_keywords);
    contact.patience_seconds = input.patience_seconds;
    contact.mute_duration_seconds = input.mute_duration_seconds;
    contact.activation_cooldown_seconds = input.activation_cooldown_seconds;
    contact.response_strategy = normalize_contact_response_strategy(&input.response_strategy);
    contact.response_guidance = normalize_contact_response_guidance(&input.response_guidance);
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_remark(
    input: RemoteImContactRemarkUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.remark_name = input.remark_name.trim().to_string();
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_route_mode(
    input: RemoteImContactRouteModeUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let config = state_read_config_cached(&state)?;
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    let requested_mode = normalize_contact_route_mode(&input.route_mode);
    let final_mode = remote_im_resolve_effective_route_mode(&config, contact);
    if requested_mode != final_mode {
        eprintln!(
            "[远程IM] 联系人路由模式已被约束修正: contact_id={}, requested={}, final={}",
            contact.id, requested_mode, final_mode
        );
    }
    contact.route_mode = final_mode;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_department_binding(
    input: RemoteImContactDepartmentBindingUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let runtime_snapshot = load_runtime_organization_snapshot(state.inner())?;
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    let next_department_id = input
        .department_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let next_agent_id = input
        .agent_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if next_department_id.is_some() != next_agent_id.is_some() {
        return Err("远程IM绑定部门和人格必须同时提供".to_string());
    }
    let next_pair = if let Some(department_id) = next_department_id.as_deref() {
        let pair = resolve_department_agent_pair(
            Some(department_id),
            next_agent_id.as_deref(),
            &runtime_snapshot.config,
        )?;
        if !runtime_snapshot
            .agents
            .iter()
            .any(|agent| agent.id == pair.1 && !agent.is_built_in_user)
        {
            return Err(format!("路由人格不存在或不可用: {}", pair.1));
        }
        Some(pair)
    } else {
        None
    };
    contact.bound_department_id = next_pair
        .as_ref()
        .map(|(department_id, _)| department_id.clone());
    contact.bound_agent_id = next_pair.as_ref().map(|(_, agent_id)| agent_id.clone());
    contact.route_mode =
        remote_im_resolve_effective_route_mode(&runtime_snapshot.config, contact);
    let conversation_id = ensure_remote_im_contact_conversation_id(state.inner(), contact)?;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    eprintln!(
        "[远程IM] 完成，任务=更新联系人处理部门，contact_id={}，conversation_id={}，department_id={}，agent_id={}",
        output.id,
        conversation_id,
        output
            .bound_department_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(""),
        conversation_service_v2()
            .get_conversation_meta(state.inner(), &conversation_id)
            .map(|conversation| conversation.agent_id)
            .unwrap_or_default()
    );
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_processing_mode(
    input: RemoteImContactProcessingModeUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.processing_mode = normalize_contact_processing_mode(&input.processing_mode);
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_workspace(
    input: RemoteImContactWorkspaceUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.shell_workspaces = input.shell_workspaces;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    if let Some(conversation_id) = output
        .bound_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        mark_prompt_cache_rebuild_for_system_environment_by_conversation(&state, conversation_id);
    }
    Ok(output)
}

#[tauri::command]
fn remote_im_list_contact_conversations(
    state: State<'_, AppState>,
) -> Result<Vec<RemoteImContactConversationSummary>, String> {
    let started_at = std::time::Instant::now();
    runtime_log_debug("[远程IM][联系人会话][列表] 开始".to_string());
    let items = conversation_service_v2().list_remote_im_contact_conversations(state.inner())?;
    runtime_log_debug(format!(
        "[远程IM][联系人会话][列表] 完成: contact_count={}, elapsed_ms={}",
        items.len(),
        started_at.elapsed().as_millis()
    ));
    Ok(items)
}

#[tauri::command]
fn remote_im_get_contact_conversation_messages(
    input: RemoteImContactConversationMessagesInput,
    state: State<'_, AppState>,
) -> Result<Vec<ChatMessage>, String> {
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let started_at = std::time::Instant::now();
    runtime_log_debug(format!(
        "[远程IM][联系人会话][读取] 开始: contact_id={}",
        contact_id
    ));
    let messages = conversation_service_v2()
        .get_remote_im_contact_conversation_messages(state.inner(), contact_id)?;
    runtime_log_debug(format!(
        "[远程IM][联系人会话][读取] 完成: contact_id={}, message_count={}, elapsed_ms={}",
        contact_id,
        messages.len(),
        started_at.elapsed().as_millis()
    ));
    Ok(messages)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactConversationBlockPageInput {
    contact_id: String,
    #[serde(default)]
    block_id: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactConversationBlockSummaryOutput {
    block_id: u32,
    message_count: usize,
    first_message_id: String,
    last_message_id: String,
    first_created_at: Option<String>,
    last_created_at: Option<String>,
    is_latest: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteImContactConversationBlockPageOutput {
    blocks: Vec<RemoteImContactConversationBlockSummaryOutput>,
    selected_block_id: u32,
    messages: Vec<ChatMessage>,
    has_prev_block: bool,
    has_next_block: bool,
}

#[tauri::command]
fn remote_im_get_contact_conversation_block_page(
    input: RemoteImContactConversationBlockPageInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContactConversationBlockPageOutput, String> {
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let started_at = std::time::Instant::now();
    runtime_log_debug(format!(
        "[远程IM][联系人会话][块分页] 开始: contact_id={}, requested_block_id={}",
        contact_id,
        input.block_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "latest".to_string())
    ));
    let page = conversation_service_v2().get_remote_im_contact_conversation_block_page(
        state.inner(),
        contact_id,
        input.block_id,
    )?;
    runtime_log_debug(format!(
        "[远程IM][联系人会话][块分页] 完成: contact_id={}, selected_block_id={}, message_count={}, elapsed_ms={}",
        contact_id,
        page.selected_block_id,
        page.messages.len(),
        started_at.elapsed().as_millis()
    ));
    Ok(RemoteImContactConversationBlockPageOutput {
        blocks: page
            .blocks
            .into_iter()
            .map(|item| RemoteImContactConversationBlockSummaryOutput {
                block_id: item.block_id,
                message_count: item.message_count,
                first_message_id: item.first_message_id,
                last_message_id: item.last_message_id,
                first_created_at: item.first_created_at,
                last_created_at: item.last_created_at,
                is_latest: item.is_latest,
            })
            .collect(),
        selected_block_id: page.selected_block_id,
        messages: page.messages,
        has_prev_block: page.has_prev_block,
        has_next_block: page.has_next_block,
    })
}

#[tauri::command]
fn remote_im_delete_contact(
    input: RemoteImContactDeleteInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let mut runtime = state_read_runtime_state_cached(&state)?;
    let before_contacts = runtime.remote_im_contacts.len();
    runtime.remote_im_contacts
        .retain(|item| item.id != contact_id);
    let removed = runtime.remote_im_contacts.len() != before_contacts;
    if removed {
        state_write_runtime_state_cached(&state, &runtime)?;
    }
    Ok(removed)
}

#[tauri::command]
fn remote_im_clear_contact_conversation(
    input: RemoteImContactDeleteInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let started_at = std::time::Instant::now();
    eprintln!(
        "[远程IM][联系人会话][清空] 开始: contact_id={}",
        contact_id
    );
    let cleared =
        conversation_service_v2().clear_remote_im_contact_conversation(state.inner(), contact_id)?;
    eprintln!(
        "[远程IM][联系人会话][清空] 完成: contact_id={}, elapsed_ms={}",
        contact_id,
        started_at.elapsed().as_millis()
    );
    Ok(cleared)
}

#[tauri::command]
fn remote_im_enqueue_message(
    input: RemoteImEnqueueInput,
    state: State<'_, AppState>,
) -> Result<RemoteImEnqueueResult, String> {
    remote_im_enqueue_message_internal(input, state.inner())
}

/// 内部入队函数，供事件消费循环调用
pub(crate) fn remote_im_enqueue_message_internal(
    input: RemoteImEnqueueInput,
    state: &AppState,
) -> Result<RemoteImEnqueueResult, String> {
    let config = state_read_config_cached(state)?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let validated = validate_enqueue_input(&input, &config)?;
    let channel = validated.channel;
    let text = validated.text;
    let images = validated.images;
    let audios = validated.audios;
    let attachments = validated.attachments;

    let now = now_iso();
    let contact_id = remote_im_upsert_contact_for_inbound(&mut runtime, &channel, &input, &now);
    let contact_idx = runtime
        .remote_im_contacts
        .iter()
        .position(|item| item.id == contact_id)
        .ok_or_else(|| format!("联系人不存在: {contact_id}"))?;
    let contact = runtime
        .remote_im_contacts
        .get_mut(contact_idx)
        .ok_or_else(|| format!("联系人不存在: {contact_id}"))?;
    let mut allow_receive = contact.allow_receive;
    if !allow_receive
        && matches!(channel.platform, RemoteImPlatform::Dingtalk)
        && channel.activate_assistant
    {
        let looks_like_default_contact = !contact.allow_send
            && !contact.allow_receive
            && contact.activation_mode == "never"
            && contact.activation_keywords.is_empty()
            && contact.activation_cooldown_seconds == 0;
        if looks_like_default_contact {
            contact.allow_send = true;
            contact.allow_receive = true;
            eprintln!(
                "[远程IM] 自动开启收信: contact_id={}, contact_name={}, channel_id={}, platform={:?}, reason=matched_default_contact",
                contact.id,
                contact.remote_contact_name,
                channel.id,
                channel.platform
            );
            allow_receive = true;
        }
    }
    if !allow_receive {
        state_write_runtime_state_cached(state, &runtime)?;
        return Err(format!("联系人未开启收信，跳过: contact_id={contact_id}"));
    }
    let (department_id, agent_id, conversation_id) = {
        let mut detached_contact = runtime
            .remote_im_contacts
            .get(contact_idx)
            .cloned()
            .ok_or_else(|| format!("联系人不存在: {contact_id}"))?;
        let resolved = resolve_contact_session_target(state, &mut runtime, &mut detached_contact)?;
        runtime.remote_im_contacts[contact_idx] = detached_contact;
        resolved
    };
    let contact_for_log = runtime
        .remote_im_contacts
        .get(contact_idx)
        .cloned()
        .ok_or_else(|| format!("联系人不存在: {contact_id}"))?;
    let sender_label = {
        let sender_name = input.sender_name.trim();
        let sender_id = input.sender_id.trim();
        if sender_name.is_empty() {
            sender_id.to_string()
        } else if sender_id.is_empty() {
            sender_name.to_string()
        } else {
            format!("{}({})", sender_name, sender_id)
        }
    };
    eprintln!(
        "[远程IM] 入站消息路由完成: contact_id={}, channel_id={}, department_id={}, agent_id={}, conversation_id={}, route_mode={}, processing_mode={}",
        contact_id,
        input.channel_id.trim(),
        department_id,
        agent_id,
        conversation_id,
        runtime.remote_im_contacts[contact_idx].route_mode,
        runtime.remote_im_contacts[contact_idx].processing_mode
    );
    eprintln!(
        "[远程IM] 入站媒体摘要: contact_id={}, channel_id={}, text_len={}, image_count={}, image_mimes={:?}, audio_count={}, attachment_count={}, attachment_names={:?}",
        contact_id,
        input.channel_id.trim(),
        text.chars().count(),
        images.len(),
        images.iter().map(|item| item.mime.clone()).collect::<Vec<_>>(),
        audios.len(),
        attachments.len(),
        attachments.iter().map(|item| item.file_name.clone()).collect::<Vec<_>>()
    );
    remote_im_append_channel_log(
        input.channel_id.trim(),
        "info",
        format!(
            "[联系人消息] 收到: contact={}, sender={}, conversation_id={}, text_len={}, image_count={}, audio_count={}, attachment_count={}, preview={}",
            remote_im_contact_log_label(&contact_for_log),
            sender_label,
            conversation_id,
            text.chars().count(),
            images.len(),
            audios.len(),
            attachments.len(),
            remote_im_preview_text(&text, 100)
        ),
    );
    if let Some(platform_message_id) = input
        .platform_message_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if remote_im_is_duplicate_platform_message(
            state,
            &conversation_id,
            input.channel_id.trim(),
            input.remote_contact_type.trim(),
            input.remote_contact_id.trim(),
            platform_message_id,
        )? {
            eprintln!(
                "[远程IM] 入站消息去重: channel_id={}, contact_id={}, conversation_id={}, platform_message_id={}",
                input.channel_id.trim(),
                input.remote_contact_id.trim(),
                conversation_id,
                platform_message_id
            );
            remote_im_append_channel_log(
                input.channel_id.trim(),
                "info",
                format!(
                    "[联系人消息] 去重跳过: contact={}, conversation_id={}, platform_message_id={}, preview={}",
                    remote_im_contact_log_label(&contact_for_log),
                    conversation_id,
                    platform_message_id,
                    remote_im_preview_text(&text, 100)
                ),
            );
            state_write_runtime_state_cached(state, &runtime)?;
            return Ok(RemoteImEnqueueResult {
                event_id: String::new(),
                conversation_id,
                activate_assistant: false,
                contact_id,
            });
        }
    }
    let message = build_chat_message_from_input(
        &input,
        &conversation_id,
        &runtime.remote_im_contacts[contact_idx],
        &now,
        &text,
        &images,
        &audios,
        &attachments,
        &state.data_path,
    );
    let guided_candidate_message = message.clone();

    let (activate_assistant, state_reason) = remote_im_prepare_enqueue_runtime_state(
        state,
        &runtime.remote_im_contacts[contact_idx],
        &text,
    )?;
    let should_handle_busy_guided_entry = {
        let runtime_states = lock_remote_im_contact_runtime_states(state)?;
        runtime_states
            .get(&contact_id)
            .map(|item| {
                item.presence_state == RemoteImPresenceState::Present
                    && item.work_state == RemoteImWorkState::Busy
                    && item.has_pending
            })
            .unwrap_or(false)
    };
    eprintln!(
        "[远程联系人状态机] 入站消息 接入: contact_id={}, conversation_id={}, activate_assistant={}, reason={}",
        contact_id, conversation_id, activate_assistant, state_reason
    );

    let event_id = Uuid::new_v4().to_string();
    let new_sender_id_for_guided = input.sender_id.trim().to_string();
    let event = create_pending_event(
        event_id.clone(),
        conversation_id.clone(),
        vec![message],
        activate_assistant,
        ChatSessionInfo {
            department_id,
            agent_id,
        },
        RemoteImMessageSource {
            channel_id: input.channel_id.trim().to_string(),
            platform: input.platform,
            im_name: input.im_name,
            remote_contact_type: input.remote_contact_type,
            remote_contact_id: input.remote_contact_id,
            remote_contact_name: input.remote_contact_name.unwrap_or_default(),
            sender_id: input.sender_id,
            sender_name: input.sender_name,
            sender_avatar_url: input.sender_avatar_url,
            platform_message_id: input.platform_message_id,
        },
    );
    let ingress = ingress_chat_event(state, event)?;
    let ingress_mode = match &ingress {
        ChatEventIngress::Direct(_) => "direct",
        ChatEventIngress::Queued { .. } => "queued",
        ChatEventIngress::Duplicate { .. } => "duplicate",
    };
    remote_im_append_channel_log(
        input.channel_id.trim(),
        "info",
        format!(
            "[联系人消息] 入队: contact={}, conversation_id={}, event_id={}, mode={}, activate={}, reason={}",
            remote_im_contact_log_label(&contact_for_log),
            conversation_id,
            event_id,
            ingress_mode,
            remote_im_yes_no(activate_assistant),
            state_reason
        ),
    );
    let busy_guided_candidate =
        should_handle_busy_guided_entry && matches!(&ingress, ChatEventIngress::Queued { .. });
    if busy_guided_candidate {
        let state_clone = state.clone();
        let contact_for_guided = runtime.remote_im_contacts[contact_idx].clone();
        let channel_id_for_guided = input.channel_id.trim().to_string();
        let conversation_id_for_guided = conversation_id.clone();
        let event_id_for_guided = event_id.clone();
        let contact_log_label = remote_im_contact_log_label(&contact_for_log);
        let guided_candidate_message = guided_candidate_message.clone();
        tokio::spawn(async move {
            let agents = state_read_agents_cached(&state_clone).unwrap_or_default();
            let current_assistant = match remote_im_secretary_current_assistant_context(
                &state_clone,
                &conversation_id_for_guided,
            )
            .or_else(|_| remote_im_resolve_contact_assistant_context(&state_clone, &contact_for_guided))
            {
                Ok(value) => value,
                Err(err) => {
                    remote_im_append_channel_log(
                        &channel_id_for_guided,
                        "warn",
                        format!(
                            "[联系人秘书] 忙碌引导跳过: contact={}, conversation_id={}, event_id={}, error={}",
                            contact_log_label, conversation_id_for_guided, event_id_for_guided, err
                        ),
                    );
                    return;
                }
            };
            let current_sender_id = remote_im_latest_group_sender_id_for_busy_guided(
                &state_clone,
                &conversation_id_for_guided,
                &contact_for_guided,
            )
            .ok()
            .flatten();
            if !remote_im_busy_guided_same_sender_allowed(
                &contact_for_guided,
                current_sender_id.as_deref(),
                &new_sender_id_for_guided,
            ) {
                remote_im_append_channel_log(
                    &channel_id_for_guided,
                    "info",
                    format!(
                        "[联系人秘书] 忙碌引导跳过: contact={}, conversation_id={}, event_id={}, reason=different_group_sender, current_sender_id={}, new_sender_id={}",
                        contact_log_label,
                        conversation_id_for_guided,
                        event_id_for_guided,
                        current_sender_id.as_deref().unwrap_or(""),
                        new_sender_id_for_guided
                    ),
                );
                return;
            }
            let current_work_message = remote_im_latest_secretary_work_message_for_busy_guided(
                &state_clone,
                &conversation_id_for_guided,
                &contact_for_guided,
                &agents,
                &current_assistant,
            )
            .ok()
            .flatten();
            let Some(new_message_digest) = remote_im_secretary_message_digest(
                &guided_candidate_message,
                &contact_for_guided,
                &agents,
                &current_assistant,
            ) else {
                return;
            };
            match run_remote_im_secretary_guided_decision(
                &state_clone,
                &contact_for_guided,
                &current_assistant,
                current_work_message.as_ref(),
                &new_message_digest,
            )
            .await
            {
                Ok(decision) => {
                    let should_upgrade_guided = match remote_im_secretary_should_upgrade_guided(
                        &state_clone,
                        &conversation_id_for_guided,
                    ) {
                        Ok(value) => value,
                        Err(err) => {
                            remote_im_append_channel_log(
                                &channel_id_for_guided,
                                "warn",
                                format!(
                                    "[联系人秘书] 忙碌引导状态判断失败: contact={}, conversation_id={}, event_id={}, error={}",
                                    contact_log_label,
                                    conversation_id_for_guided,
                                    event_id_for_guided,
                                    err
                                ),
                            );
                            false
                        }
                    };
                    if decision.emit_log {
                        remote_im_append_channel_log(
                            &channel_id_for_guided,
                            "info",
                            format!(
                                "[联系人秘书] 忙碌引导判断: contact={}, conversation_id={}, result={}, route={}, reason={}",
                                contact_log_label,
                                conversation_id_for_guided,
                                if decision.should_interrupt { "需要优先处理" } else { "继续排队" },
                                if decision.should_interrupt {
                                    if should_upgrade_guided {
                                        "升级引导"
                                    } else {
                                        "直接激活"
                                    }
                                } else {
                                    "继续排队"
                                },
                                decision.reason
                            ),
                        );
                    }
                    if decision.should_interrupt {
                        if should_upgrade_guided {
                            if let Err(err) = mark_queue_event_guided_with_log(
                                &state_clone,
                                &event_id_for_guided,
                                decision.emit_log,
                            ) {
                                remote_im_append_channel_log(
                                    &channel_id_for_guided,
                                    "warn",
                                    format!(
                                        "[联系人秘书] 忙碌引导升级失败: contact={}, conversation_id={}, event_id={}, error={}",
                                        contact_log_label,
                                        conversation_id_for_guided,
                                        event_id_for_guided,
                                        err
                                    ),
                                );
                            }
                        } else {
                            trigger_chat_queue_processing(&state_clone);
                        }
                    }
                }
                Err(err) => {
                    remote_im_append_channel_log(
                        &channel_id_for_guided,
                        "warn",
                        format!(
                            "[联系人秘书] 忙碌引导判断失败: contact={}, conversation_id={}, event_id={}, error={}",
                            contact_log_label,
                            conversation_id_for_guided,
                            event_id_for_guided,
                            err
                        ),
                    );
                }
            }
        });
    }
    state_write_runtime_state_cached(state, &runtime)?;
    trigger_chat_event_after_ingress(state, ingress);
    Ok(RemoteImEnqueueResult {
        event_id,
        conversation_id,
        activate_assistant,
        contact_id,
    })
}
