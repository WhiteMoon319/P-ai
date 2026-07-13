async fn ide_chat_remote_im_get_channel_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let channel_id = match params {
        Value::Object(mut map) => map
            .remove("channelId")
            .or_else(|| map.remove("channel_id"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    let config = state_read_config_cached(state).map_err(|e| format!("{e:?}"))?;
    if let Some(channel) = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
    {
        let status = match channel.platform {
            RemoteImPlatform::OnebotV11 => get_channel_connection_status(channel_id).await?,
            RemoteImPlatform::Dingtalk => dingtalk_stream_manager()
                .get_channel_status(&channel.id)
                .await,
            RemoteImPlatform::Feishu => ChannelConnectionStatus {
                channel_id: channel.id.clone(),
                connected: false,
                peer_addr: None,
                connected_at: None,
                listen_addr: String::new(),
                status_text: None,
                last_error: None,
                account_id: None,
                base_url: None,
                login_session_key: None,
                qrcode_url: None,
            },
            RemoteImPlatform::WeixinOc => weixin_oc_manager().build_status(&channel.id).await,
        };
        return ide_chat_serialize(status);
    }
    ide_chat_serialize(get_channel_connection_status(channel_id).await?)
}

async fn remote_im_restart_channel_inner(
    channel_id: String,
    state: &AppState,
) -> Result<ChannelConnectionStatus, String> {
    let channel_id = channel_id.trim().to_string();
    if channel_id.is_empty() {
        return Err("channelId 为必填项。".to_string());
    }
    runtime_log_info(format!("[远程IM] 重启渠道: {}", channel_id));
    onebot_v11_ws_manager()
        .add_log(&channel_id, "info", "[远程IM] 收到渠道重启请求")
        .await;
    let config = state_read_config_cached(state)?;
    let channel = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
        .ok_or_else(|| format!("渠道 {} 未找到", channel_id))?
        .clone();
    onebot_v11_ws_manager()
        .add_log(
            &channel_id,
            "info",
            &format!(
                "[远程IM] 当前渠道配置: enabled={}, platform={:?}",
                channel.enabled, channel.platform
            ),
        )
        .await;

    let effective_channel = remote_im_channel_with_effective_credentials(state, &channel)?;
    let manager = onebot_v11_ws_manager();
    manager
        .reconcile_channel_runtime(&effective_channel)
        .await
        .map_err(|err| format!("重启渠道失败: {}", err))?;
    runtime_log_info(format!(
        "[远程IM] 渠道 {} 已按配置收敛: enabled={}, platform={:?}",
        channel_id, channel.enabled, channel.platform
    ));

    if channel.enabled && channel.platform == RemoteImPlatform::OnebotV11 {
        manager
            .start_event_consumer(channel_id.clone(), state.clone())
            .await
            .map_err(|err| format!("重启事件消费器失败: {}", err))?;
    } else if channel.enabled && channel.platform == RemoteImPlatform::Dingtalk {
        let state_clone = state.clone();
        let manager = dingtalk_stream_manager();
        let channel_clone = remote_im_channel_with_effective_credentials(&state_clone, &channel)?;
        tauri::async_runtime::spawn(async move {
            if let Err(err) = manager
                .reconcile_channel_runtime(&channel_clone, state_clone)
                .await
            {
                runtime_log_error(format!(
                    "[远程IM] 钉钉渠道收敛失败: channel_id={}, platform={:?}, error={}",
                    channel_clone.id, channel_clone.platform, err
                ));
            }
        });
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        weixin_oc_manager()
            .reconcile_channel_runtime(&effective_channel, state.clone())
            .await?;
    }

    if channel.platform == RemoteImPlatform::Dingtalk {
        Ok(dingtalk_stream_manager()
            .get_channel_status(&channel_id)
            .await)
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        Ok(weixin_oc_manager().build_status(&channel_id).await)
    } else {
        Ok(manager.get_connection_status(&channel_id).await)
    }
}

async fn ide_chat_remote_im_restart_channel_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let channel_id = match params {
        Value::Object(mut map) => map
            .remove("channelId")
            .or_else(|| map.remove("channel_id"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(remote_im_restart_channel_inner(channel_id, state).await?)
}

async fn ide_chat_remote_im_get_channel_logs_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let channel_id = match params {
        Value::Object(mut map) => map
            .remove("channelId")
            .or_else(|| map.remove("channel_id"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(get_remote_im_channel_logs(state, channel_id).await?)
}

async fn ide_chat_remote_im_get_contact_logs_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactLogsInput>(params, "input")?;
    let (channel_id, contact_marker) =
        remote_im_resolve_contact_log_query(state, &input.contact_id)?;
    let logs = get_remote_im_channel_logs(state, channel_id).await?;
    ide_chat_serialize(remote_im_filter_channel_logs_for_contact(logs, &contact_marker))
}

async fn get_remote_im_channel_logs(
    state: &AppState,
    channel_id: String,
) -> Result<Vec<ChannelLogEntry>, String> {
    let config = state_read_config_cached(state)?;
    let channel = remote_im_channel_by_id(&config, &channel_id)
        .ok_or_else(|| format!("未找到远程 IM 渠道: {}", channel_id))?;
    match channel.platform {
        RemoteImPlatform::Dingtalk => Ok(dingtalk_stream_manager().get_logs(&channel_id).await),
        RemoteImPlatform::WeixinOc => Ok(weixin_oc_manager().get_logs(&channel_id).await),
        _ => get_channel_logs(channel_id).await,
    }
}

fn ide_chat_remote_im_list_channels_for_web_settings(state: &AppState) -> Result<Value, String> {
    let config = state_read_config_cached(state)?;
    ide_chat_serialize(config.remote_im_channels)
}

fn ide_chat_remote_im_list_contacts_for_web_settings(state: &AppState) -> Result<Value, String> {
    let runtime = state_read_runtime_state_cached(state)?;
    let mut contacts = runtime.remote_im_contacts;
    contacts.sort_by(|a, b| {
        a.channel_id
            .cmp(&b.channel_id)
            .then_with(|| b.last_message_at.cmp(&a.last_message_at))
            .then_with(|| a.id.cmp(&b.id))
    });
    ide_chat_serialize(contacts)
}

fn ide_chat_remote_im_update_contact_allow_send_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactAllowSendUpdateInput>(params, "input")?;
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
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactAllowSendFilesUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.allow_send_files = input.allow_send_files;
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_activation_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactActivationUpdateInput>(params, "input")?;
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
    contact.response_strategy = normalize_contact_response_strategy(&input.response_strategy);
    contact.response_guidance = normalize_contact_response_guidance(&input.response_guidance);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_department_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactDepartmentBindingUpdateInput>(params, "input")?;
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
        "[远程IM] 完成，任务=更新联系人处理部门，contact_id={}，conversation_id={}",
        output.id,
        conversation_id
    ));
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_processing_mode_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactProcessingModeUpdateInput>(params, "input")?;
    let mut runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter_mut()
        .find(|item| item.id == input.contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{}", input.contact_id))?;
    contact.processing_mode = normalize_contact_processing_mode(&input.processing_mode);
    let output = contact.clone();
    state_write_runtime_state_cached(state, &runtime)?;
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_update_contact_workspace_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactWorkspaceUpdateInput>(params, "input")?;
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
    ide_chat_serialize(output)
}

fn ide_chat_remote_im_delete_contact_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactDeleteInput>(params, "input")?;
    let contact_id = input.contact_id.trim();
    if contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let mut runtime = state_read_runtime_state_cached(state)?;
    let before_contacts = runtime.remote_im_contacts.len();
    runtime.remote_im_contacts.retain(|item| item.id != contact_id);
    let removed = runtime.remote_im_contacts.len() != before_contacts;
    if removed {
        state_write_runtime_state_cached(state, &runtime)?;
    }
    ide_chat_serialize(removed)
}

async fn ide_chat_remote_im_weixin_oc_start_login_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStartInput>(params, "input")?;
    ide_chat_serialize(weixin_oc_manager().start_login(state, input).await?)
}

async fn ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    ide_chat_serialize(weixin_oc_manager().poll_login_status(state, input).await?)
}

