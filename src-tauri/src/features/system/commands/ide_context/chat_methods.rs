fn ide_chat_runtime_for_conversation(
    state: &AppState,
    conversation_id: &str,
) -> Option<ConversationRuntimeSnapshot> {
    read_conversation_runtime_snapshot(state, conversation_id).ok()
}

fn ide_chat_sidebar_window_label(client_id: &str) -> String {
    format!("vscode-sidebar:{}", client_id.trim())
}

fn ide_chat_emit_overview_updated(state: &AppState) -> Result<(), String> {
    let overview_payload = conversation_service_v2().refresh_unarchived_conversation_overview(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    Ok(())
}

fn ide_chat_release_sidebar_conversation(
    state: &AppState,
    sidebar_label: &str,
) -> Result<(), String> {
    if let Some(client_id) = ide_chat_sidebar_client_id_from_label(sidebar_label) {
        if let Ok(mut conversations) = ide_context_chat_client_conversations().lock() {
            conversations.remove(&client_id);
        }
    }
    if unregister_detached_chat_window_by_label(sidebar_label).is_some() {
        ide_chat_emit_overview_updated(state)?;
    }
    Ok(())
}

fn ide_chat_register_sidebar_conversation(
    state: &AppState,
    conversation_id: &str,
    sidebar_label: &str,
    opened_conversation_id: &mut Option<String>,
) -> Result<(), String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    if conversation_meta.id.trim() == SYSTEM_NOTIFICATION_CONVERSATION_ID
        || conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_SYSTEM_NOTIFICATION
    {
        if opened_conversation_id.as_deref() != Some(conversation_id) {
            ide_chat_release_sidebar_conversation(state, sidebar_label)?;
        }
        if let Some(client_id) = ide_chat_sidebar_client_id_from_label(sidebar_label) {
            if let Ok(mut conversations) = ide_context_chat_client_conversations().lock() {
                conversations.remove(&client_id);
            }
        }
        *opened_conversation_id = Some(conversation_id.to_string());
        return Ok(());
    }
    if opened_conversation_id.as_deref() != Some(conversation_id) {
        ide_chat_release_sidebar_conversation(state, sidebar_label)?;
    }
    register_detached_chat_window(conversation_id, sidebar_label)?;
    if let Some(client_id) = ide_chat_sidebar_client_id_from_label(sidebar_label) {
        if let Ok(mut conversations) = ide_context_chat_client_conversations().lock() {
            conversations.insert(client_id, conversation_id.to_string());
        }
    }
    *opened_conversation_id = Some(conversation_id.to_string());
    ide_chat_emit_overview_updated(state)?;
    Ok(())
}

fn ide_chat_conversation_open_result(state: &AppState, conversation_id: &str) -> Result<Value, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    if conversation_meta.status.trim() == "archived"
        || conversation_meta
            .archived_at
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
    {
        return Err("conversation is archived".to_string());
    }
    let messages = conversation_service_v2().get_recent_messages_for_frontend_display_only(
        state,
        conversation_id,
        DEFAULT_FOREGROUND_SNAPSHOT_RECENT_LIMIT,
    )?;
    let runtime = ide_chat_runtime_for_conversation(state, conversation_id);
    let persona = ide_chat_persona_payload(state, Some(conversation_meta.agent_id.as_str()))?;
    let conversation = ide_chat_conversation_from_meta_view(&conversation_meta);
    let model = ide_chat_model_payload_for_conversation(state, &conversation)?;
    Ok(serde_json::json!({
        "conversationId": conversation_meta.id,
        "title": conversation_meta.title,
        "agentId": conversation_meta.agent_id,
        "departmentId": conversation_meta.department_id,
        "updatedAt": conversation_meta.updated_at,
        "messages": messages,
        "runtime": runtime,
        "persona": persona,
        "model": model,
        "currentTodos": conversation_meta.current_todos,
        "activeGoal": conversation_meta.active_goal,
    }))
}

