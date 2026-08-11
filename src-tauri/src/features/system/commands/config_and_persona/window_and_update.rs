








#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateRecordHotkeyInput {
    pub(crate) record_hotkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateRecordBackgroundWakeInput {
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecordHotkeyUpdateResult {
    pub(crate) record_hotkey: String,
    pub(crate) record_background_wake_enabled: bool,
    pub(crate) min_record_seconds: u32,
    pub(crate) max_record_seconds: u32,
}

// 聊天视图前台激活状态：由前端 useChatForegroundActivity 上报
// （visibility + focus + viewMode==="chat"），所有平台统一存储，
// Android 无窗口焦点 API，通知等逻辑依赖该状态判断"会话是否在前台可见"。
pub(crate) static CHAT_VIEW_FOREGROUND_ACTIVE: std::sync::OnceLock<std::sync::atomic::AtomicBool> =
    std::sync::OnceLock::new();

#[cfg(target_os = "android")]
pub(crate) fn chat_view_foreground_active() -> bool {
    CHAT_VIEW_FOREGROUND_ACTIVE
        .get()
        .map(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false)
}



pub(crate) fn parse_version_parts(input: &str) -> Vec<u64> {
    let cleaned = input
        .trim()
        .trim_start_matches(['v', 'V'])
        .split(['-', '+'])
        .next()
        .unwrap_or_default();
    cleaned
        .split('.')
        .map(|part| part.trim().parse::<u64>().unwrap_or(0))
        .collect()
}

pub(crate) fn validate_department_names_unique(config: &AppConfig) -> Result<(), String> {
    let mut seen = std::collections::HashSet::<String>::new();
    for department in &config.departments {
        let name = department.name.trim();
        if name.is_empty() {
            return Err("部门名称不能为空".to_string());
        }
        let key = name.to_ascii_lowercase();
        if !seen.insert(key) {
            return Err(format!("部门名称不能重复：{name}"));
        }
    }
    Ok(())
}

pub(crate) fn changed_department_ids(old_config: &AppConfig, new_config: &AppConfig) -> Vec<String> {
    let old_by_id = old_config
        .departments
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let new_by_id = new_config
        .departments
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    old_by_id
        .keys()
        .chain(new_by_id.keys())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|id| old_by_id.get(id) != new_by_id.get(id))
        .collect::<Vec<_>>()
}

pub(crate) fn changed_department_tree_ids(old_config: &AppConfig, new_config: &AppConfig) -> Vec<String> {
    let old_children = old_config
        .departments
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                normalize_department_child_ids(&item.child_department_ids, &item.id),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    let new_children = new_config
        .departments
        .iter()
        .map(|item| {
            (
                item.id.clone(),
                normalize_department_child_ids(&item.child_department_ids, &item.id),
            )
        })
        .collect::<std::collections::HashMap<_, _>>();
    old_children
        .keys()
        .chain(new_children.keys())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|id| old_children.get(id) != new_children.get(id))
        .collect::<Vec<_>>()
}

pub(crate) fn changed_department_content_ids(old_config: &AppConfig, new_config: &AppConfig) -> Vec<String> {
    let strip_tree = |department: &DepartmentConfig| {
        let mut cloned = department.clone();
        cloned.child_department_ids = Vec::new();
        cloned
    };
    let old_by_id = old_config
        .departments
        .iter()
        .map(|item| (item.id.clone(), strip_tree(item)))
        .collect::<std::collections::HashMap<_, _>>();
    let new_by_id = new_config
        .departments
        .iter()
        .map(|item| (item.id.clone(), strip_tree(item)))
        .collect::<std::collections::HashMap<_, _>>();
    old_by_id
        .keys()
        .chain(new_by_id.keys())
        .cloned()
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .filter(|id| old_by_id.get(id) != new_by_id.get(id))
        .collect::<Vec<_>>()
}

pub(crate) fn config_provider_domain_changed(old_config: &AppConfig, new_config: &AppConfig) -> bool {
    let old_providers = serde_json::to_string(&old_config.api_providers).unwrap_or_default();
    let new_providers = serde_json::to_string(&new_config.api_providers).unwrap_or_default();
    let old_api_configs = serde_json::to_string(&old_config.api_configs).unwrap_or_default();
    let new_api_configs = serde_json::to_string(&new_config.api_configs).unwrap_or_default();
    old_providers != new_providers
        || old_api_configs != new_api_configs
        || old_config.assistant_department_api_config_id
            != new_config.assistant_department_api_config_id
        || old_config.tool_review_api_config_id != new_config.tool_review_api_config_id
        || old_config.selected_api_config_id != new_config.selected_api_config_id
}

