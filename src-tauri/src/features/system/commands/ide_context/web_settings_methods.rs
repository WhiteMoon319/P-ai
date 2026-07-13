fn ide_chat_load_config_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(load_config_inner(state)?)
}

fn ide_chat_load_app_bootstrap_snapshot_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(read_app_bootstrap_snapshot(state)?)
}

fn ide_chat_save_config_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    ide_context_runtime: &IdeContextRuntime,
    params: Value,
) -> Result<Value, String> {
    let config = ide_chat_parse_param_field::<AppConfig>(params, "config")?;
    ide_chat_serialize(save_config_inner(config, app, state, ide_context_runtime)?)
}

fn ide_chat_load_agents_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(load_agents_inner(state)?)
}

async fn ide_chat_list_unarchived_conversations_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let app_state = state.clone();
    let summaries = tokio::task::spawn_blocking(move || list_unarchived_conversations_blocking(&app_state))
        .await
        .map_err(|err| format!("读取未归档会话列表任务异常：{err}"))??;
    ide_chat_serialize(summaries)
}

fn ide_chat_remote_im_list_contact_conversations_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(conversation_service_v2().list_remote_im_contact_conversations(state)?)
}

fn ide_chat_list_delegate_conversations_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(list_delegate_conversations_inner(state)?)
}

async fn ide_chat_get_prompt_preview_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SessionSelector>(params.clone(), "input")?;
    let preview_mode = ide_chat_parse_optional_param_field::<String>(params, "previewMode")?;
    ide_chat_serialize(get_prompt_preview_inner(input, preview_mode, state).await?)
}

async fn ide_chat_get_system_prompt_preview_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SessionSelector>(params, "input")?;
    ide_chat_serialize(get_prompt_preview_inner(input, None, state).await.map(|preview| {
        SystemPromptPreview {
            system_prompt: preview.preamble,
        }
    })?)
}

fn ide_chat_save_agents_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveAgentsInput>(params, "input")?;
    ide_chat_serialize(save_agents_inner(input, app, state)?)
}

fn ide_chat_load_chat_settings_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(load_chat_settings_inner(state)?)
}

fn ide_chat_save_chat_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ChatSettings>(params, "input")?;
    let patch = ChatSettingsPatch {
        assistant_department_agent_id: Some(input.assistant_department_agent_id),
        user_alias: Some(input.user_alias),
        response_style_id: Some(input.response_style_id),
        pdf_read_mode: Some(input.pdf_read_mode),
        background_voice_screenshot_keywords: Some(input.background_voice_screenshot_keywords),
        background_voice_screenshot_mode: Some(input.background_voice_screenshot_mode),
        instruction_presets: Some(input.instruction_presets),
    };
    ide_chat_serialize(patch_chat_settings_inner(patch, app, state)?)
}

fn ide_chat_patch_chat_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ChatSettingsPatch>(params, "input")?;
    ide_chat_serialize(patch_chat_settings_inner(input, app, state)?)
}

fn ide_chat_patch_conversation_api_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ConversationApiSettingsPatch>(params, "input")?;
    ide_chat_serialize(patch_conversation_api_settings_inner(input, app, state)?)
}

fn ide_chat_save_conversation_api_settings_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ConversationApiSettings>(params, "input")?;
    let patch = ConversationApiSettingsPatch {
        assistant_department_api_config_id: Some(input.assistant_department_api_config_id),
        vision_api_config_id: Some(input.vision_api_config_id),
        tool_review_api_config_id: Some(input.tool_review_api_config_id),
        stt_api_config_id: Some(input.stt_api_config_id),
        stt_auto_send: Some(input.stt_auto_send),
    };
    ide_chat_serialize(patch_conversation_api_settings_inner(patch, app, state)?)
}

fn ide_chat_avatar_data_url_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<AvatarDataPathInput>(params, "input")?;
    ide_chat_serialize(read_avatar_data_url_inner(input, state)?)
}

fn ide_chat_save_agent_avatar_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveAgentAvatarInput>(params, "input")?;
    ide_chat_serialize(save_agent_avatar_inner(input, state)?)
}