fn ide_chat_ensure_sidebar_workspace(
    state: &AppState,
    conversation_id: &str,
    workspace_path: &str,
    _workspace_name: Option<&str>,
) -> Result<(), String> {
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let mut workspaces = conversation_meta.shell_workspaces.clone();
    let has_main = workspaces.iter().any(|ws| {
        normalize_shell_workspace_level_text(&ws.level) == SHELL_WORKSPACE_LEVEL_MAIN
    });
    if has_main {
        return Ok(());
    }
    let name = std::path::Path::new(workspace_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| workspace_path.to_string());
    workspaces.push(ShellWorkspaceConfig {
        id: "vscode-sidebar-main-workspace".to_string(),
        name: name.to_string(),
        path: workspace_path.to_string(),
        level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(),
        access: SHELL_WORKSPACE_ACCESS_APPROVAL.to_string(),
        built_in: false,
    });
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &workspaces);
    apply_conversation_chat_workspace_changes(
        state,
        conversation_id,
        Some(None),
        Some(normalized_workspaces),
        None,
    )?;
    Ok(())
}

fn ide_chat_conversation_list(state: &AppState, current_viewer_id: &str) -> Result<Value, String> {
    let viewer_id = current_viewer_id.trim();
    let summaries = conversation_service_v2()
        .list_unarchived_conversation_summaries(state)?
        .summaries
        .into_iter()
        .map(|mut item| {
            item.runtime_state = ide_chat_runtime_for_conversation(state, &item.conversation_id)
                .map(|snapshot| snapshot.runtime_state);
            item.state.current_viewer_id = Some(viewer_id.to_string());
            item
        })
        .collect::<Vec<_>>();
    let remote_im_contact_conversations = conversation_service_v2().list_remote_im_contact_conversations(state)?;
    let persona = ide_chat_persona_payload(state, None)?;
    Ok(serde_json::json!({
        "conversations": summaries,
        "unarchivedConversations": summaries,
        "remoteImContactConversations": remote_im_contact_conversations,
        "persona": persona,
        "viewerId": viewer_id,
    }))
}

async fn ide_chat_conversation_changed_since(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ListUnarchivedConversationsChangedSinceInput>(params)?;
    let app_state = state.clone();
    tokio::task::spawn_blocking(move || {
        serde_json::to_value(list_unarchived_conversations_changed_since_blocking(&app_state, &input)?)
            .map_err(|err| format!("Serialize conversation changed-since result failed: {err}"))
    })
    .await
    .map_err(|err| format!("读取未归档会话列表差量任务异常：{err}"))?
}

fn ide_chat_conversation_block_page(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationBlockPageInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let page = if let Some(block_id) = input.block_id {
        conversation_service_v2().get_conversation_block(state, conversation_id, block_id)?
    } else {
        conversation_service_v2().get_conversation_last_block(state, conversation_id)?
    };
    Ok(serde_json::json!({
        "blocks": page.blocks.into_iter().map(|item| {
            serde_json::json!({
                "blockId": item.block_id,
                "messageCount": item.message_count,
                "firstMessageId": item.first_message_id,
                "lastMessageId": item.last_message_id,
                "firstCreatedAt": item.first_created_at,
                "lastCreatedAt": item.last_created_at,
                "isLatest": item.is_latest,
            })
        }).collect::<Vec<_>>(),
        "selectedBlockId": page.selected_block_id,
        "messages": page.messages,
        "hasPrevBlock": page.has_prev_block,
        "hasNextBlock": page.has_next_block,
    }))
}

fn ide_chat_conversation_fast_request_turns(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GetConversationFastRequestTurnsInput>(params)?;
    serde_json::to_value(
        conversation_service_v2()
            .get_conversation_fast_request_turns(state, &input.conversation_id)?,
    )
    .map_err(|err| format!("Serialize fast request turns failed: {err}"))
}

