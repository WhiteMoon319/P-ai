#[tauri::command]
fn show_main_window(app: AppHandle) -> Result<(), String> {
    show_window(&app, "main")
}

#[tauri::command]
fn show_chat_window(app: AppHandle) -> Result<(), String> {
    show_window(&app, "chat")
}

#[tauri::command]
fn show_archives_window(app: AppHandle) -> Result<(), String> {
    show_window(&app, "archives")
}

#[tauri::command]
fn show_quick_setup_window(app: AppHandle) -> Result<(), String> {
    show_window(&app, "quick-setup")
}

#[tauri::command]
fn open_runtime_logs_window(app: AppHandle) -> Result<(), String> {
    show_runtime_logs_window(&app)
}

#[tauri::command]
fn hide_current_window(window: tauri::Window) -> Result<(), String> {
    window.hide().map_err(|err| format!("隐藏当前窗口失败：{err}"))
}

#[tauri::command]
fn complete_quick_setup_and_open_chat(app: AppHandle) -> Result<(), String> {
    show_window(&app, "chat")?;
    if let Some(window) = app.get_webview_window("quick-setup") {
        let _ = window.hide();
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebviewZoomUpdatedPayload {
    percent: u32,
}

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

fn emit_webview_zoom_percent_updated(app: &AppHandle, percent: u32) {
    let _ = app.emit(
        "easy-call:webview-zoom-updated",
        WebviewZoomUpdatedPayload { percent },
    );
}

fn apply_webview_zoom_percent(app: &AppHandle, percent: u32) -> Result<u32, String> {
    let normalized = normalize_webview_zoom_percent(percent);
    let scale_factor = normalized as f64 / 100.0;
    let mut failed = Vec::new();
    for (label, window) in app.webview_windows() {
        if let Err(err) = window.set_zoom(scale_factor) {
            failed.push(format!("{label}: {err}"));
        }
    }
    if !failed.is_empty() {
        return Err(format!("应用界面缩放失败：{}", failed.join("；")));
    }
    Ok(normalized)
}

#[tauri::command]
fn set_webview_zoom_percent(percent: u32, app: AppHandle) -> Result<u32, String> {
    let normalized = apply_webview_zoom_percent(&app, percent)?;
    emit_webview_zoom_percent_updated(&app, normalized);
    Ok(normalized)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetachedChatWindowInput {
    conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetachedChatWindowOutput {
    conversation_id: String,
    window_label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_notification_conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DetachedChatWindowInfo {
    detached: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    window_label: Option<String>,
}

#[tauri::command]
fn detach_current_conversation_to_window(
    input: DetachedChatWindowInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<DetachedChatWindowOutput, String> {
    let conversation_id = input.conversation_id.trim();
    eprintln!(
        "[独立聊天窗口] 收到独立窗口请求：conversation_id={}",
        conversation_id
    );
    if conversation_id.is_empty() {
        eprintln!("[独立聊天窗口] 拒绝：当前没有可独立出去的会话");
        return Err("当前没有可独立出去的会话。".to_string());
    }
    if get_conversation_runtime_state(&state, conversation_id)? == MainSessionState::OrganizingContext {
        eprintln!(
            "[独立聊天窗口] 拒绝：会话正在整理上下文 conversation_id={}",
            conversation_id
        );
        return Err("当前会话正在整理上下文，暂时不能独立出去。".to_string());
    }
    let runtime = state_read_runtime_state_cached(&state)?;
    let system_notification_conversation_id = runtime
        .main_conversation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    if system_notification_conversation_id.as_deref() == Some(conversation_id) {
        eprintln!(
            "[独立聊天窗口] 拒绝：系统通知会话不能独立打开 conversation_id={}",
            conversation_id
        );
        return Err("系统通知会话不能独立打开，请选择一个普通会话。".to_string());
    }

    let conversation_meta = conversation_service_v2().get_conversation_meta(&state, conversation_id)?;
    if !conversation_meta.summary.trim().is_empty()
        || (!conversation_meta.visible_in_foreground_lists
            && !conversation_meta.is_remote_im_contact)
    {
        eprintln!(
            "[独立聊天窗口] 拒绝：会话不在前台列表 conversation_id={}",
            conversation_id
        );
        return Err("只能独立打开未归档前台会话或远程联系人会话。".to_string());
    }
    let title = if !conversation_meta.title.trim().is_empty() {
        conversation_meta.title.trim().to_string()
    } else if let Some(summary_title) = conversation_meta
        .latest_summary_title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        summary_title.to_string()
    } else if let Some(preview_title) = conversation_meta
        .preview_messages
        .iter()
        .find(|message| message.role.trim().eq_ignore_ascii_case("user"))
        .map(|message| message.text_preview.trim())
        .filter(|value| !value.is_empty())
    {
        preview_title.chars().take(12).collect()
    } else {
        conversation_id.to_string()
    };
    eprintln!(
        "[独立聊天窗口] 准备创建窗口：conversation_id={}，title={}",
        conversation_id,
        title
    );
    let window_label = open_detached_chat_window(&app, conversation_id, Some(&title))?;
    eprintln!(
        "[独立聊天窗口] 窗口请求已登记：conversation_id={}，window_label={}",
        conversation_id,
        window_label
    );
    clear_conversation_list_activity_mark(&state, conversation_id);
    emit_unarchived_conversation_overview_updated_from_state(&state)?;
    Ok(DetachedChatWindowOutput {
        conversation_id: conversation_id.to_string(),
        window_label,
        system_notification_conversation_id,
    })
}

#[tauri::command]
fn get_detached_chat_window_info(window: tauri::Window) -> Result<DetachedChatWindowInfo, String> {
    let label = window.label().trim().to_string();
    if !is_detached_chat_window_label(&label) {
        return Ok(DetachedChatWindowInfo {
            detached: false,
            conversation_id: None,
            window_label: Some(label),
        });
    }
    Ok(DetachedChatWindowInfo {
        detached: true,
        conversation_id: detached_chat_conversation_for_window(&label),
        window_label: Some(label),
    })
}

#[tauri::command]
fn set_chat_window_active(active: bool) {
    static CHAT_WINDOW_INACTIVE_LOGGED_ONCE: std::sync::atomic::AtomicBool =
        std::sync::atomic::AtomicBool::new(false);
    if !active && !CHAT_WINDOW_INACTIVE_LOGGED_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        eprintln!("[系统] 聊天窗口激活状态变更：跳过");
    }
    set_record_hotkey_probe_chat_window_active(active);
}

#[tauri::command]
fn get_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
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
            eprintln!(
                "set_preferred_release_source 锁定 preferred_release_source 失败：source={}, err={}",
                source,
                err
            );
        }
    }
}

async fn probe_release_source_once(state: &AppState) {
    set_preferred_release_source(state, "github");
}

#[tauri::command]
fn get_project_repository_url(_state: State<'_, AppState>) -> String {
    GITHUB_REPO_PAGE.to_string()
}

#[tauri::command]
fn set_github_update_method(
    update_method: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppConfig, String> {
    let normalized = normalize_github_update_method(&update_method);
    let mut config = state_read_config_cached(&state)?;
    normalize_app_config(&mut config);
    if config.github_update_method != normalized {
        config.github_update_method = normalized.clone();
        state_write_config_cached(&state, &config)?;
        eprintln!("[自动更新] 更新方式偏好已保存：method={normalized}");
    }
    let data = state_read_agents_runtime_snapshot(&state)?;
    let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(runtime_config)
}

#[tauri::command]
fn set_skipped_github_update_version(
    version: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppConfig, String> {
    let normalized = normalize_skipped_github_update_version(&version);
    let mut config = state_read_config_cached(&state)?;
    normalize_app_config(&mut config);
    if config.skipped_github_update_version != normalized {
        config.skipped_github_update_version = normalized.clone();
        state_write_config_cached(&state, &config)?;
        eprintln!("[自动更新] 已保存跳过版本：version={normalized}");
    }
    let data = state_read_agents_runtime_snapshot(&state)?;
    let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
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

#[tauri::command]
fn set_ui_language(
    ui_language: String,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AppConfig, String> {
    let normalized = normalize_ui_language(&ui_language);
    let mut config = state_read_config_cached(&state)?;
    normalize_app_config(&mut config);
    if config.ui_language != normalized {
        config.ui_language = normalized.clone();
        state_write_config_cached(&state, &config)?;
        eprintln!("[配置] 界面语言已保存：ui_language={normalized}");
    }
    let data = state_read_agents_runtime_snapshot(&state)?;
    let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(runtime_config)
}

#[tauri::command]
fn load_config(state: State<'_, AppState>) -> Result<AppConfig, String> {
    load_config_inner(&state)
}

fn load_config_inner(state: &AppState) -> Result<AppConfig, String> {
    let mut result = state_read_config_cached(&state)?;
    normalize_app_config(&mut result);
    let workspace_changed = ensure_default_shell_workspace_in_config(&mut result, &state);
    let remote_im_private_state_migrated =
        remote_im_migrate_channel_private_states(&state, &mut result)?;
    if workspace_changed || remote_im_private_state_migrated {
        state_write_config_cached(&state, &result)?;
    }
    let _ = run_app_data_migrations_with_state(&state, &result)?;
    let runtime_agents = state_read_agents_cached(&state)?;
    let snapshot =
        build_runtime_organization_snapshot_from_parts(&state.data_path, &result, &runtime_agents)?;
    Ok(snapshot.config)
}

fn read_app_bootstrap_snapshot(state: &AppState) -> Result<AppBootstrapSnapshot, String> {
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    let workspace_changed = ensure_default_shell_workspace_in_config(&mut config, state);
    let remote_im_private_state_migrated =
        remote_im_migrate_channel_private_states(state, &mut config)?;
    if workspace_changed || remote_im_private_state_migrated {
        state_write_config_cached(state, &config)?;
    }
    let _ = run_app_data_migrations_with_state(state, &config)?;
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

#[tauri::command]
fn load_app_bootstrap_snapshot(state: State<'_, AppState>) -> Result<AppBootstrapSnapshot, String> {
    read_app_bootstrap_snapshot(&state)
}

#[tauri::command]
fn is_backend_ready(state: State<'_, AppState>) -> bool {
    state.backend_ready.load(std::sync::atomic::Ordering::Acquire)
}

#[tauri::command]
fn webview_pong(window: tauri::Window) {
    webview_record_pong(window.label());
}

#[tauri::command]
fn debug_crash_webview(webview: tauri::Webview) -> Result<(), String> {
    webview.eval("(function(){const a=[];while(true){a.push(new Array(1000000).fill('x'));}})();")
        .map_err(|err| format!("注入崩溃脚本失败：{err}"))
}

#[tauri::command]
fn list_system_fonts() -> Result<Vec<String>, String> {
    let mut families = font_kit::source::SystemSource::new()
        .all_families()
        .map_err(|err| format!("列出系统字体失败：{err}"))?;
    families.sort_by_key(|name| name.to_ascii_lowercase());
    families.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    Ok(families)
}

fn validate_record_hotkey_available(config: &AppConfig) -> Result<String, String> {
    let normalized = normalize_record_hotkey_label(&config.record_hotkey)?;
    if normalized.is_empty() {
        return Ok(normalized);
    }
    let record_signature = record_hotkey_signature(&normalized)
        .ok_or_else(|| format!("录音热键格式无效：{}", normalized))?;
    let summon_signature = record_hotkey_signature(&config.hotkey);
    if summon_signature.as_deref() == Some(record_signature.as_str()) {
        return Err(format!(
            "录音热键 {} 不能与呼唤热键 {} 相同。",
            normalized, config.hotkey
        ));
    }
    Ok(normalized)
}

#[tauri::command]
fn update_record_hotkey(
    input: UpdateRecordHotkeyInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RecordHotkeyUpdateResult, String> {
    let mut config = state_read_config_cached(&state)?;
    normalize_app_config(&mut config);
    let candidate = input.record_hotkey.trim();
    if config.record_hotkey.trim() == candidate {
        let normalized = normalize_record_hotkey_label(&config.record_hotkey)?;
        if normalized != config.record_hotkey {
            config.record_hotkey = normalized.clone();
            state_write_config_cached(&state, &config)?;
            let data = state_read_agents_runtime_snapshot(&state)?;
            let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
            let _ = app.emit("easy-call:config-updated", &runtime_config);
        }
        return Ok(RecordHotkeyUpdateResult {
            record_hotkey: config.record_hotkey.clone(),
            record_background_wake_enabled: config.record_background_wake_enabled,
            min_record_seconds: config.min_record_seconds,
            max_record_seconds: config.max_record_seconds,
        });
    }
    let normalized = {
        let mut next = config.clone();
        next.record_hotkey = candidate.to_string();
        validate_record_hotkey_available(&next)?
    };
    config.record_hotkey = normalized.clone();
    state_write_config_cached(&state, &config)?;
    set_record_hotkey_probe_hotkey(&normalized)?;
    let data = state_read_agents_runtime_snapshot(&state)?;
    let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(RecordHotkeyUpdateResult {
        record_hotkey: normalized,
        record_background_wake_enabled: config.record_background_wake_enabled,
        min_record_seconds: config.min_record_seconds,
        max_record_seconds: config.max_record_seconds,
    })
}

#[tauri::command]
fn update_record_background_wake(
    input: UpdateRecordBackgroundWakeInput,
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<RecordHotkeyUpdateResult, String> {
    let mut config = state_read_config_cached(&state)?;
    normalize_app_config(&mut config);
    let next_enabled = input.enabled;
    if config.record_background_wake_enabled != next_enabled {
        config.record_background_wake_enabled = next_enabled;
        state_write_config_cached(&state, &config)?;
    }
    set_record_hotkey_probe_background_wake_enabled(config.record_background_wake_enabled);
    runtime_log_info(format!(
        "[录音热键] 完成，任务=后台唤醒切换，enabled={}",
        config.record_background_wake_enabled
    ));
    let data = state_read_agents_runtime_snapshot(&state)?;
    let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    Ok(RecordHotkeyUpdateResult {
        record_hotkey: config.record_hotkey.clone(),
        record_background_wake_enabled: config.record_background_wake_enabled,
        min_record_seconds: config.min_record_seconds,
        max_record_seconds: config.max_record_seconds,
    })
}

#[tauri::command]
fn save_config(
    config: AppConfig,
    app: AppHandle,
    state: State<'_, AppState>,
    ide_context_runtime: State<'_, IdeContextRuntime>,
) -> Result<AppConfig, String> {
    save_config_inner(config, &app, &state, &ide_context_runtime)
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
    tauri::async_runtime::spawn(async move {
        for channel in channels {
            let channel_id = channel.id.trim().to_string();
            if channel_id.is_empty() {
                continue;
            }
            match channel.platform {
                RemoteImPlatform::OnebotV11 => {
                    if let Err(err) = onebot_v11_ws_manager().stop_channel(&channel_id).await {
                        eprintln!(
                            "[远程IM] 删除渠道后停止 OneBot v11 运行态失败: channel_id={}, error={}",
                            channel_id, err
                        );
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
                eprintln!(
                    "[远程IM] 删除渠道后清理私有状态失败: channel_id={}, platform={:?}, error={}",
                    channel_id, channel.platform, err
                );
            }
        }
    });
}

fn save_config_inner(
    config: AppConfig,
    app: &AppHandle,
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
        if let Err(err) = register_hotkey_from_config(&app, &main_config) {
            eprintln!(
                "[热键] 召唤热键运行时注册失败，配置已保存但该热键暂不可用：hotkey={}, err={}",
                main_config.hotkey,
                err
            );
        }
    }
    if !main_config.web_access_enabled && IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst) {
        eprintln!(
            "[网络访问] 配置已关闭，停止 Web 访问服务: port={}",
            base_config.web_access_port
        );
        tauri::async_runtime::spawn(async move {
            shutdown_web_access_server().await;
        });
    } else if main_config.web_access_enabled
        && (!base_config.web_access_enabled
            || ((base_config.web_access_port != main_config.web_access_port
                || base_config.web_access_password != main_config.web_access_password)
                && IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst)))
    {
        eprintln!(
            "[网络访问] 配置已启用、端口或密码已变更，重启 Web 访问服务: old_enabled={}, new_enabled={}, old_port={}, new_port={}",
            base_config.web_access_enabled,
            main_config.web_access_enabled,
            base_config.web_access_port,
            main_config.web_access_port
        );
        let app = app.clone();
        let state = state.clone();
        let ide_context_runtime = ide_context_runtime.clone();
        tauri::async_runtime::spawn(async move {
            restart_web_access_server(
                app,
                state,
                ide_context_runtime,
            )
            .await;
        });
    }
    match apply_webview_zoom_percent(&app, main_config.webview_zoom_percent) {
        Ok(percent) => emit_webview_zoom_percent_updated(&app, percent),
        Err(err) => eprintln!("[外观] 应用界面缩放失败：{}", err),
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