fn ide_chat_clear_agent_avatar_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ClearAgentAvatarInput>(params, "input")?;
    clear_agent_avatar_inner(input, state)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_sync_tray_icon_for_web_settings(app: &AppHandle) -> Result<Value, String> {
    sync_default_tray_icon(app)?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_refresh_models_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<RefreshModelsInput>(params, "input")?;
    ide_chat_serialize(refresh_models_inner(state, input).await?)
}

async fn ide_chat_quick_genai_chat_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<QuickGenaiChatInput>(params, "input")?;
    ide_chat_serialize(quick_genai_chat_inner(state, input).await?)
}

async fn ide_chat_fetch_model_metadata_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<FetchModelMetadataInput>(params, "input")?;
    ide_chat_serialize(fetch_model_metadata_inner(state, input).await?)
}

async fn ide_chat_test_embedding_connection_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestEmbeddingConnectionInput>(params, "input")?;
    ide_chat_serialize(test_embedding_connection_inner(input).await?)
}

async fn ide_chat_test_rerank_connection_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestRerankConnectionInput>(params, "input")?;
    ide_chat_serialize(test_rerank_connection_inner(input).await?)
}

async fn ide_chat_test_voice_connection_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestVoiceConnectionInput>(params, "input")?;
    ide_chat_serialize(test_voice_connection_inner(input).await?)
}

fn ide_chat_resolve_model_adapter_kind_for_web_settings(params: Value) -> Result<Value, String> {
    let model_name = match params {
        Value::Object(mut map) => map
            .remove("modelName")
            .or_else(|| map.remove("model_name"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(resolve_model_adapter_kind_label(&model_name))
}

fn ide_chat_check_tools_status_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CheckToolsStatusInput>(params, "input")?;
    ide_chat_serialize(check_tools_status_inner(input, state)?)
}

fn ide_chat_get_image_text_cache_stats_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(get_image_text_cache_stats_inner(state)?)
}

fn ide_chat_clear_image_text_cache_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(clear_image_text_cache_inner(state)?)
}

async fn ide_chat_list_tool_catalog_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(list_tool_catalog_inner(state).await?)
}

async fn ide_chat_list_department_permission_catalog_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(list_department_permission_catalog_inner(state).await?)
}

fn ide_chat_open_external_url_for_web_settings(params: Value) -> Result<Value, String> {
    let url = match params {
        Value::Object(mut map) => map
            .remove("url")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    open_external_url(url)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_read_local_chat_image_thumbnail_for_web_settings(
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ReadLocalChatImageThumbnailInput>(params, "input")?;
    ide_chat_serialize(read_local_chat_image_thumbnail(input)?)
}

fn ide_chat_read_local_chat_image_original_for_web_settings(
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ReadLocalChatImageThumbnailInput>(params, "input")?;
    ide_chat_serialize(read_local_chat_image_original(input)?)
}

async fn ide_chat_web_access_info_for_web_settings(
    app: &AppHandle,
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<Value, String> {
    ide_chat_serialize(get_web_access_info_inner(app, state, ide_context_runtime, false).await?)
}

include!("memory_methods.rs");

fn ide_chat_task_list_tasks_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(task_store_list_tasks(&state.data_path)?)
}

fn ide_chat_task_get_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskGetInput>(params, "input")?;
    ide_chat_serialize(task_store_get_task(&state.data_path, input.task_id.trim())?)
}

fn ide_chat_task_create_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskCreateInput>(params, "input")?;
    let input = task_create_input_for_write(state, &input)?;
    let task = task_store_create_task(&state.data_path, &input)?;
    task_scheduler_notify_changed(state);
    ide_chat_serialize(task)
}

fn ide_chat_task_update_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskUpdateInput>(params, "input")?;
    let input = task_update_input_for_write(state, &input)?;
    let task = task_store_update_task(&state.data_path, &input)?;
    task_scheduler_notify_changed(state);
    ide_chat_serialize(task)
}

