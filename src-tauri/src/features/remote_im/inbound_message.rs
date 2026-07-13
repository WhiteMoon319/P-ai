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
                    runtime_log_error(format!(
                        "[远程IM] 入站图片外置化失败，保留原始内容: conversation_id={}，contact_id={}，mime={}，bytes_len={}，error={}",
                        conversation_id,
                        contact_id,
                        img.mime,
                        img.bytes_base64.len(),
                        err
                    ));
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
                    runtime_log_error(format!(
                        "[远程IM] 入站音频外置化失败，保留原始内容: conversation_id={}，contact_id={}，mime={}，bytes_len={}，error={}",
                        conversation_id,
                        contact_id,
                        audio.mime,
                        audio.bytes_base64.len(),
                        err
                    ));
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
    let queue_mode = if activate_assistant && sender_info.remote_contact_type.trim().eq_ignore_ascii_case("private")
    {
        ChatQueueMode::Guided
    } else {
        ChatQueueMode::Normal
    };
    ChatPendingEvent {
        id: event_id,
        conversation_id,
        created_at: now_iso(),
        source: ChatEventSource::RemoteIm,
        queue_mode,
        messages,
        activate_assistant,
        assistant_message_id: None,
        session_info,
        runtime_context: None,
        sender_info: Some(sender_info),
    }
}
