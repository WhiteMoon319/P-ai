fn remote_im_list_channels_inner(state: &AppState) -> Result<Vec<RemoteImChannelConfig>, String> {
    let config = state_read_config_cached(state)?;
    Ok(config.remote_im_channels)
}

#[tauri::command]
fn remote_im_list_channels(state: State<'_, AppState>) -> Result<Vec<RemoteImChannelConfig>, String> {
    remote_im_list_channels_inner(state.inner())
}

fn remote_im_list_contacts_inner(state: &AppState) -> Result<Vec<RemoteImContact>, String> {
    let runtime = state_read_runtime_state_cached(state)?;
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
fn remote_im_list_contacts(state: State<'_, AppState>) -> Result<Vec<RemoteImContact>, String> {
    remote_im_list_contacts_inner(state.inner())
}

#[tauri::command]
fn remote_im_get_default_group_response_guidance() -> String {
    default_remote_im_contact_response_guidance()
}

fn remote_im_update_contact_allow_send_inner(
    state: &AppState,
    input: RemoteImContactAllowSendUpdateInput,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send = input.allow_send;
    contact.allow_receive = input.allow_send;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_allow_send(
    input: RemoteImContactAllowSendUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_allow_send_inner(state.inner(), input)
}

fn remote_im_update_contact_allow_send_files_inner(
    state: &AppState,
    input: RemoteImContactAllowSendFilesUpdateInput,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send_files = input.allow_send_files;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_allow_send_files(
    input: RemoteImContactAllowSendFilesUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_allow_send_files_inner(state.inner(), input)
}

fn remote_im_update_contact_blocked_message_prefixes_inner(
    state: &AppState,
    input: RemoteImContactBlockedMessagePrefixesUpdateInput,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.blocked_message_prefixes =
        normalize_contact_blocked_message_prefixes(&input.blocked_message_prefixes);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_blocked_message_prefixes(
    input: RemoteImContactBlockedMessagePrefixesUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_blocked_message_prefixes_inner(state.inner(), input)
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

fn remote_im_update_contact_activation_inner(
    state: &AppState,
    input: RemoteImContactActivationUpdateInput,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
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
    if !remote_im_contact_is_private(contact) {
        contact.response_strategy = normalize_contact_response_strategy(&input.response_strategy);
    }
    contact.response_guidance = normalize_contact_response_guidance(&input.response_guidance);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_activation(
    input: RemoteImContactActivationUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_activation_inner(state.inner(), input)
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
        runtime_log_info(format!(
            "[远程IM] 联系人路由模式已被约束修正: contact_id={}, requested={}, final={}",
            contact.id, requested_mode, final_mode
        ));
    }
    contact.route_mode = final_mode;
    let output = contact.clone();
    state_write_runtime_state_cached(&state, &runtime)?;
    Ok(output)
}

fn remote_im_update_contact_department_binding_inner(
    state: &AppState,
    input: RemoteImContactDepartmentBindingUpdateInput,
) -> Result<RemoteImContact, String> {
    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let mut runtime = state_read_runtime_state_cached(state)?;
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
    let conversation_id = ensure_remote_im_contact_conversation_id(state, contact)?;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    runtime_log_info(format!(
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
            .get_conversation_meta(state, &conversation_id)
            .map(|conversation| conversation.agent_id)
            .unwrap_or_default()
    ));
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_department_binding(
    input: RemoteImContactDepartmentBindingUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_department_binding_inner(state.inner(), input)
}

fn remote_im_update_contact_processing_mode_inner(
    state: &AppState,
    input: RemoteImContactProcessingModeUpdateInput,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.processing_mode = normalize_contact_processing_mode(&input.processing_mode);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_processing_mode(
    input: RemoteImContactProcessingModeUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_processing_mode_inner(state.inner(), input)
}

fn remote_im_update_contact_workspace_inner(
    state: &AppState,
    input: RemoteImContactWorkspaceUpdateInput,
) -> Result<RemoteImContact, String> {
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.shell_workspaces = input.shell_workspaces;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    if let Some(conversation_id) = output
        .bound_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        mark_prompt_cache_rebuild_for_system_environment_by_conversation(state, conversation_id);
    }
    Ok(output)
}

#[tauri::command]
fn remote_im_update_contact_workspace(
    input: RemoteImContactWorkspaceUpdateInput,
    state: State<'_, AppState>,
) -> Result<RemoteImContact, String> {
    remote_im_update_contact_workspace_inner(state.inner(), input)
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
    remote_im_get_contact_conversation_block_page_inner(input, state.inner())
}

fn remote_im_get_contact_conversation_block_page_inner(
    input: RemoteImContactConversationBlockPageInput,
    state: &AppState,
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
        state,
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

fn remote_im_delete_contact_inner(
    state: &AppState,
    input: RemoteImContactDeleteInput,
) -> Result<bool, String> {
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let mut runtime = state_read_runtime_state_cached(state)?;
    let before_contacts = runtime.remote_im_contacts.len();
    runtime.remote_im_contacts
        .retain(|item| item.id != contact_id);
    let removed = runtime.remote_im_contacts.len() != before_contacts;
    if removed {
        state_write_runtime_state_cached(state, &runtime)?;
    }
    Ok(removed)
}

#[tauri::command]
fn remote_im_delete_contact(
    input: RemoteImContactDeleteInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    remote_im_delete_contact_inner(state.inner(), input)
}

#[tauri::command]
fn remote_im_clear_contact_conversation(
    input: RemoteImContactDeleteInput,
    state: State<'_, AppState>,
) -> Result<bool, String> {
    remote_im_clear_contact_conversation_inner(input, state.inner())
}

fn remote_im_clear_contact_conversation_inner(
    input: RemoteImContactDeleteInput,
    state: &AppState,
) -> Result<bool, String> {
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[远程IM][联系人会话][清空] 开始: contact_id={}",
        contact_id
    ));
    let cleared =
        conversation_service_v2().clear_remote_im_contact_conversation(state, contact_id)?;
    runtime_log_info(format!(
        "[远程IM][联系人会话][清空] 完成: contact_id={}, elapsed_ms={}",
        contact_id,
        started_at.elapsed().as_millis()
    ));
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

    let existing_contact = remote_im_find_contact_for_inbound(&runtime, &input);
    let blocked_prefixes = existing_contact
        .map(|contact| contact.blocked_message_prefixes.clone())
        .unwrap_or_else(default_remote_im_contact_blocked_message_prefixes);
    if let Some(prefix) = remote_im_blocked_inbound_message_prefix(&text, &blocked_prefixes) {
        let contact_id = existing_contact
            .map(|contact| contact.id.clone())
            .unwrap_or_default();
        let contact_label = existing_contact
            .map(remote_im_contact_log_label)
            .unwrap_or_else(|| {
                format!(
                    "{}[{}:{}]",
                    input.remote_contact_name.as_deref().unwrap_or_default().trim(),
                    input.remote_contact_type.trim(),
                    input.remote_contact_id.trim()
                )
            });
        runtime_log_info(format!(
            "[远程IM] 入站消息跳过: contact_id={}, channel_id={}, 原因=命中消息头过滤, 过滤前缀={}, 文本长度={}",
            contact_id,
            input.channel_id.trim(),
            prefix,
            text.chars().count()
        ));
        remote_im_append_channel_log(
            input.channel_id.trim(),
            "info",
            format!(
                "[联系人消息] 过滤跳过: contact={}, prefix={}, text_len={}",
                contact_label,
                prefix,
                text.chars().count()
            ),
        );
        return Ok(RemoteImEnqueueResult {
            event_id: String::new(),
            conversation_id: String::new(),
            activate_assistant: false,
            contact_id,
        });
    }

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
            runtime_log_info(format!(
                "[远程IM] 自动开启收信: contact_id={}, contact_name={}, channel_id={}, platform={:?}, reason=matched_default_contact",
                contact.id,
                contact.remote_contact_name,
                channel.id,
                channel.platform
            ));
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
    runtime_log_info(format!(
        "[远程IM] 入站消息路由完成: contact_id={}, channel_id={}, department_id={}, agent_id={}, conversation_id={}, route_mode={}, processing_mode={}",
        contact_id,
        input.channel_id.trim(),
        department_id,
        agent_id,
        conversation_id,
        runtime.remote_im_contacts[contact_idx].route_mode,
        runtime.remote_im_contacts[contact_idx].processing_mode
    ));
    runtime_log_info(format!(
        "[远程IM] 入站媒体摘要: contact_id={}, channel_id={}, text_len={}, image_count={}, image_mimes={:?}, audio_count={}, attachment_count={}, attachment_names={:?}",
        contact_id,
        input.channel_id.trim(),
        text.chars().count(),
        images.len(),
        images.iter().map(|item| item.mime.clone()).collect::<Vec<_>>(),
        audios.len(),
        attachments.len(),
        attachments.iter().map(|item| item.file_name.clone()).collect::<Vec<_>>()
    ));
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
            runtime_log_info(format!(
                "[远程IM] 入站消息去重: channel_id={}, contact_id={}, conversation_id={}, platform_message_id={}",
                input.channel_id.trim(),
                input.remote_contact_id.trim(),
                conversation_id,
                platform_message_id
            ));
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
    let (activate_assistant, state_reason) = remote_im_prepare_enqueue_runtime_state(
        state,
        &runtime.remote_im_contacts[contact_idx],
        &text,
    )?;
    runtime_log_info(format!(
        "[远程联系人状态机] 入站消息 接入: contact_id={}, conversation_id={}, activate_assistant={}, reason={}",
        contact_id, conversation_id, activate_assistant, state_reason
    ));

    let event_id = Uuid::new_v4().to_string();
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
    let should_trigger_guided_queue = event.queue_mode == ChatQueueMode::Guided;
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
    state_write_runtime_state_cached(state, &runtime)?;
    if should_trigger_guided_queue && matches!(&ingress, ChatEventIngress::Queued { .. }) {
        trigger_guided_queue_processing(state, &conversation_id);
    } else {
        trigger_chat_event_after_ingress(state, ingress);
    }
    Ok(RemoteImEnqueueResult {
        event_id,
        conversation_id,
        activate_assistant,
        contact_id,
    })
}