fn ide_chat_task_complete_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskCompleteInput>(params, "input")?;
    let task = task_store_complete_task(&state.data_path, &input)?;
    task_scheduler_notify_changed(state);
    ide_chat_serialize(task)
}

fn ide_chat_task_delete_task_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskDeleteInput>(params, "input")?;
    task_store_delete_task(&state.data_path, input.task_id.trim())?;
    task_scheduler_notify_changed(state);
    Ok(serde_json::json!(null))
}

fn ide_chat_task_list_run_logs_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskRunLogListInput>(params, "input")?;
    ide_chat_serialize(task_store_list_run_logs(
        &state.data_path,
        input.task_id.as_deref(),
        input.limit.unwrap_or(50),
    )?)
}

async fn ide_chat_task_optimize_draft_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TaskOptimizeDraftInput>(params, "input")?;
    ide_chat_serialize(task_optimize_draft_internal(input, state).await?)
}

include!("mcp_methods.rs");

async fn ide_chat_get_storage_usage_overview_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(start_storage_overview_refresh_if_needed(state.clone(), false).await)
}

async fn ide_chat_refresh_storage_usage_overview_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(start_storage_overview_refresh_if_needed(state.clone(), true).await)
}

async fn ide_chat_get_usage_overview_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(start_usage_overview_refresh_if_needed(state.clone(), false).await)
}

async fn ide_chat_refresh_usage_overview_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(start_usage_overview_refresh_if_needed(state.clone(), true).await)
}

fn ide_chat_open_storage_usage_item_directory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<OpenStorageUsageItemDirectoryInput>(params, "input")?;
    let item_id = input.item_id.trim();
    let target = storage_usage_target_path(state, item_id)
        .ok_or_else(|| format!("未知存储分类：{item_id}"))?;
    let app_root = app_root_from_data_path(&state.data_path);
    let open_dir = storage_existing_directory_for_open(&target)?;
    let canonical_root = app_root.canonicalize().unwrap_or(app_root);
    let canonical_open_dir = open_dir.canonicalize().unwrap_or(open_dir.clone());
    if !canonical_open_dir.starts_with(&canonical_root) {
        return Err(format!(
            "拒绝打开应用私有目录之外的路径，path={}",
            canonical_open_dir.display()
        ));
    }
    open_shell_path_in_file_manager(&canonical_open_dir)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_cleanup_storage_legacy_items_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CleanupStorageLegacyItemsInput>(params, "input")?;
    let cleanup_kind = input.cleanup_kind.trim();
    let (scope, label) = match cleanup_kind {
        STORAGE_CLEANUP_LEGACY_CONVERSATIONS => (
            StorageLegacyConversationScope::Normal,
            "旧普通会话 JSON",
        ),
        STORAGE_CLEANUP_LEGACY_DELEGATE_CONVERSATIONS => (
            StorageLegacyConversationScope::Delegate,
            "旧委托会话 JSON",
        ),
        _ => return Err(format!("未知存储清理类型：{cleanup_kind}")),
    };
    let _migration_guard = lock_message_store_migration();
    runtime_log_info(format!(
        "[存储] 开始，任务=清理{}，cleanup_kind={}",
        label,
        cleanup_kind
    ));
    let started_at = std::time::Instant::now();
    let result = cleanup_storage_legacy_scope(state, scope);
    match &result {
        Ok(report) => runtime_log_warn(format!(
            "[存储] 完成，任务=清理{}，cleanup_kind={}，删除文件数={}，跳过文件数={}，释放字节={}，耗时毫秒={}",
            label,
            cleanup_kind,
            report.deleted_file_count,
            report.skipped_file_count,
            report.freed_bytes,
            started_at.elapsed().as_millis()
        )),
        Err(err) => runtime_log_error(format!(
            "[存储] 失败，任务=清理{}，cleanup_kind={}，error={}，耗时毫秒={}",
            label,
            cleanup_kind,
            err,
            started_at.elapsed().as_millis()
        )),
    }
    ide_chat_serialize(result?)
}