async fn ide_chat_remote_im_weixin_oc_logout_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    weixin_oc_manager()
        .logout(state, input.channel_id.as_str())
        .await?;
    ide_chat_serialize(true)
}

async fn ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    let config = state_read_config_cached(state)?;
    let channel = remote_im_channel_by_id(&config, &input.channel_id)
        .ok_or_else(|| format!("渠道不存在: {}", input.channel_id))?;
    if channel.platform != RemoteImPlatform::WeixinOc {
        return Err("该渠道不是个人微信渠道".to_string());
    }
    let credentials = remote_im_effective_credentials(state, channel)?;
    let creds = WeixinOcCredentials::from_value(&credentials);
    if creds.account_id.trim().is_empty() || creds.token.trim().is_empty() {
        return ide_chat_serialize(WeixinOcSyncContactsResult {
            channel_id: input.channel_id,
            synced_count: 0,
            message: "当前还没有完成扫码登录，请先登录后再同步联系人。".to_string(),
        });
    }
    let user_id = creds.user_id.trim().to_string();
    let (_, created) = sync_weixin_oc_contact_from_user_id(state, &channel, &user_id)?;
    ide_chat_serialize(WeixinOcSyncContactsResult {
        channel_id: input.channel_id,
        synced_count: 1,
        message: if created {
            format!("已同步个人微信联系人：{}", user_id)
        } else {
            format!("联系人已存在，无需重复同步：{}", user_id)
        },
    })
}