fn ide_chat_conversation_runtime_snapshot(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    serde_json::to_value(read_conversation_runtime_snapshot(state, conversation_id)?)
        .map_err(|err| format!("Serialize conversation runtime snapshot failed: {err}"))
}

async fn ide_chat_conversation_freshness_snapshot(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ForegroundConversationFreshnessInput>(params)?;
    let app_state = state.clone();
    tokio::task::spawn_blocking(move || {
        serde_json::to_value(get_foreground_conversation_freshness_snapshot_blocking(input, &app_state)?)
            .map_err(|err| format!("Serialize conversation freshness snapshot failed: {err}"))
    })
    .await
    .map_err(|err| format!("读取前台 freshness 快照任务异常：{err}"))?
}

fn ide_chat_mark_conversation_read(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<MarkConversationReadInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    serde_json::to_value(
        conversation_service_v2()
            .mark_conversation_read(state, conversation_id)?
            .conversation
            .is_some(),
    )
        .map_err(|err| format!("Serialize mark conversation read result failed: {err}"))
}

fn ide_chat_create_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatCreateConversationInput>(params)?;
    let normalized_shell_workspaces = input
        .shell_workspaces
        .as_ref()
        .map(|workspaces| normalize_conversation_shell_workspaces(state, workspaces))
        .filter(|workspaces| !workspaces.is_empty());
    let fallback_workspace_path = input
        .workspace_path
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .to_string();
    let shell_workspaces = if let Some(workspaces) = normalized_shell_workspaces {
        Some(workspaces)
    } else if !fallback_workspace_path.is_empty() {
        let name = std::path::Path::new(&fallback_workspace_path)
            .file_name()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_else(|| fallback_workspace_path.clone());
        let fallback_workspaces = normalize_conversation_shell_workspaces(
            state,
            &[ShellWorkspaceConfig {
                id: "vscode-sidebar-main-workspace".to_string(),
                name,
                path: fallback_workspace_path.clone(),
                level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(),
                access: SHELL_WORKSPACE_ACCESS_APPROVAL.to_string(),
                built_in: false,
            }],
        );
        (!fallback_workspaces.is_empty()).then_some(fallback_workspaces)
    } else {
        None
    };
    let result = conversation_service_v2().create_conversation(
        state,
        &CreateUnarchivedConversationInput {
            api_config_id: None,
            agent_id: input.agent_id,
            department_id: input.department_id,
            title: input.title,
            copy_source_conversation_id: None,
            shell_workspaces,
            shell_autonomous_mode: input.shell_autonomous_mode,
        },
    )?;
    emit_unarchived_conversation_overview_updated_payload(state, &result.overview_payload);
    let conversation = ide_chat_conversation_open_result(state, &result.conversation_id)?;
    Ok(serde_json::json!({
        "conversationId": result.conversation_id,
        "unarchivedConversations": result.overview_payload.unarchived_conversations,
        "conversation": conversation,
    }))
}

fn ide_chat_delete_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let result = conversation_service_v2().delete_conversation(state, conversation_id)?;
    let _ = delegate_runtime_thread_conversation_delete_by_root(state, conversation_id);
    let overview_payload = conversation_service_v2().refresh_unarchived_conversation_overview(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    Ok(serde_json::json!({
        "deletedConversationId": result.deleted_conversation_id,
        "preferredConversationId": overview_payload.preferred_conversation_id,
        "unarchivedConversations": overview_payload.unarchived_conversations,
    }))
}

async fn ide_chat_batch_archive_conversations(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_params::<BatchArchiveConversationsInput>(params)?;
    let output = batch_archive_conversations_inner(state, input).await?;
    ide_chat_serialize(output)
}

fn ide_chat_rebind_conversation_recipient(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<RebindUnarchivedConversationRecipientInput>(params)?;
    let output = rebind_unarchived_conversation_recipient_inner(input, state)?;
    let overview_payload = conversation_service_v2().refresh_unarchived_conversation_overview(state)?;
    Ok(serde_json::json!({
        "conversationId": output.conversation_id,
        "departmentId": output.department_id,
        "agentId": output.agent_id,
        "preferredApiConfigId": output.preferred_api_config_id,
        "unarchivedConversations": overview_payload.unarchived_conversations,
    }))
}