fn ide_chat_migration_error_message(err: MigrationCommandError) -> String {
    match err.code {
        Some(code) if !code.trim().is_empty() => format!("{}: {}", code, err.message),
        _ => err.message,
    }
}

fn ide_chat_uploaded_migration_package_path(
    state: &AppState,
    input: &PreviewImportConfigMigrationPackageInput,
) -> Result<Option<PathBuf>, String> {
    let bytes_base64 = input
        .package_bytes_base64
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let Some(bytes_base64) = bytes_base64 else {
        return Ok(None);
    };
    let bytes = B64
        .decode(bytes_base64)
        .map_err(|err| format!("解析迁移包上传内容失败: {err}"))?;
    let upload_dir = migration_temp_root(state).join("uploads");
    fs::create_dir_all(&upload_dir)
        .map_err(|err| format!("创建迁移包上传临时目录失败: {err}"))?;
    let extension = input
        .package_file_name
        .as_deref()
        .and_then(|name| Path::new(name).extension())
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| value.eq_ignore_ascii_case("zip"))
        .unwrap_or("zip");
    let path = upload_dir.join(format!("{}.{}", Uuid::new_v4(), extension));
    fs::write(&path, bytes).map_err(|err| format!("写入迁移包上传临时文件失败: {err}"))?;
    Ok(Some(path))
}

fn ide_chat_export_config_migration_package_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ExportConfigMigrationPackageInput>(params, "input")?;
    validate_export_migration_password(&input.password)?;
    let total_started_at = std::time::Instant::now();
    runtime_log_info(format!(
        "[迁移包导出] 开始 task=export_config_migration_package trigger=web_settings password_present={} password_len={}",
        !input.password.trim().is_empty(),
        input.password.chars().count()
    ));
    let payload_started_at = std::time::Instant::now();
    let payload = build_export_payload(state)?;
    runtime_log_debug(format!(
        "[迁移包导出] 完成 task=export_config_migration_package trigger=web_settings stage=build_export_payload provider_count={} api_config_count={} memory_count={} duration_ms={}",
        payload.config.api_providers.len(),
        payload.config.api_configs.len(),
        payload.memories.len(),
        payload_started_at.elapsed().as_millis()
    ));
    let manifest = MigrationManifest {
        schema_version: MIGRATION_SCHEMA_VERSION,
        migration_version: payload.runtime_data.data_migration_version.max(DATA_MIGRATION_VERSION_V1_BASELINE),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        exported_at: now_iso(),
    };
    let exports_dir = app_root_from_data_path(&state.data_path).join("exports");
    fs::create_dir_all(&exports_dir).map_err(|err| format!("创建导出目录失败: {err}"))?;
    let file_name = format!(
        "p-ai-migration-{}.zip",
        now_iso()
            .replace(':', "-")
            .replace('/', "-")
            .replace('\\', "-")
    );
    let path = exports_dir.join(&file_name);
    write_migration_package(&path, input.password.trim(), &manifest, &payload)?;
    let bytes = fs::read(&path).map_err(|err| format!("读取迁移包失败: {err}"))?;
    runtime_log_debug(format!(
        "[迁移包导出] 完成 task=export_config_migration_package trigger=web_settings stage=write_migration_package path={} provider_count={} api_config_count={} memory_count={} total_duration_ms={}",
        path.to_string_lossy(),
        payload.config.api_providers.len(),
        payload.config.api_configs.len(),
        payload.memories.len(),
        total_started_at.elapsed().as_millis()
    ));
    ide_chat_serialize(ExportConfigMigrationPackageResult {
        path: path.to_string_lossy().to_string(),
        provider_count: payload.config.api_providers.len(),
        api_config_count: payload.config.api_configs.len(),
        memory_count: payload.memories.len(),
        file_name,
        bytes_base64: B64.encode(bytes),
    })
}

