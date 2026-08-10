








#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRecordHotkeyInput {
    record_hotkey: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateRecordBackgroundWakeInput {
    enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordHotkeyUpdateResult {
    record_hotkey: String,
    record_background_wake_enabled: bool,
    min_record_seconds: u32,
    max_record_seconds: u32,
}

// 聊天视图前台激活状态：由前端 useChatForegroundActivity 上报
// （visibility + focus + viewMode==="chat"），所有平台统一存储，
// Android 无窗口焦点 API，通知等逻辑依赖该状态判断"会话是否在前台可见"。
static CHAT_VIEW_FOREGROUND_ACTIVE: std::sync::OnceLock<std::sync::atomic::AtomicBool> =
    std::sync::OnceLock::new();

#[cfg(target_os = "android")]
fn chat_view_foreground_active() -> bool {
    CHAT_VIEW_FOREGROUND_ACTIVE
        .get()
        .map(|flag| flag.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(false)
}



fn parse_version_parts(input: &str) -> Vec<u64> {
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

fn validate_department_names_unique(config: &AppConfig) -> Result<(), String> {
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

fn changed_department_ids(old_config: &AppConfig, new_config: &AppConfig) -> Vec<String> {
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

fn changed_department_tree_ids(old_config: &AppConfig, new_config: &AppConfig) -> Vec<String> {
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

fn changed_department_content_ids(old_config: &AppConfig, new_config: &AppConfig) -> Vec<String> {
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

fn config_provider_domain_changed(old_config: &AppConfig, new_config: &AppConfig) -> bool {
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

fn ide_chat_broadcast_simple_notification(method: &str) {
    ide_chat_broadcast_notification(method, serde_json::json!({}));
}

fn broadcast_sidebar_persona_changed() {
    ide_chat_broadcast_simple_notification("persona.changed");
}

fn broadcast_sidebar_department_changed() {
    ide_chat_broadcast_simple_notification("department.changed");
}

fn broadcast_sidebar_department_tree_changed() {
    ide_chat_broadcast_simple_notification("departmentTree.changed");
}

fn broadcast_sidebar_provider_changed() {
    ide_chat_broadcast_simple_notification("provider.changed");
}

fn split_main_config_departments(departments: &[DepartmentConfig]) -> Vec<DepartmentConfig> {
    departments
        .iter()
        .filter(|item| !is_private_workspace_source(&item.source))
        .cloned()
        .collect::<Vec<_>>()
}

fn persist_departments_by_source(
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

fn runtime_config_with_private_organization(
    state: &AppState,
    config: &AppConfig,
    data: &AppData,
) -> Result<AppConfig, String> {
    build_runtime_organization_snapshot_from_parts(&state.data_path, config, &data.agents)
        .map(|snapshot| snapshot.config)
}

fn runtime_agents_with_private_organization(
    state: &AppState,
    config: &AppConfig,
    data: &AppData,
) -> Result<Vec<AgentProfile>, String> {
    build_runtime_organization_snapshot_from_parts(&state.data_path, config, &data.agents)
        .map(|snapshot| snapshot.agents)
}

fn private_agent_operation_error(agent_id: &str) -> String {
    format!("当前人格来自私有工作区，不能直接在主配置中修改：{agent_id}")
}

fn is_newer_version(current: &str, latest: &str) -> bool {
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

const GITHUB_REPO_PAGE: &str = "https://github.com/kawayiYokami/P-ai";

fn set_preferred_release_source(state: &AppState, source: &str) {
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

async fn probe_release_source_once(state: &AppState) {
    set_preferred_release_source(state, "github");
}



fn set_github_update_method_inner(
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



fn normalize_ui_language(value: &str) -> String {
    match value.trim() {
        "en-US" => "en-US".to_string(),
        "zh-TW" => "zh-TW".to_string(),
        _ => "zh-CN".to_string(),
    }
}


fn set_ui_language_inner(
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



fn get_department_default_draft_inner(
    state: &AppState,
    department_id: &str,
) -> Result<DepartmentConfig, String> {
    let config = load_config_inner(state)?;
    default_department_draft(department_id, &config.ui_language)
}

fn load_config_inner(state: &AppState) -> Result<AppConfig, String> {
    let mut result = state_read_config_cached(&state)?;
    let config_changed_by_normalize = normalize_app_config_and_detect_changes(&mut result);
    let workspace_changed = ensure_default_shell_workspace_in_config(&mut result, &state);
    let remote_im_private_state_migrated =
        remote_im_migrate_channel_private_states(&state, &mut result)?;
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

fn read_app_bootstrap_snapshot(state: &AppState) -> Result<AppBootstrapSnapshot, String> {
    let mut config = state_read_config_cached(state)?;
    let config_changed_by_normalize = normalize_app_config_and_detect_changes(&mut config);
    let workspace_changed = ensure_default_shell_workspace_in_config(&mut config, state);
    let remote_im_private_state_migrated =
        remote_im_migrate_channel_private_states(state, &mut config)?;
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










fn removed_remote_im_channels(
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

fn stop_removed_remote_im_channel_runtimes(
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
            if let Err(err) =
                remote_im_delete_channel_private_state(&state, &channel.platform, &channel_id)
            {
                runtime_log_error(format!(
                    "[远程IM] 删除渠道后清理私有状态失败: channel_id={}, platform={:?}, error={}",
                    channel_id, channel.platform, err
                ));
            }
        }
    });
}

fn save_config_inner(
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
    remote_im_migrate_channel_private_states(&state, &mut config)?;
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
    Ok(runtime_config)
}