fn ide_chat_queue_attachment(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatQueueAttachmentInput>(params)?;
    if input.bytes_base64.trim().is_empty() {
        return Err("Attachment payload is empty.".to_string());
    }
    let raw = B64
        .decode(input.bytes_base64.trim())
        .map_err(|err| format!("Decode attachment base64 failed: {err}"))?;
    let queued = queue_attachment_from_raw(
        state,
        input.file_name.trim(),
        input.mime.trim(),
        &raw,
    )?;
    serde_json::to_value(queued).map_err(|err| format!("serialize queued attachment failed: {err}"))
}

async fn ide_chat_send_message(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<SendChatRequest>(params)?;
    let output = submit_chat_message_inner(input, state).await?;
    ide_chat_serialize(output)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatQueueEventInput {
    event_id: String,
}

fn ide_chat_queue_snapshot(state: &AppState) -> Result<Value, String> {
    let snapshot = get_queue_snapshot(state)?;
    serde_json::to_value(snapshot).map_err(|err| format!("serialize queue snapshot failed: {err}"))
}

fn ide_chat_session_state_snapshot(state: &AppState) -> Result<Value, String> {
    let snapshot = get_main_session_state(state)?;
    serde_json::to_value(snapshot).map_err(|err| format!("serialize session state failed: {err}"))
}

fn ide_chat_recall_queue_event(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatQueueEventInput>(params)?;
    let event_id = input.event_id.trim();
    if event_id.is_empty() {
        return Err("eventId is required".to_string());
    }
    ide_chat_serialize(recall_chat_queue_event_inner(event_id, state)?)
}

fn ide_chat_mark_queue_event_guided(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatQueueEventInput>(params)?;
    let event_id = input.event_id.trim();
    if event_id.is_empty() {
        return Err("eventId is required".to_string());
    }
    ide_chat_serialize(mark_chat_queue_event_guided_inner(event_id, state)?)
}

fn ide_chat_stop_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<StopChatRequest>(params)?;
    let stop_result = stop_chat_message_inner(input, state)?;
    let conversation_id = stop_result.conversation_id.clone().unwrap_or_default();
    ide_chat_broadcast_notification(
        "chat.roundFinished",
        serde_json::json!({
            "conversationId": conversation_id,
            "status": "stopped",
            "assistantText": stop_result.assistant_text,
            "assistantMessage": stop_result.assistant_message,
            "archivedBeforeSend": false,
        }),
    );
    Ok(serde_json::json!({
        "conversationId": conversation_id,
        "status": "stopped",
        "aborted": stop_result.aborted,
        "persisted": stop_result.persisted,
        "assistantText": stop_result.assistant_text,
        "assistantMessage": stop_result.assistant_message,
    }))
}

fn ide_chat_session_for_conversation(state: &AppState, conversation_id: &str) -> Result<SessionSelector, String> {
    let conversation_id = conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let agent_id = conversation_meta.agent_id.trim().to_string();
    if agent_id.is_empty() {
        return Err("会话信息不完整".to_string());
    }
    let department_id = conversation_meta.department_id.trim().to_string();
    Ok(SessionSelector {
        api_config_id: None,
        department_id: (!department_id.is_empty()).then_some(department_id),
        agent_id,
        conversation_id: Some(conversation_id.to_string()),
    })
}

