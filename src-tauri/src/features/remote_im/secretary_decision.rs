#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RemoteImSecretaryDecisionReply {
    #[serde(default, alias = "should_reply")]
    pub(crate) should_reply: bool,
    #[serde(default, alias = "target_delegate_id")]
    pub(crate) target_delegate_id: Option<String>,
    #[serde(default)]
    pub(crate) reason: String,
}

#[derive(Debug, Clone)]
pub(crate) struct RemoteImSecretaryDecision {
    pub(crate) should_reply: bool,
    pub(crate) target_delegate_id: Option<String>,
    pub(crate) reason: String,
    pub(crate) model_name: String,
    pub(crate) emit_log: bool,
}

pub(crate) fn remote_im_secretary_current_assistant_context(
    state: &AppState,
    conversation_id: &str,
) -> Result<RemoteImConversationAssistantContext, String> {
    get_conversation_remote_im_assistant_context(state, conversation_id)?
        .ok_or_else(|| format!("缺少当前助理上下文: conversation_id={}", conversation_id.trim()))
}

pub(crate) fn remote_im_resolve_contact_assistant_context(
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

pub(crate) fn remote_im_resolve_secretary_contact(
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

pub(crate) async fn run_remote_im_secretary_decision(
    state: &AppState,
    contact: &RemoteImContact,
    current_assistant: &RemoteImConversationAssistantContext,
    history_messages: &[RemoteImSecretaryMessageDigest],
    new_batch_messages: &[RemoteImSecretaryMessageDigest],
    work_ledger: &str,
    active_delegate_ids: &[String],
) -> Result<RemoteImSecretaryDecision, String> {
    if effective_remote_im_contact_response_strategy(contact) == "always_reply" {
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
        &effective_remote_im_channel_response_guidance(state, contact),
        current_assistant,
        history_messages,
        new_batch_messages,
        work_ledger,
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
            timeout_secs: Some(60),
            json_only: true,
        },
        Some(state),
        Vec::new(),
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