fn ide_chat_preview_import_config_migration_package_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let mut input =
        ide_chat_parse_param_field::<PreviewImportConfigMigrationPackageInput>(params, "input")?;
    if let Some(uploaded_path) = ide_chat_uploaded_migration_package_path(state, &input)? {
        input.package_path = Some(uploaded_path.to_string_lossy().to_string());
    }
    let package_path = input
        .package_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| "迁移包路径不能为空".to_string())?;
    let preview_id = Uuid::new_v4().to_string();
    let preview_dir = migration_preview_dir(state, &preview_id);
    unzip_migration_package_to_dir(&package_path, input.password.trim(), &preview_dir)
        .map_err(ide_chat_migration_error_message)?;
    let (manifest, payload) = read_preview_payload(&preview_dir)?;
    let package_version = assert_manifest_version(&manifest, &payload)?;

    let current_config = state_read_config_cached(state)?;
    let memory_preview = preview_memory_import(state, &preview_dir, &payload.memories)?;
    let (_, provider_added_count, provider_updated_count) =
        merge_api_providers(&current_config.api_providers, &payload.config.api_providers);
    let (_, api_config_added_count, api_config_updated_count) =
        merge_api_configs(&current_config.api_configs, &payload.config.api_configs);
    state
        .migration_preview_dirs
        .lock()
        .map_err(|err| format!("锁定迁移预检目录失败: {err}"))?
        .insert(preview_id.clone(), preview_dir.to_string_lossy().to_string());

    ide_chat_serialize(PreviewImportConfigMigrationPackageResult {
        preview_id,
        package_version: format_migration_version_label(package_version),
        memory_added_count: memory_preview.created_count,
        memory_merged_count: memory_preview.merged_count,
        provider_added_count,
        provider_updated_count,
        api_config_added_count,
        api_config_updated_count,
        oauth_file_count: payload.oauth_files.len(),
        avatar_file_count: payload.avatar_files.len(),
    })
}

fn ide_chat_apply_import_config_migration_package_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ApplyImportConfigMigrationPackageInput>(params, "input")?;
    let preview_dir = state
        .migration_preview_dirs
        .lock()
        .map_err(|err| format!("锁定迁移预检目录失败: {err}"))?
        .remove(input.preview_id.trim())
        .ok_or_else(|| "迁移预检已失效，请重新选择迁移包。".to_string())?;
    let preview_dir = PathBuf::from(preview_dir);
    let (manifest, payload) = read_preview_payload(&preview_dir)?;
    assert_manifest_version(&manifest, &payload)?;
    let backup_dir = backup_current_migration_targets(state)?;
    let current_config = state_read_config_cached(state)?;
    let current_data = state_read_agents_runtime_snapshot(state)?;
    let (
        final_config,
        provider_added_count,
        provider_updated_count,
        api_config_added_count,
        api_config_updated_count,
    ) = build_imported_config(&current_config, &payload.config);
    let avatar_path_map = write_avatar_files(state, &payload.avatar_files)?;
    write_oauth_files(&final_config, &payload.oauth_files)?;
    let final_data = build_imported_runtime(&current_data, &payload.runtime_data, &avatar_path_map);
    let memory_stats = memory_store_import_memories(&state.data_path, &payload.memories)?;
    state_write_config_cached(state, &final_config)?;
    state_write_agents_cached(state, &final_data.agents)?;
    state_write_runtime_state_cached(state, &build_runtime_state_file(&final_data))?;
    if let Err(err) = fs::remove_dir_all(&preview_dir) {
        runtime_log_warn(format!(
            "[迁移包导入] 失败 task=apply_import_config_migration_package stage=remove_preview_dir path={} err={:?}",
            preview_dir.display(),
            err
        ));
    }
    let result = ApplyImportConfigMigrationPackageResult {
        imported_memory_count: memory_stats.imported_count,
        created_memory_count: memory_stats.created_count,
        merged_memory_count: memory_stats.merged_count,
        provider_added_count,
        provider_updated_count,
        api_config_added_count,
        api_config_updated_count,
        backup_dir: backup_dir.to_string_lossy().to_string(),
    };
    let app_handle = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        graceful_restart_app(&app_handle);
    });
    ide_chat_serialize(result)
}

fn ide_chat_list_recent_llm_round_logs_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let capacity = llm_round_log_capacity_for_state(state);
    let logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    ide_chat_serialize(recent_llm_round_logs_for_ui(&logs, capacity))
}