async fn ide_chat_rewind_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatRewindInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let message_id = input.message_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if message_id.is_empty() {
        return Err("messageId is required".to_string());
    }

    let started_at = std::time::Instant::now();
    let session = ide_chat_session_for_conversation(state, &conversation_id)?;
    let request = RewindConversationInput {
        session,
        message_id: message_id.clone(),
        undo_apply_patch: input.undo_apply_patch,
    };
    let result = conversation_service_v2().rewind_conversation(
        state,
        &request,
        &message_id,
        &started_at,
    )?;
    if result.removed_count > 0 {
        emit_conversation_todos_updated_payload(
            state,
            &ConversationTodosUpdatedPayload {
                conversation_id: result.conversation_id.clone(),
                current_todo: result.current_todo.clone(),
                current_todos: result.current_todos.clone(),
            },
        );
        ide_chat_emit_overview_updated(state)?;
    }
    let mut recalled_user_message = result.recalled_user_message;
    if let Some(message) = recalled_user_message.as_mut() {
        materialize_message_parts_from_media_refs(&mut message.parts, &state.data_path);
    }
    let conversation = ide_chat_conversation_open_result(state, &conversation_id)?;
    Ok(serde_json::json!({
        "conversationId": conversation_id,
        "removedCount": result.removed_count,
        "remainingCount": result.remaining_count,
        "recalledUserMessage": recalled_user_message,
        "conversation": conversation,
    }))
}

async fn ide_chat_rewind_preview(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatRewindInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let message_id = input.message_id.trim().to_string();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if message_id.is_empty() {
        return Err("messageId is required".to_string());
    }

    let started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[会话撤回] 开始，任务=ide_chat_rewind_preview，conversation_id={}，message_id={}",
        conversation_id,
        message_id
    ));
    let session = ide_chat_session_for_conversation(state, &conversation_id)?;
    let request = RewindConversationInput {
        session,
        message_id: message_id.clone(),
        undo_apply_patch: false,
    };
    let result = conversation_service_v2().preview_rewind_conversation(
        state,
        &request,
        &message_id,
    )?;
    runtime_log_info(format!(
        "[会话撤回] 完成，任务=ide_chat_rewind_preview，conversation_id={}，can_undo_patch={}，duration_ms={}",
        result.conversation_id,
        result.can_undo_patch,
        started_at.elapsed().as_millis()
    ));
    Ok(serde_json::json!({
        "conversationId": result.conversation_id,
        "canUndoPatch": result.can_undo_patch,
        "hint": result.hint,
    }))
}

fn ide_chat_compact_preview(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let session = ide_chat_session_for_conversation(state, &input.conversation_id)?;
    let (selected_api, _resolved_api, source, _effective_agent_id) =
        resolve_archive_target_conversation(state, &session)?;
    let preview = build_trim_compaction_preview_result(state, &selected_api, &source)?;
    Ok(serde_json::to_value(preview).map_err(|err| format!("serialize compact preview failed: {err}"))?)
}

async fn ide_chat_compact_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let session = ide_chat_session_for_conversation(state, &input.conversation_id)?;
    let (selected_api, resolved_api, source, effective_agent_id) =
        resolve_archive_target_conversation(state, &session)?;
    let preview = build_trim_compaction_preview_result(state, &selected_api, &source)?;
    if !preview.can_compact {
        return Err(preview
            .compaction_disabled_reason
            .unwrap_or_else(|| "当前会话暂时不能压缩。".to_string()));
    }
    let result = run_context_compaction_pipeline(
        state,
        &selected_api,
        &resolved_api,
        &source,
        &effective_agent_id,
        "manual_trim_compaction",
        "COMPACTION-FORCE",
        &[],
        false,
    )
    .await?;
    trigger_chat_queue_processing(state);
    let overview_payload = conversation_service_v2().refresh_unarchived_conversation_overview(state)?;
    emit_unarchived_conversation_overview_updated_payload(state, &overview_payload);
    if let Some(compaction_message) = result.compaction_message.clone() {
        ide_chat_broadcast_notification(
            "conversation.messageAppended",
            serde_json::json!({
                "conversationId": source.id,
                "message": compaction_message,
            }),
        );
    }
    Ok(serde_json::to_value(result).map_err(|err| format!("serialize compact result failed: {err}"))?)
}