pub(crate) fn ide_chat_broadcast_simple_notification(method: &str) {
    ide_chat_broadcast_notification(method, serde_json::json!({}));
}

pub(crate) fn broadcast_sidebar_persona_changed() {
    ide_chat_broadcast_simple_notification("persona.changed");
}

pub(crate) fn broadcast_sidebar_department_changed() {
    ide_chat_broadcast_simple_notification("department.changed");
}

pub(crate) fn broadcast_sidebar_department_tree_changed() {
    ide_chat_broadcast_simple_notification("departmentTree.changed");
}

pub(crate) fn broadcast_sidebar_provider_changed() {
    ide_chat_broadcast_simple_notification("provider.changed");
}

pub(crate) fn split_main_config_departments(departments: &[DepartmentConfig]) -> Vec<DepartmentConfig> {
    departments
        .iter()
        .filter(|item| !is_private_workspace_source(&item.source))
        .cloned()
        .collect::<Vec<_>>()
}

pub(crate) fn persist_departments_by_source(
    state: &AppState,
    runtime_config: &AppConfig,
) -> Result<AppConfig, String> {
    sync_private_departments_to_workspace(
        &state.data_path,
        runtime_config,
        &runtime_config.departments,
    )?;
    let mut main_config = runtime_config.clone();
    main_config.departments = split_main_config_departments(&runtime_config.departments);
    state_write_config_cached(state, &main_config)?;
    Ok(main_config)
}

pub(crate) fn runtime_config_with_private_organization(
    state: &AppState,
    config: &AppConfig,
    data: &AppData,
) -> Result<AppConfig, String> {
    build_runtime_organization_snapshot_from_parts(&state.data_path, config, &data.agents)
        .map(|snapshot| snapshot.config)
}

pub(crate) fn runtime_agents_with_private_organization(
    state: &AppState,
    config: &AppConfig,
    data: &AppData,
) -> Result<Vec<AgentProfile>, String> {
    build_runtime_organization_snapshot_from_parts(&state.data_path, config, &data.agents)
        .map(|snapshot| snapshot.agents)
}

pub(crate) fn private_agent_operation_error(agent_id: &str) -> String {
    format!("当前人格来自私有工作区，不能直接在主配置中修改：{agent_id}")
}

pub(crate) fn is_newer_version(current: &str, latest: &str) -> bool {
    let a = parse_version_parts(current);
    let b = parse_version_parts(latest);
    let max_len = a.len().max(b.len());
    for idx in 0..max_len {
        let av = *a.get(idx).unwrap_or(&0);
        let bv = *b.get(idx).unwrap_or(&0);
        if bv > av {
            return true;
        }
        if bv < av {
            return false;
        }
    }
    false
}

pub(crate) const GITHUB_REPO_PAGE: &str = "https://github.com/kawayiYokami/P-ai";

pub(crate) fn set_preferred_release_source(state: &AppState, source: &str) {
    match state.preferred_release_source.lock() {
        Ok(mut slot) => {
            *slot = source.to_string();
        }
        Err(err) => {
            runtime_log_error(format!(
                "set_preferred_release_source 锁定 preferred_release_source 失败：source={}, err={}",
                source,
                err
            ));
        }
    }
}

pub(crate) async fn probe_release_source_once(state: &AppState) {
    set_preferred_release_source(state, "github");
}



