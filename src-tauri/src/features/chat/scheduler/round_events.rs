fn latest_user_text_from_events(events: &[ChatPendingEvent]) -> String {
    events
        .iter()
        .flat_map(|event| event.messages.iter())
        .rev()
        .find_map(|message| {
            if message.role.trim() != "user" {
                return None;
            }
            message.parts.iter().find_map(|part| match part {
                MessagePart::Text { text, .. } => Some(text.clone()),
                _ => None,
            })
        })
        .unwrap_or_default()
}

fn emit_history_flushed_event(
    state: &AppState,
    payload: &serde_json::Value,
    conversation_id: &str,
    event_ids: &[String],
) {
    ide_chat_broadcast_notification("chat.historyFlushed", payload.clone());
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_error(format!(
            "[聊天调度] history_flushed emit 失败: app_handle unavailable, conversation_id={}, event_ids={:?}",
            conversation_id, event_ids
        ));
        return;
    };
    match app_handle.emit(CHAT_HISTORY_FLUSHED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => {
            runtime_log_error(format!(
                "[聊天调度] history_flushed emit 失败: conversation_id={}, event_ids={:?}, error={}",
                conversation_id, event_ids, err
            ));
        }
    }
}

fn emit_round_started_event(
    state: &AppState,
    conversation_id: &str,
    activation_id: &str,
    request_id: &str,
    assistant_message_id: &str,
    reason: &str,
    department_id: &str,
    agent_id: &str,
    started_at: &str,
    started_at_ms: u64,
) {
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_error(format!(
            "[聊天推送] emit round_started 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        ));
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "activationId": activation_id,
        "requestId": request_id,
        "assistantMessageId": assistant_message_id,
        "reason": reason,
        "departmentId": department_id,
        "agentId": agent_id,
        "startedAt": started_at,
        "startedAtMs": started_at_ms,
    });
    ide_chat_broadcast_notification("chat.roundStarted", payload.clone());
    match app_handle.emit(CHAT_ROUND_STARTED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => runtime_log_error(format!(
            "[聊天推送] emit round_started 失败: conversation_id={}, error={}",
            conversation_id, err
        )),
    }
}

fn emit_round_completed_event(
    state: &AppState,
    conversation_id: &str,
    result: &SendChatResult,
    activation_id: Option<&str>,
    request_id: Option<&str>,
) {
    notify_local_chat_round_completed(state, conversation_id, &result.assistant_text);
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_error(format!(
            "[聊天推送] emit round_completed 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        ));
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "activationId": activation_id.map(str::trim).filter(|value| !value.is_empty()),
        "requestId": request_id.map(str::trim).filter(|value| !value.is_empty()),
        "status": "completed",
        "assistantText": result.assistant_text,
        "archivedBeforeSend": result.archived_before_send,
        "assistantMessage": result
            .assistant_message
            .clone()
            .map(project_message_for_frontend_display_only),
    });
    ide_chat_broadcast_notification("chat.roundFinished", payload.clone());
    match app_handle.emit(CHAT_ROUND_COMPLETED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => runtime_log_error(format!(
            "[聊天推送] emit round_completed 失败: conversation_id={}, error={}",
            conversation_id, err
        )),
    }
}

fn notify_local_chat_round_completed(
    state: &AppState,
    conversation_id: &str,
    assistant_text: &str,
) {
    let conversation_meta = match conversation_service_v2().get_conversation_meta(state, conversation_id) {
        Ok(conversation_meta) => conversation_meta,
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取本地会话完成通知上下文，conversation_id={}，error={}",
                conversation_id, err
            ));
            return;
        }
    };
    if !conversation_meta_is_local_normal_chat_for_notification(&conversation_meta) {
        return;
    }
    if conversation_has_focused_chat_view(state, conversation_id) {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话完成通知，conversation_id={}，reason=chat_view_focused",
            conversation_id
        ));
        return;
    }
    let notification_settings = local_chat_notification_settings(state, conversation_id);
    if !notification_settings.enabled {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话完成通知，conversation_id={}，reason=notification_disabled",
            conversation_id
        ));
        return;
    }
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_warn(format!(
            "[通知] 跳过，任务=发送本地会话完成通知，conversation_id={}，reason=app_handle_unavailable",
            conversation_id
        ));
        return;
    };
    let speaker_name = notification_speaker_name_for_conversation_meta(
        state,
        &conversation_meta,
        notification_settings.ui_language,
    );
    let body = native_notification_text_excerpt(
        assistant_text,
        NATIVE_NOTIFICATION_BODY_MAX_CHARS,
    );
    let final_body = if body.trim().is_empty() {
        local_chat_notification_text(
            notification_settings.ui_language,
            "已完成本轮回复。",
            "已完成本輪回覆。",
            "Finished this reply.",
        )
    } else {
        body
    };
    if let Err(err) = send_native_notification(
        &app_handle,
        &speaker_name,
        &final_body,
        notification_settings.sound_enabled,
    ) {
        runtime_log_warn(format!(
            "[通知] 失败，任务=发送本地会话完成通知，conversation_id={}，error={}",
            conversation_id, err
        ));
    }
}

fn local_chat_notification_text(
    ui_language: &str,
    zh_cn: &str,
    zh_tw: &str,
    en_us: &str,
) -> String {
    match ui_language.trim() {
        "en-US" => en_us.to_string(),
        "zh-TW" => zh_tw.to_string(),
        _ => zh_cn.to_string(),
    }
}