fn ide_chat_model_list(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation_meta =
        conversation_service_v2().get_conversation_meta(state, input.conversation_id.trim())?;
    let conversation = ide_chat_conversation_from_meta_view(&conversation_meta);
    ide_chat_model_payload_for_conversation(state, &conversation)
}

fn ide_chat_select_model(state: &AppState, _app: &AppHandle, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatSelectModelInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let api_config_id = input.api_config_id.trim();
    runtime_log_info(format!(
        "[会话模型] 开始，任务=切换会话首选模型，入口=vscode_sidebar，会话ID={}，api_config_id={}",
        conversation_id,
        if api_config_id.is_empty() { "部门模型" } else { api_config_id }
    ));
    let preferred_api_config_id = if api_config_id.is_empty() {
        None
    } else {
        let config = state_read_config_cached(state)?;
        let resolved_api_config_id = resolve_model_role_api_config_id(&config, api_config_id)
            .ok_or_else(|| format!("Model role '{api_config_id}' is not configured."))?;
        let selected_api = config
            .api_configs
            .iter()
            .find(|item| item.id.trim() == resolved_api_config_id)
            .ok_or_else(|| format!("API config '{api_config_id}' not found."))?;
        if !is_text_chat_api(selected_api) {
            return Err(format!("API config '{api_config_id}' does not support chat text."));
        }
        Some(resolved_api_config_id)
    };
    let updated_conversation = conversation_service_v2().set_preferred_api_config_id(
        state,
        conversation_id,
        preferred_api_config_id,
    )?;
    runtime_log_info(format!(
        "[会话模型] 完成，任务=切换会话首选模型，入口=vscode_sidebar，会话ID={}，api_config_id={}",
        conversation_id,
        updated_conversation
            .preferred_api_config_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("部门模型")
    ));
    ide_chat_model_payload_for_conversation(state, &updated_conversation)
}

fn ide_chat_resolve_terminal_approval(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatResolveTerminalApprovalInput>(params)?;
    let resolved = resolve_terminal_approval_request(
        state,
        input.request_id.trim(),
        input.approved,
    )?;
    Ok(serde_json::json!({ "resolved": resolved }))
}

fn ide_chat_approve_terminal_approval_for_session(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatTerminalApprovalRequestIdInput>(params)?;
    let approved = approve_terminal_approval_for_session_request(state, input.request_id.trim())?;
    Ok(serde_json::json!({ "approved": approved }))
}

fn ide_chat_approve_terminal_approval_for_workspace(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatTerminalApprovalRequestIdInput>(params)?;
    let approved =
        approve_terminal_approval_for_workspace_request(state, input.request_id.trim())?;
    Ok(serde_json::json!({ "approved": approved }))
}

fn ide_chat_set_conversation_plan_mode(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<SetConversationPlanModeInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId 不能为空".to_string());
    }
    let current_enabled =
        get_conversation_plan_mode_enabled(state, conversation_id).unwrap_or(false);
    if current_enabled != input.plan_mode_enabled {
        set_conversation_plan_mode_enabled(state, conversation_id, input.plan_mode_enabled)?;
        runtime_log_info(format!(
            "[计划模式] 完成，任务=VSCode边栏切换会话运行时计划模式，会话ID={}，状态={}",
            conversation_id,
            if input.plan_mode_enabled { "开启" } else { "关闭" }
        ));
    }
    Ok(serde_json::json!({
        "conversationId": conversation_id,
        "planModeEnabled": input.plan_mode_enabled,
    }))
}

async fn ide_chat_confirm_plan(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ConfirmPlanAndContinueInput>(params)?;
    let continued = confirm_plan_and_continue_inner(state, &input).await?;
    Ok(serde_json::json!({ "continued": continued }))
}

fn ide_chat_tool_review_reports(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewConversationInput>(params)?;
    serde_json::to_value(list_tool_review_reports_internal(input, state)?)
        .map_err(|err| format!("Serialize tool review reports failed: {err}"))
}

