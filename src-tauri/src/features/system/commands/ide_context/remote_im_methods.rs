pub(crate) async fn remote_im_get_channel_status_inner(
    state: &AppState,
    channel_id: String,
) -> Result<ChannelConnectionStatus, String> {
    let config = state_read_config_cached(state).map_err(|e| format!("{e:?}"))?;
    if let Some(channel) = config
        .remote_im_channels
        .iter()
        .find(|ch| ch.id == channel_id)
    {
        return match channel.platform {
            RemoteImPlatform::OnebotV11 => get_channel_connection_status(channel_id).await,
            RemoteImPlatform::Dingtalk => Ok(dingtalk_stream_manager()
                .get_channel_status(&channel.id)
                .await),
            RemoteImPlatform::Feishu => Ok(ChannelConnectionStatus {
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
            }),
            RemoteImPlatform::WeixinOc => Ok(weixin_oc_manager().build_status(&channel.id).await),
        };
    }
    get_channel_connection_status(channel_id).await
}

pub(crate) async fn ide_chat_remote_im_get_channel_status_for_web_settings(
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
    ide_chat_serialize(remote_im_get_channel_status_inner(state, channel_id).await?)
}

pub(crate) async fn remote_im_restart_channel_inner(
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

    let ctx = remote_im_channel_store_ctx_from_state(state);
    let effective_channel = remote_im_channel_with_effective_credentials(&ctx, &channel)?;
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
        let access: std::sync::Arc<dyn OnebotV11StateAccess> =
            std::sync::Arc::new(AppStateOnebotAccess::new(state));
        manager
            .start_event_consumer(channel_id.clone(), access)
            .await
            .map_err(|err| format!("重启事件消费器失败: {}", err))?;
    } else if channel.enabled && channel.platform == RemoteImPlatform::Dingtalk {
        let manager = dingtalk_stream_manager();
        let channel_clone = channel.clone();
        tokio::spawn(async move {
            if let Err(err) = manager
                .reconcile_channel_runtime(&channel_clone)
                .await
            {
                runtime_log_error(format!(
                    "[远程IM] 钉钉渠道收敛失败: channel_id={}, platform={:?}, error={}",
                    channel_clone.id, channel_clone.platform, err
                ));
            }
        });
    } else if channel.platform == RemoteImPlatform::WeixinOc {
        let access = std::sync::Arc::new(AppStateWeixinOcAccess::new(state));
        weixin_oc_manager()
            .reconcile_channel_runtime(&effective_channel, access)
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

pub(crate) async fn ide_chat_remote_im_restart_channel_for_web_settings(
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

pub(crate) async fn ide_chat_remote_im_get_channel_logs_for_web_settings(
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

pub(crate) async fn ide_chat_remote_im_get_contact_logs_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactLogsInput>(params, "input")?;
    ide_chat_serialize(remote_im_get_contact_logs_inner(state, input).await?)
}

pub(crate) async fn remote_im_get_contact_logs_inner(
    state: &AppState,
    input: RemoteImContactLogsInput,
) -> Result<Vec<ChannelLogEntry>, String> {
    let (channel_id, contact_marker) =
        remote_im_resolve_contact_log_query(state, &input.contact_id)?;
    let logs = get_remote_im_channel_logs(state, channel_id).await?;
    Ok(remote_im_filter_channel_logs_for_contact(logs, &contact_marker))
}

pub(crate) async fn get_remote_im_channel_logs(
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

pub(crate) fn ide_chat_remote_im_list_channels_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(remote_im_list_channels_inner(state)?)
}

pub(crate) fn ide_chat_remote_im_list_contacts_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(remote_im_list_contacts_inner(state)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_allow_send_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactAllowSendUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_allow_send_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactAllowSendFilesUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_allow_send_files_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_blocked_message_prefixes_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactBlockedMessagePrefixesUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_blocked_message_prefixes_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_activation_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactActivationUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_activation_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_department_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactDepartmentBindingUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_department_binding_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_processing_mode_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactProcessingModeUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_processing_mode_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_update_contact_workspace_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactWorkspaceUpdateInput>(params, "input")?;
    ide_chat_serialize(remote_im_update_contact_workspace_inner(state, input)?)
}

pub(crate) fn ide_chat_remote_im_delete_contact_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RemoteImContactDeleteInput>(params, "input")?;
    ide_chat_serialize(remote_im_delete_contact_inner(state, input)?)
}

pub(crate) async fn ide_chat_remote_im_weixin_oc_start_login_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStartInput>(params, "input")?;
    let access = std::sync::Arc::new(AppStateWeixinOcAccess::new(state));
    ide_chat_serialize(weixin_oc_manager().start_login(access, input).await?)
}