fn ide_chat_get_recent_llm_round_log_section_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let (id, section) = match params {
        Value::Object(mut map) => {
            let id = map
                .remove("id")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_default();
            let section = map
                .remove("section")
                .and_then(|value| value.as_str().map(ToOwned::to_owned))
                .unwrap_or_default();
            (id, section)
        }
        _ => (String::new(), String::new()),
    };
    let id = id.trim().to_string();
    if id.is_empty() {
        return Ok(serde_json::json!(null));
    }
    let logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    ide_chat_serialize(logs.pipeline_logs.iter().rev().chain(logs.other_logs.iter().rev()).find_map(|entry| {
        find_llm_round_log_entry_by_id(entry, &id)
            .and_then(|entry| llm_round_log_section_value(entry, &section))
    }))
}

fn ide_chat_clear_recent_llm_round_logs_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let mut logs = state
        .llm_round_logs
        .lock()
        .map_err(|_| "Failed to lock llm round logs".to_string())?;
    logs.pipeline_logs.clear();
    logs.other_logs.clear();
    pending_chat_round_buffer()
        .lock()
        .map_err(|_| "Failed to lock pending chat round logs".to_string())?
        .rounds_by_chat_session
        .clear();
    ide_chat_serialize(true)
}

fn ide_chat_list_terminal_shell_candidates_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let (preferred_kind, current, options) = terminal_shell_candidates_for_ui(state);
    Ok(serde_json::json!({
        "preferredKind": preferred_kind,
        "currentKind": current.kind,
        "currentPath": current.path,
        "options": options,
    }))
}

fn ide_chat_open_chat_shell_workspace_dir_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_optional_param_field::<ShellWorkspacePathInput>(params, "input")?;
    let root = resolve_requested_shell_workspace_root(
        state,
        input.as_ref().and_then(|value| value.workspace_path.as_deref()),
        true,
    )?;
    open_shell_path_in_file_manager(&root)?;
    ide_chat_serialize(shell_workspace_display_path(&root))
}

fn ide_chat_reset_chat_shell_workspace_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_optional_param_field::<ShellWorkspacePathInput>(params, "input")?;
    let root = resolve_requested_shell_workspace_root(
        state,
        input.as_ref().and_then(|value| value.workspace_path.as_deref()),
        true,
    )?;
    ensure_workspace_mcp_layout_at_root(&root)?;
    ensure_workspace_skills_layout_at_root(&root)?;
    ensure_workspace_private_organization_layout_at_root(&root)?;
    ide_chat_serialize(shell_workspace_display_path(&root))
}

fn ide_chat_get_default_chat_shell_workspace_path_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let root = terminal_default_session_root_canonical(state)?;
    ide_chat_serialize(shell_workspace_display_path(&root))
}

async fn ide_chat_migrate_shell_workspace_directory_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<MigrateWorkspaceDirectoryInput>(params, "input")?;
    ide_chat_serialize(migrate_shell_workspace_directory(input, app.clone()).await?)
}