fn ide_chat_tool_review_delete_report(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<DeleteToolReviewReportInput>(params)?;
    delete_tool_review_report_internal(input, state)?;
    Ok(serde_json::json!({ "deleted": true }))
}

async fn ide_chat_tool_review_commit_options(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCommitPageInput>(params)?;
    serde_json::to_value(list_tool_review_commit_options_internal_command(input, state).await?)
        .map_err(|err| format!("Serialize tool review commit options failed: {err}"))
}

async fn ide_chat_tool_review_submit_code(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCodeReviewInput>(params)?;
    serde_json::to_value(submit_tool_review_code_internal(input, state).await?)
        .map_err(|err| format!("Serialize tool review submit result failed: {err}"))
}

fn ide_chat_tool_review_batches(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewConversationInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return serde_json::to_value(ListToolReviewBatchesOutput {
            batches: Vec::new(),
            current_batch_key: None,
        })
        .map_err(|err| format!("Serialize tool review batches failed: {err}"));
    }
    let (batches, current_batch_key) = with_tool_review_conversation(state, conversation_id, |conversation| {
        let batches = collect_tool_review_batches_internal(conversation);
        let current_batch_key = conversation
            .messages
            .iter()
            .rev()
            .find(|message| message.role.trim().eq_ignore_ascii_case("user"))
            .map(|message| message.id.clone());
        Ok((batches, current_batch_key))
    })?;
    serde_json::to_value(ListToolReviewBatchesOutput {
        current_batch_key,
        batches: batches
            .iter()
            .map(tool_review_batch_summary_from_collected)
            .collect(),
    })
    .map_err(|err| format!("Serialize tool review batches failed: {err}"))
}

fn ide_chat_tool_review_item_detail(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCallInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    let call_id = input.call_id.trim();
    if conversation_id.is_empty() || call_id.is_empty() {
        return Err("conversationId 和 callId 不能为空。".to_string());
    }
    let detail = with_tool_review_conversation(state, conversation_id, |conversation| {
        let item = tool_review_find_item(conversation, call_id)?;
        Ok(tool_review_item_detail_from_collected(&item))
    })?;
    serde_json::to_value(detail)
        .map_err(|err| format!("Serialize tool review item detail failed: {err}"))
}

async fn ide_chat_tool_review_item_review(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewCallInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    let call_id = input.call_id.trim();
    if conversation_id.is_empty() || call_id.is_empty() {
        return Err("conversationId 和 callId 不能为空。".to_string());
    }
    serde_json::to_value(tool_review_run_for_call_internal(state, conversation_id, call_id).await?)
        .map_err(|err| format!("Serialize tool review item result failed: {err}"))
}

fn ide_chat_tool_review_item_decision(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewSetUserDecisionInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let call_id = input.call_id.trim().to_string();
    if conversation_id.is_empty() || call_id.is_empty() {
        return Err("conversationId 和 callId 不能为空。".to_string());
    }
    let opinion = input.opinion.trim().to_string();
    let user_decision_review = serde_json::json!({
        "kind": "user_decision",
        "allow": input.allow,
        "reviewOpinion": if opinion.is_empty() {
            if input.allow { "用户已批准本次工具执行" } else { "用户已否决本次工具执行" }
        } else {
            opinion.as_str()
        },
        "userOpinion": opinion,
    });
    let detail = conversation_service_v2().update_unarchived_conversation_by_id(
        state,
        &conversation_id,
        |conversation| {
            tool_review_write_call_review(conversation, &call_id, &user_decision_review)?;
            let refreshed = tool_review_find_item(conversation, &call_id)?;
            Ok(tool_review_item_detail_from_collected(&refreshed))
        },
    )?;
    serde_json::to_value(detail)
        .map_err(|err| format!("Serialize tool review decision result failed: {err}"))
}