fn conversation_meta_is_local_normal_chat_for_notification(
    conversation_meta: &ConversationMetaView,
) -> bool {
    conversation_meta.conversation_kind.trim() == CONVERSATION_KIND_CHAT
        && conversation_meta.conversation_kind.trim() != CONVERSATION_KIND_SYSTEM_NOTIFICATION
        && conversation_meta.conversation_kind.trim() != CONVERSATION_KIND_DELEGATE
        && conversation_meta.conversation_kind.trim() != CONVERSATION_KIND_REMOTE_IM_CONTACT
}

fn notification_speaker_name_for_conversation_meta(
    state: &AppState,
    conversation_meta: &ConversationMetaView,
    ui_language: &str,
) -> String {
    let agent_id = conversation_meta.agent_id.trim();
    if agent_id.is_empty() {
        return local_chat_notification_text(
            ui_language,
            "当前人格",
            "當前人格",
            "Current persona",
        );
    }
    match state_read_agents_cached(state) {
        Ok(agents) => agents
            .iter()
            .find(|agent| agent.id.trim() == agent_id)
            .map(|agent| agent.name.trim())
            .filter(|name| !name.is_empty())
            .unwrap_or(agent_id)
            .to_string(),
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取人格名称失败后回退ID，conversation_id={}，agent_id={}，error={}",
                conversation_meta.id, agent_id, err
            ));
            agent_id.to_string()
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct LocalChatNotificationSettings {
    enabled: bool,
    sound_enabled: bool,
    ui_language: &'static str,
}

impl Default for LocalChatNotificationSettings {
    fn default() -> Self {
        Self {
            enabled: default_message_notification_enabled(),
            sound_enabled: default_message_notification_sound_enabled(),
            ui_language: "zh-CN",
        }
    }
}

fn local_chat_notification_settings(
    state: &AppState,
    conversation_id: &str,
) -> LocalChatNotificationSettings {
    match state_read_config_cached(state) {
        Ok(config) => LocalChatNotificationSettings {
            enabled: config.message_notification_enabled,
            sound_enabled: config.message_notification_sound_enabled,
            ui_language: match config.ui_language.trim() {
                "en-US" => "en-US",
                "zh-TW" => "zh-TW",
                _ => "zh-CN",
            },
        },
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取通知设置失败后回退默认值，conversation_id={}，error={}",
                conversation_id, err
            ));
            LocalChatNotificationSettings::default()
        }
    }
}

fn emit_round_failed_event(
    state: &AppState,
    conversation_id: &str,
    error_text: &str,
    activation_id: Option<&str>,
    request_id: Option<&str>,
) {
    notify_local_chat_round_failed(state, conversation_id, error_text);
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_error(format!(
            "[聊天推送] emit round_failed 失败: app_handle unavailable, conversation_id={}",
            conversation_id
        ));
        return;
    };
    let payload = serde_json::json!({
        "conversationId": conversation_id,
        "activationId": activation_id.map(str::trim).filter(|value| !value.is_empty()),
        "requestId": request_id.map(str::trim).filter(|value| !value.is_empty()),
        "status": "failed",
        "error": error_text,
    });
    ide_chat_broadcast_notification("chat.roundFinished", payload.clone());
    match app_handle.emit(CHAT_ROUND_FAILED_EVENT, payload) {
        Ok(_) => {}
        Err(err) => runtime_log_error(format!(
            "[聊天推送] emit round_failed 失败: conversation_id={}, error={}",
            conversation_id, err
        )),
    }
}

fn notify_local_chat_round_failed(state: &AppState, conversation_id: &str, error_text: &str) {
    let conversation_meta = match conversation_service_v2().get_conversation_meta(state, conversation_id) {
        Ok(conversation_meta) => conversation_meta,
        Err(err) => {
            runtime_log_warn(format!(
                "[通知] 跳过，任务=读取本地会话失败通知上下文，conversation_id={}，error={}",
                conversation_id, err
            ));
            return;
        }
    };
    if !conversation_meta_is_local_normal_chat_for_notification(&conversation_meta) {
        return;
    }
    if conversation_has_focused_chat_view(state, conversation_id) {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话失败通知，conversation_id={}，reason=chat_view_focused",
            conversation_id
        ));
        return;
    }
    let notification_settings = local_chat_notification_settings(state, conversation_id);
    if !notification_settings.enabled {
        runtime_log_debug(format!(
            "[通知] 跳过，任务=本地会话失败通知，conversation_id={}，reason=notification_disabled",
            conversation_id
        ));
        return;
    }
    let app_handle = match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    };
    let Some(app_handle) = app_handle else {
        runtime_log_warn(format!(
            "[通知] 跳过，任务=发送本地会话失败通知，conversation_id={}，reason=app_handle_unavailable",
            conversation_id
        ));
        return;
    };
    let speaker_name = notification_speaker_name_for_conversation_meta(
        state,
        &conversation_meta,
        notification_settings.ui_language,
    );
    let body = native_notification_text_excerpt(
        error_text,
        NATIVE_NOTIFICATION_BODY_MAX_CHARS,
    );
    let final_body = if body.trim().is_empty() {
        local_chat_notification_text(
            notification_settings.ui_language,
            "本轮调度失败。",
            "本輪調度失敗。",
            "This round failed.",
        )
    } else {
        body
    };
    let title = match notification_settings.ui_language {
        "en-US" => format!("{speaker_name} response failed"),
        "zh-TW" => format!("{speaker_name} 調度失敗"),
        _ => format!("{speaker_name} 调度失败"),
    };
    if let Err(err) = send_native_notification(
        &app_handle,
        &title,
        &final_body,
        notification_settings.sound_enabled,
    ) {
        runtime_log_warn(format!(
            "[通知] 失败，任务=发送本地会话失败通知，conversation_id={}，error={}",
            conversation_id, err
        ));
    }
}