async fn ide_chat_install_host_runtime_prerequisite_for_web_settings(
    params: Value,
) -> Result<Value, String> {
    let kind = match params {
        Value::Object(mut map) => map
            .remove("kind")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    ide_chat_serialize(install_host_runtime_prerequisite(kind).await?)
}

fn ide_chat_get_host_runtime_prerequisites_for_web_settings() -> Result<Value, String> {
    ide_chat_serialize(get_host_runtime_prerequisites())
}

fn ide_chat_show_window_for_web_settings(app: &AppHandle, label: &str) -> Result<Value, String> {
    show_window(app, label)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_open_runtime_logs_window_for_web_settings(app: &AppHandle) -> Result<Value, String> {
    show_runtime_logs_window(app)?;
    Ok(serde_json::json!(null))
}

fn ide_chat_set_webview_zoom_percent_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let percent = match params {
        Value::Object(mut map) => map
            .remove("percent")
            .and_then(|value| value.as_u64())
            .unwrap_or(100),
        _ => 100,
    };
    let normalized = apply_webview_zoom_percent(app, percent as u32)?;
    emit_webview_zoom_percent_updated(app, normalized);
    ide_chat_serialize(normalized)
}

fn ide_chat_set_github_update_method_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let update_method = match params {
        Value::Object(mut map) => map
            .remove("updateMethod")
            .or_else(|| map.remove("update_method"))
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
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
    ide_chat_serialize(runtime_config)
}

fn ide_chat_set_skipped_github_update_version_for_web_settings(
    state: &AppState,
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let version = match params {
        Value::Object(mut map) => map
            .remove("version")
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_default(),
        _ => String::new(),
    };
    let normalized = normalize_skipped_github_update_version(&version);
    let mut config = state_read_config_cached(state)?;
    normalize_app_config(&mut config);
    if config.skipped_github_update_version != normalized {
        config.skipped_github_update_version = normalized.clone();
        state_write_config_cached(state, &config)?;
        runtime_log_warn(format!("[自动更新] 已保存跳过版本：version={normalized}"));
    }
    sync_update_state_from_skip_version(app, &normalized);
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;
    let _ = app.emit("easy-call:config-updated", &runtime_config);
    ide_chat_serialize(runtime_config)
}

fn ide_chat_get_github_update_state_for_web_settings(app: &AppHandle) -> Result<Value, String> {
    ide_chat_serialize(get_github_update_state(app.clone())?)
}

async fn ide_chat_check_github_update_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let (update_method, respect_cooldown) = match params {
        Value::Object(mut map) => {
            let update_method = map
                .remove("updateMethod")
                .or_else(|| map.remove("update_method"))
                .and_then(|value| value.as_str().map(ToOwned::to_owned));
            let respect_cooldown = map
                .remove("respectCooldown")
                .or_else(|| map.remove("respect_cooldown"))
                .or_else(|| map.remove("useCachedResult"))
                .or_else(|| map.remove("use_cached_result"))
                .and_then(|value| value.as_bool());
            (update_method, respect_cooldown)
        }
        _ => (None, None),
    };
    ide_chat_serialize(check_github_update(app.clone(), update_method, respect_cooldown).await?)
}

async fn ide_chat_start_github_update_for_web_settings(
    app: &AppHandle,
    params: Value,
) -> Result<Value, String> {
    let (force, update_method) = match params {
        Value::Object(mut map) => {
            let force = map
                .remove("force")
                .and_then(|value| value.as_bool())
                .unwrap_or(false);
            let update_method = map
                .remove("updateMethod")
                .or_else(|| map.remove("update_method"))
                .and_then(|value| value.as_str().map(ToOwned::to_owned));
            (force, update_method)
        }
        _ => (false, None),
    };
    start_github_update(app.clone(), force, update_method).await?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_cancel_github_update_for_web_settings() -> Result<Value, String> {
    cancel_github_update().await?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_apply_prepared_github_update_for_web_settings(
    app: &AppHandle,
) -> Result<Value, String> {
    apply_prepared_github_update(app.clone()).await?;
    Ok(serde_json::json!(null))
}

async fn ide_chat_codex_get_auth_status_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexAuthStatusInput>(params, "input")?;
    ide_chat_serialize(codex_get_auth_status(input).await?)
}

async fn ide_chat_codex_start_oauth_login_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexStartOAuthLoginInput>(params, "input")?;
    ide_chat_serialize(codex_start_oauth_login(input).await?)
}

async fn ide_chat_codex_get_rate_limits_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexGetRateLimitsInput>(params, "input")?;
    ide_chat_serialize(codex_get_rate_limits(input).await?)
}

async fn ide_chat_codex_consume_rate_limit_reset_credit_for_web_settings(
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexGetRateLimitsInput>(params, "input")?;
    ide_chat_serialize(codex_consume_rate_limit_reset_credit(input).await?)
}

fn ide_chat_codex_logout_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<CodexLogoutInput>(params, "input")?;
    ide_chat_serialize(codex_logout(input)?)
}

include!("remote_im_methods.rs");