async fn ide_chat_tool_review_batch_review(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ToolReviewBatchActionInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId 不能为空。".to_string());
    }
    let conversation = with_tool_review_conversation(state, conversation_id, |conversation| {
        Ok(conversation.clone())
    })?;
    let (_batch_number, batch) = tool_review_find_batch_by_index(&conversation, input.batch_index)?;
    let reviewed_call_ids = tool_review_run_missing_reviews_for_batch(state, conversation_id, &batch).await?;
    serde_json::to_value(RunToolReviewBatchOutput {
        batch_key: batch.batch_key,
        reviewed_call_ids,
    })
    .map_err(|err| format!("Serialize tool review batch result failed: {err}"))
}

async fn ide_chat_branch_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<BranchUnarchivedConversationFromSelectionInput>(params)?;
    serde_json::to_value(branch_unarchived_conversation_from_selection_internal(input, state).await?)
        .map_err(|err| format!("Serialize branch conversation result failed: {err}"))
}

async fn ide_chat_branch_conversation_from_message(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_params::<CreateConversationBranchFromMessageInput>(params)?;
    serde_json::to_value(create_conversation_branch_from_message_internal(input, state).await?)
        .map_err(|err| format!("Serialize branch conversation from message result failed: {err}"))
}

async fn ide_chat_submit_delegate(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<SubmitUserAsyncDelegateInput>(params)?;
    serde_json::to_value(submit_user_async_delegate_internal(input, state).await?)
        .map_err(|err| format!("Serialize delegate submit result failed: {err}"))
}

fn ide_chat_task_create(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskCreateInput>(params)?;
    let input = task_create_input_for_write(state, &input)?;
    let task = task_store_create_task(&state.data_path, &input)?;
    task_scheduler_notify_changed(state);
    serde_json::to_value(task)
        .map_err(|err| format!("Serialize task create result failed: {err}"))
}

fn ide_chat_task_update(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskUpdateInput>(params)?;
    let input = task_update_input_for_write(state, &input)?;
    let task = task_store_update_task(&state.data_path, &input)?;
    task_scheduler_notify_changed(state);
    serde_json::to_value(task)
        .map_err(|err| format!("Serialize task update result failed: {err}"))
}

fn ide_chat_task_delete(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskDeleteInput>(params)?;
    task_store_delete_task(&state.data_path, input.task_id.trim())?;
    task_scheduler_notify_changed(state);
    Ok(serde_json::json!(true))
}

fn ide_chat_task_list(state: &AppState) -> Result<Value, String> {
    serde_json::to_value(task_store_list_tasks(&state.data_path)?)
        .map_err(|err| format!("Serialize task list result failed: {err}"))
}

async fn ide_chat_task_optimize_draft(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskOptimizeDraftInput>(params)?;
    serde_json::to_value(task_optimize_draft_internal(input, state).await?)
        .map_err(|err| format!("Serialize task optimize result failed: {err}"))
}

async fn ide_chat_task_dispatch_now(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<TaskDispatchNowInput>(params)?;
    let task = task_store_get_task_record(&state.data_path, input.task_id.trim())?;
    let Some(session) = task_resolve_dispatch_session(state, &task)? else {
        task_fail_missing_bound_conversation(state, &task)?;
        return Ok(serde_json::json!(false));
    };
    task_dispatch_due_task(state, &task, &session).await?;
    Ok(serde_json::json!(true))
}

fn ide_chat_goal_current(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GoalCancelInput>(params)?;
    serde_json::to_value(goal_get_current_inner(state, &input.conversation_id)?)
        .map_err(|err| format!("Serialize goal current result failed: {err}"))
}

fn ide_chat_goal_create(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GoalCreateInput>(params)?;
    serde_json::to_value(goal_create_goal_inner(
        state,
        &input.conversation_id,
        &input.objective,
    )?)
    .map_err(|err| format!("Serialize goal create result failed: {err}"))
}

fn ide_chat_goal_cancel(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GoalCancelInput>(params)?;
    serde_json::to_value(goal_cancel_goal_inner(state, &input.conversation_id)?)
        .map_err(|err| format!("Serialize goal cancel result failed: {err}"))
}