pub(crate) async fn ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    let access = std::sync::Arc::new(AppStateWeixinOcAccess::new(state));
    ide_chat_serialize(weixin_oc_manager().poll_login_status(access, input).await?)
}

pub(crate) async fn ide_chat_remote_im_weixin_oc_logout_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    let access = std::sync::Arc::new(AppStateWeixinOcAccess::new(state));
    weixin_oc_manager()
        .logout(access, input.channel_id.as_str())
        .await?;
    ide_chat_serialize(true)
}

pub(crate) async fn ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<WeixinOcLoginStatusInput>(params, "input")?;
    ide_chat_serialize(remote_im_weixin_oc_sync_contacts_inner(state, input)?)
}

/// 启动/恢复全部 enabled 的远程 IM 渠道（Android 原生启动语义）。
///
/// 与桌面端 Vue `afterSafetyGateReady` 调用的 `remoteIm.services.start` 一致：
/// 遍历配置中 `enabled=true` 的渠道，逐个复用 `remote_im_restart_channel_inner`
/// 的收敛逻辑（OneBot reconcile + event consumer / 钉钉 / 微信），保证幂等：
/// - reconcile_channel_runtime 内部先 stop 再按 enabled 决定是否启动，重复调用安全；
/// - OneBot start_event_consumer 用 restart_serialized 防止重复创建消费任务。
/// 返回每个渠道的启动结果汇总，失败渠道不吞错误（记录 last_error + 返回明细）。
pub(crate) async fn start_remote_im_services_inner(
    state: &AppState,
) -> Result<serde_json::Value, String> {
    let config = state_read_config_cached(state).map_err(|e| format!("{e:?}"))?;
    let enabled_channels = config
        .remote_im_channels
        .iter()
        .filter(|ch| ch.enabled)
        .map(|ch| ch.id.clone())
        .collect::<Vec<_>>();
    runtime_log_info(format!(
        "[远程IM] 启动全部已启用渠道: enabled_count={}, channels={:?}",
        enabled_channels.len(),
        enabled_channels
    ));
    if enabled_channels.is_empty() {
        return Ok(serde_json::json!({
            "ok": true,
            "started": 0,
            "failed": 0,
            "channels": [],
        }));
    }

    let mut results = Vec::<serde_json::Value>::new();
    let mut started = 0usize;
    let mut failed = 0usize;
    for channel_id in enabled_channels {
        match remote_im_restart_channel_inner(channel_id.clone(), state).await {
            Ok(status) => {
                started += 1;
                results.push(serde_json::json!({
                    "channelId": channel_id,
                    "ok": true,
                    "connected": status.connected,
                    "statusText": status.status_text,
                }));
            }
            Err(err) => {
                failed += 1;
                runtime_log_error(format!(
                    "[远程IM] 启动渠道失败: channel_id={}, error={}",
                    channel_id, err
                ));
                onebot_v11_ws_manager()
                    .add_log(
                        &channel_id,
                        "error",
                        &format!("[远程IM] 启动渠道失败: {}", err),
                    )
                    .await;
                results.push(serde_json::json!({
                    "channelId": channel_id,
                    "ok": false,
                    "error": err,
                }));
            }
        }
    }
    Ok(serde_json::json!({
        "ok": true,
        "started": started,
        "failed": failed,
        "channels": results,
    }))
}

pub(crate) async fn ide_chat_start_remote_im_services_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(start_remote_im_services_inner(state).await?)
}