pub(crate) fn set_github_update_method_inner(
    update_method: String,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<AppConfig, String> {
    let normalized = normalize_github_update_method(&update_method);
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    if config.github_update_method != normalized {
        config.github_update_method = normalized.clone();
        state_write_config_cached(state, &config)?;
        runtime_log_info(format!("[自动更新] 更新方式偏好已保存：method={normalized}"));
    }
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(runtime_config)
}



pub(crate) fn normalize_ui_language(value: &str) -> String {
    match value.trim() {
        "en-US" => "en-US".to_string(),
        "zh-TW" => "zh-TW".to_string(),
        _ => "zh-CN".to_string(),
    }
}


pub(crate) fn set_ui_language_inner(
    ui_language: String,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<AppConfig, String> {
    let normalized = normalize_ui_language(&ui_language);
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    if config.ui_language != normalized {
        config.ui_language = normalized.clone();
        state_write_config_cached(state, &config)?;
        runtime_log_info(format!("[配置] 界面语言已保存：ui_language={normalized}"));
    }
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(runtime_config)
}



pub(crate) fn get_department_default_draft_inner(
    state: &AppState,
    department_id: &str,
) -> Result<DepartmentConfig, String> {
    let config = load_config_inner(state)?;
    default_department_draft(department_id, &config.ui_language)
}

pub(crate) fn load_config_inner(state: &AppState) -> Result<AppConfig, String> {
    let mut result = state_read_config_cached(&state)?;
    let config_changed_by_normalize = normalize_app_config_and_detect_changes(&mut result);
    let workspace_changed = ensure_default_shell_workspace_in_config(&mut result, &state);
    let ctx = remote_im_channel_store_ctx_from_state(state);
    let remote_im_private_state_migrated =
        remote_im_migrate_channel_private_states(&ctx, &mut result)?;
    if config_changed_by_normalize || workspace_changed || remote_im_private_state_migrated {
        state_write_config_cached(&state, &result)?;
    }
    let _ = run_app_data_migrations_with_state(&state, &result)?;
    // 无可用 LLM 时强制进入简单设置模式，方便首次启动用户直接配置供应商。
    if !has_usable_text_llm(&result) {
        result.simple_setup_mode = true;
    }
    let runtime_agents = state_read_agents_cached(&state)?;
    let snapshot =
        build_runtime_organization_snapshot_from_parts(&state.data_path, &result, &runtime_agents)?;
    Ok(snapshot.config)
}

pub(crate) fn read_app_bootstrap_snapshot(state: &AppState) -> Result<AppBootstrapSnapshot, String> {
    let mut config = state_read_config_cached(state)?;
    let config_changed_by_normalize = normalize_app_config_and_detect_changes(&mut config);
    let workspace_changed = ensure_default_shell_workspace_in_config(&mut config, state);
    let ctx = remote_im_channel_store_ctx_from_state(state);
    let remote_im_private_state_migrated =
        remote_im_migrate_channel_private_states(&ctx, &mut config)?;
    if config_changed_by_normalize || workspace_changed || remote_im_private_state_migrated {
        state_write_config_cached(state, &config)?;
    }
    let _ = run_app_data_migrations_with_state(state, &config)?;
    // 无可用 LLM 时强制进入简单设置模式，方便首次启动用户直接配置供应商。
    if !has_usable_text_llm(&config) {
        config.simple_setup_mode = true;
    }
    // 启动快照阶段修复会话总索引，避免旧版本误删归档入口后仍需人工恢复。
    let _ = state_read_chat_index_cached(state)?;
    let mut data = state_read_agents_runtime_snapshot(state)?;
    let assistant_agent_id =
        assistant_department_agent_id(&config).unwrap_or_else(default_assistant_department_agent_id);
    let runtime_changed = if data.assistant_department_agent_id != assistant_agent_id {
        data.assistant_department_agent_id = assistant_agent_id;
        true
    } else {
        false
    };
    if runtime_changed {
        state_write_runtime_state_cached(state, &build_runtime_state_file(&data))?;
    }
    let runtime_snapshot =
        build_runtime_organization_snapshot_from_parts(&state.data_path, &config, &data.agents)?;
    let mut runtime_data = data.clone();
    runtime_data.agents = runtime_snapshot.agents.clone();
    let chat_settings = ChatSettings {
        assistant_department_agent_id: data.assistant_department_agent_id.clone(),
        user_alias: user_persona_name(&runtime_data),
        response_style_id: data.response_style_id.clone(),
        pdf_read_mode: data.pdf_read_mode.clone(),
        background_voice_screenshot_keywords: data.background_voice_screenshot_keywords.clone(),
        background_voice_screenshot_mode: data.background_voice_screenshot_mode.clone(),
        instruction_presets: data.instruction_presets.clone(),
    };
    Ok(AppBootstrapSnapshot {
        config: runtime_snapshot.config,
        agents: runtime_data.agents,
        chat_settings,
    })
}










pub(crate) fn removed_remote_im_channels(
    previous: &AppConfig,
    next: &AppConfig,
) -> Vec<RemoteImChannelConfig> {
    let next_ids = next
        .remote_im_channels
        .iter()
        .map(|channel| channel.id.trim().to_ascii_lowercase())
        .collect::<std::collections::HashSet<_>>();
    previous
        .remote_im_channels
        .iter()
        .filter(|channel| !next_ids.contains(&channel.id.trim().to_ascii_lowercase()))
        .cloned()
        .collect()
}

pub(crate) fn stop_removed_remote_im_channel_runtimes(
    state: AppState,
    channels: Vec<RemoteImChannelConfig>,
) {
    if channels.is_empty() {
        return;
    }
    tokio::spawn(async move {
        for channel in channels {
            let channel_id = channel.id.trim().to_string();
            if channel_id.is_empty() {
                continue;
            }
            match channel.platform {
                RemoteImPlatform::OnebotV11 => {
                    if let Err(err) = onebot_v11_ws_manager().stop_channel(&channel_id).await {
                        runtime_log_error(format!(
                            "[远程IM] 删除渠道后停止 OneBot v11 运行态失败: channel_id={}, error={}",
                            channel_id, err
                        ));
                    }
                }
                RemoteImPlatform::Dingtalk => {
                    dingtalk_stream_manager().stop_channel(&channel_id).await;
                }
                RemoteImPlatform::WeixinOc => {
                    weixin_oc_manager().stop_channel(&channel_id).await;
                }
                RemoteImPlatform::Feishu => {}
            }
            if let Err(err) = {
                let ctx = remote_im_channel_store_ctx_from_state(&state);
                remote_im_delete_channel_private_state(
                    &ctx,
                    &channel.platform,
                    &channel_id,
                )
            } {
                runtime_log_error(format!(
                    "[远程IM] 删除渠道后清理私有状态失败: channel_id={}, platform={:?}, error={}",
                    channel_id, channel.platform, err
                ));
            }
        }
    });
}

/** 配置局部更新输入：所有字段可空，只更新传入的非空字段（读旧配置 merge，避免覆盖丢失）。 */
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PatchAppConfigInput {
    #[serde(default)]
    pub(crate) web_access_enabled: Option<bool>,
    #[serde(default)]
    pub(crate) web_access_port: Option<u16>,
    #[serde(default)]
    pub(crate) web_access_password: Option<String>,
    #[serde(default)]
    pub(crate) ui_language: Option<String>,
    #[serde(default)]
    pub(crate) ui_size_scale: Option<u16>,
    #[serde(default)]
    pub(crate) message_notification_enabled: Option<bool>,
    #[serde(default)]
    pub(crate) message_notification_sound_enabled: Option<bool>,
    #[serde(default)]
    pub(crate) desktop_operation_notice_enabled: Option<bool>,
    #[serde(default)]
    pub(crate) stt_api_config_id: Option<String>,
    #[serde(default)]
    pub(crate) stt_auto_send: Option<bool>,
    #[serde(default)]
    pub(crate) api_configs: Option<Vec<ApiConfig>>,
}

/** 局部更新配置：读当前配置，仅覆盖传入字段，其余保留（修复 saveConfig 全量覆盖丢字段）。 */
pub(crate) fn patch_config_inner(
    input: PatchAppConfigInput,
    app: &NativeAppHandle,
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<AppConfig, String> {
    let mut config = state_read_config_cached(state)?;
    if let Some(v) = input.web_access_enabled { config.web_access_enabled = v; }
    if let Some(v) = input.web_access_port { config.web_access_port = normalize_web_access_port(v); }
    if let Some(v) = input.web_access_password { config.web_access_password = normalize_web_access_password(&v); }
    if let Some(v) = input.ui_language { config.ui_language = v; }
    if let Some(v) = input.ui_size_scale { config.ui_size_scale = v; }
    if let Some(v) = input.message_notification_enabled { config.message_notification_enabled = v; }
    if let Some(v) = input.message_notification_sound_enabled { config.message_notification_sound_enabled = v; }
    if let Some(v) = input.desktop_operation_notice_enabled { config.desktop_operation_notice_enabled = v; }
    if let Some(v) = input.stt_api_config_id { config.stt_api_config_id = Some(v); }
    if let Some(v) = input.stt_auto_send { config.stt_auto_send = v; }
    if let Some(v) = input.api_configs { config.api_configs = v; }
    // 复用 save_config_inner 的完整保存流程（引用清理/广播/服务联动）
    save_config_inner(config, app, state, ide_context_runtime)
}

pub(crate) fn save_config_inner(
    config: AppConfig,
    app: &NativeAppHandle,
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<AppConfig, String> {
    if config.api_configs.is_empty() {
        return Err("至少需要配置一个 API 配置。".to_string());
    }
    let mut config = config;
    normalize_app_config(&mut config);
    let ctx = remote_im_channel_store_ctx_from_state(state);
    remote_im_migrate_channel_private_states(&ctx, &mut config)?;
    let _ = ensure_default_shell_workspace_in_config(&mut config, &state);
    #[cfg(not(target_os = "android"))]
    set_record_hotkey_probe_background_wake_enabled(config.record_background_wake_enabled);

    let mut data = state_read_agents_runtime_snapshot(&state)?;
    let base_config = state_read_config_cached(&state)?;
    let removed_remote_im_channels = removed_remote_im_channels(&base_config, &config);
    let previous_runtime_config = runtime_config_with_private_organization(&state, &base_config, &data)?;
    let departments_changed = changed_department_ids(&previous_runtime_config, &config);
    let department_content_changed = !changed_department_content_ids(&previous_runtime_config, &config).is_empty();
    let department_tree_changed = !changed_department_tree_ids(&previous_runtime_config, &config).is_empty();
    let provider_changed = config_provider_domain_changed(&base_config, &config);
    let shell_workspaces_changed = base_config.shell_workspaces != config.shell_workspaces;
    validate_department_names_unique(&config)?;
    let main_config = persist_departments_by_source(&state, &config)?;
    if !departments_changed.is_empty() {
        mark_prompt_cache_rebuild_for_system_sources_by_departments(
            &state,
            &departments_changed,
        );
    }
    if shell_workspaces_changed {
        mark_prompt_cache_rebuild_for_all_system_environments(&state);
    }
    let assistant_workspace_label_synced = if shell_workspaces_changed {
        sync_assistant_workspace_label_for_unarchived_conversations(
            &state,
            &base_config,
            &config,
        )?
    } else {
        0
    };
    if let Some(agent_id) = assistant_department_agent_id(&config) {
        if data.assistant_department_agent_id != agent_id {
            data.assistant_department_agent_id = agent_id;
            state_write_runtime_state_cached(&state, &build_runtime_state_file(&data))
                .map_err(|err| format!("配置已保存，但运行状态保存失败：{err}"))?;
        }
    }
    if base_config.hotkey != main_config.hotkey {
        // Android 无窗口热键，跳过热键注册
    }
    let runtime_config = runtime_config_with_private_organization(&state, &main_config, &data)
        .map_err(|err| format!("配置已保存，但运行时配置刷新失败：{err}"))?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    if assistant_workspace_label_synced > 0 {
        emit_unarchived_conversation_overview_updated_from_state(&state)?;
    }
    if department_content_changed {
        broadcast_sidebar_department_changed();
    }
    if department_tree_changed {
        broadcast_sidebar_department_tree_changed();
    }
    if provider_changed {
        broadcast_sidebar_provider_changed();
    }
    stop_removed_remote_im_channel_runtimes(state.clone(), removed_remote_im_channels);
    // Web 访问（远程连接）配置变更联动：关闭时停服务，启用/改端口或密码时重启。
    if !main_config.web_access_enabled && IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst) {
        runtime_log_info(format!(
            "[网络访问] 配置已关闭，停止 Web 访问服务: port={}",
            base_config.web_access_port
        ));
        tokio::spawn(async move {
            shutdown_web_access_server().await;
        });
    } else if main_config.web_access_enabled
        && (!base_config.web_access_enabled
            || ((base_config.web_access_port != main_config.web_access_port
                || base_config.web_access_password != main_config.web_access_password)
                && IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst)))
    {
        runtime_log_info(format!(
            "[网络访问] 配置已启用、端口或密码已变更，重启 Web 访问服务: old_enabled={}, new_enabled={}, old_port={}, new_port={}",
            base_config.web_access_enabled,
            main_config.web_access_enabled,
            base_config.web_access_port,
            main_config.web_access_port
        ));
        let app = app.clone();
        let state = state.clone();
        let ide_context_runtime = ide_context_runtime.clone();
        tokio::spawn(async move {
            restart_web_access_server(app, state, ide_context_runtime).await;
        });
    }
    Ok(runtime_config)
}

pub(crate) fn set_skipped_github_update_version_inner(
    version: String,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<AppConfig, String> {
    let normalized = normalize_skipped_github_update_version(&version);
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    if config.skipped_github_update_version != normalized {
        config.skipped_github_update_version = normalized.clone();
        state_write_config_cached(state, &config)?;
        runtime_log_warn(format!("[自动更新] 已保存跳过版本：version={normalized}"));
    }
    #[cfg(not(target_os = "android"))]
    if let Some(inner) = &app.inner {
        sync_update_state_from_skip_version(inner, &normalized);
    }
    #[cfg(target_os = "android")]
    sync_update_state_from_skip_version(app, &normalized);
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(runtime_config)
}
