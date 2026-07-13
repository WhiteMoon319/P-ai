fn ide_chat_jsonrpc_success(id: Option<Value>, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn ide_chat_jsonrpc_error(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": IdeChatJsonRpcError {
            code,
            message: message.into(),
        },
    })
}

fn ide_chat_parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value::<T>(params).map_err(|err| format!("invalid params: {err}"))
}

fn ide_chat_parse_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<T, String> {
    match params {
        Value::Object(mut map) => map
            .remove(field)
            .ok_or_else(|| format!("{field} is required"))
            .and_then(ide_chat_parse_params::<T>),
        _ => Err(format!("{field} is required")),
    }
}

fn ide_chat_parse_optional_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<Option<T>, String> {
    match params {
        Value::Object(mut map) => map
            .remove(field)
            .map(ide_chat_parse_params::<T>)
            .transpose(),
        _ => Ok(None),
    }
}

fn ide_chat_serialize<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|err| format!("serialize result failed: {err}"))
}

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

fn ide_chat_list_memories_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(memory_store_list_memories(&state.data_path)?)
}

fn ide_chat_delete_memory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<DeleteMemoryInput>(params, "input")?;
    memory_store_delete_memory(&state.data_path, &input.memory_id)?;
    ide_chat_serialize(DeleteMemoryResult {
        status: "deleted".to_string(),
    })
}

fn ide_chat_preview_export_memories_for_web_settings(state: &AppState) -> Result<Value, String> {
    let owner_scope_by_agent = load_importable_agent_scope_labels(state)?;
    let scopes = build_export_scope_items(&state.data_path, &owner_scope_by_agent)?;
    ide_chat_serialize(PreviewExportMemoriesResult {
        total_count: scopes.iter().map(|item| item.count).sum(),
        scopes,
    })
}

fn ide_chat_export_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let selected_scopes = match params {
        Value::Object(mut map) => map
            .remove("input")
            .and_then(|value| {
                value
                    .get("scopes")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.as_str().unwrap_or_default().to_string())
                            .collect::<Vec<_>>()
                    })
            })
            .map(|scopes| normalize_selected_export_scopes(&scopes))
            .transpose()?,
        _ => None,
    };
    let owner_scope_by_agent = load_importable_agent_scope_labels(state)?;
    ide_chat_serialize(build_memory_exchange_payload(
        &state.data_path,
        &owner_scope_by_agent,
        selected_scopes.as_ref(),
    )?)
}

fn ide_chat_export_memories_to_path_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ExportMemoriesToPathInput>(params, "input")?;
    let target = PathBuf::from(input.path.trim());
    if input.path.trim().is_empty() {
        return Err("导出路径不能为空".to_string());
    }
    let parent = target
        .parent()
        .ok_or_else(|| "导出路径缺少父目录".to_string())?;
    fs::create_dir_all(parent).map_err(|err| format!("创建导出目录失败: {err}"))?;
    let selected_scopes = normalize_selected_export_scopes(&input.scopes)?;
    let owner_scope_by_agent = load_importable_agent_scope_labels(state)?;
    let payload = build_memory_exchange_payload(
        &state.data_path,
        &owner_scope_by_agent,
        Some(&selected_scopes),
    )?;
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("序列化导出记忆备份失败: {err}"))?;
    fs::write(&target, body).map_err(|err| format!("写入导出记忆备份失败: {err}"))?;
    ide_chat_serialize(ExportMemoriesFileResult {
        path: target.to_string_lossy().to_string(),
        count: payload.records.len(),
    })
}

fn ide_chat_import_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ImportMemoriesInput>(params, "input")?;
    let stats = memory_store_import_memories(&state.data_path, &input.memories)?;
    ide_chat_serialize(ImportMemoriesResult {
        imported_count: stats.imported_count,
        created_count: stats.created_count,
        merged_count: stats.merged_count,
        total_count: stats.total_count,
    })
}

fn ide_chat_preview_import_angel_memories_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<PreviewImportAngelMemoriesInput>(params, "input")?;
    let parsed = parse_angel_memory_payload(&input.payload)?;
    ide_chat_serialize(PreviewImportAngelMemoriesResult {
        total_count: parsed.len(),
        scopes: build_preview_scope_items(&parsed),
        samples: sampled_angel_memory_preview_items(&parsed, 10),
    })
}

fn ide_chat_import_angel_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ImportAngelMemoriesInput>(params, "input")?;
    let parsed = parse_angel_memory_payload(&input.payload)?;
    let scope_targets = resolve_import_scope_targets(state, &parsed, &input.scope_agent_mappings)?;
    let stats = import_angel_memories_by_scope(&state.data_path, &parsed, &scope_targets)?;
    ide_chat_serialize(ImportMemoriesResult {
        imported_count: stats.imported_count,
        created_count: stats.created_count,
        merged_count: stats.merged_count,
        total_count: stats.total_count,
    })
}

fn ide_chat_search_memories_mixed_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SearchMemoriesMixedInput>(params, "input")?;
    let started = std::time::Instant::now();
    let query = input.query.trim();
    if query.is_empty() {
        return ide_chat_serialize(SearchMemoriesMixedResult {
            memories: memory_store_list_memories(&state.data_path)?
                .into_iter()
                .map(|memory| SearchMemoriesMixedHit {
                    memory,
                    bm25_score: 0.0,
                    bm25_raw_score: 0.0,
                    vector_score: 0.0,
                    rerank_score: 0.0,
                    final_score: 0.0,
                })
                .collect::<Vec<_>>(),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let memories = memory_store_list_memories(&state.data_path)?;
    let ranked = memory_mixed_ranked_items(
        &state.data_path,
        &memories,
        query,
        MEMORY_MATCH_MAX_ITEMS * MEMORY_CANDIDATE_MULTIPLIER,
        0.0,
    );
    if ranked.is_empty() {
        return ide_chat_serialize(SearchMemoriesMixedResult {
            memories: Vec::new(),
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let memory_map = memories
        .into_iter()
        .map(|memory| (memory.id.clone(), memory))
        .collect::<std::collections::HashMap<_, _>>();
    let mut out = Vec::<SearchMemoriesMixedHit>::new();
    for item in ranked {
        if let Some(memory) = memory_map.get(&item.memory_id) {
            out.push(SearchMemoriesMixedHit {
                memory: memory.clone(),
                bm25_score: item.bm25_score,
                bm25_raw_score: item.bm25_raw_score,
                vector_score: item.vector_score,
                rerank_score: item.rerank_score,
                final_score: item.final_score,
            });
        }
    }
    ide_chat_serialize(SearchMemoriesMixedResult {
        memories: out,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn ide_chat_search_chat_history_slices_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ChatHistorySearchInput>(params, "input")?;
    ide_chat_serialize(chat_history_search_for_agent(state, &input)?)
}

fn ide_chat_get_memory_provider_bindings_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let conn = memory_store_open(&state.data_path)?;
    ide_chat_serialize(MemoryProviderBindings {
        embedding_api_config_id: memory_store_get_runtime_state(
            &conn,
            KB_STATE_EMBEDDING_API_CONFIG_ID,
        )?,
        rerank_api_config_id: memory_store_get_runtime_state(
            &conn,
            KB_STATE_RERANK_API_CONFIG_ID,
        )?,
    })
}

fn ide_chat_get_memory_embedding_sync_progress_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    let conn = memory_store_open(&state.data_path)?;
    ide_chat_serialize(MemoryEmbeddingSyncProgress {
        status: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_STATUS)?
            .unwrap_or_else(|| "idle".to_string()),
        done_batches: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_DONE_BATCHES)?
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0),
        total_batches: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_TOTAL_BATCHES)?
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0),
        trace_id: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_TRACE_ID)?,
        error: memory_store_get_runtime_state(&conn, KB_STATE_REBUILD_ERROR)?,
    })
}

fn ide_chat_test_memory_embedding_provider_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestMemoryEmbeddingProviderInput>(params, "input")?;
    let started = std::time::Instant::now();
    let provider_id = input.provider_id.as_deref().unwrap_or("openai_embedding");
    let provider_kind = memory_provider_kind_from_id(provider_id);
    if matches!(provider_kind, MemoryProviderKind::VllmRerank) {
        return Err("rerank provider cannot be used as embedding provider.".to_string());
    }
    let app_config = read_config(&state.config_path)?;
    let provider_cfg = memory_resolve_provider_api_config(
        &app_config,
        provider_kind,
        input.api_config_id.as_deref(),
        provider_id,
    )
    .ok_or_else(|| "No matching API config for embedding test.".to_string())?;
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provider = memory_create_embedding_provider(provider_kind, &provider_cfg, model_name)?;
    let text = input
        .text
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("memory embedding connectivity test")
        .to_string();
    let vectors = provider.embed_batch(&vec![text])?;
    let first = vectors
        .first()
        .ok_or_else(|| "embedding test returned empty vectors".to_string())?;
    let dim = first.len();
    if dim == 0 {
        return Err("embedding test returned zero-dim vector".to_string());
    }
    ide_chat_serialize(TestMemoryEmbeddingProviderResult {
        provider_kind: format!("{provider_kind:?}"),
        model_name: model_name.unwrap_or(provider_cfg.model.trim()).to_string(),
        vector_dim: dim,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn ide_chat_test_memory_rerank_provider_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestMemoryRerankProviderInput>(params, "input")?;
    let started = std::time::Instant::now();
    let app_config = read_config(&state.config_path)?;
    let provider_kind = MemoryProviderKind::VllmRerank;
    let provider_cfg = memory_resolve_provider_api_config(
        &app_config,
        provider_kind,
        input.api_config_id.as_deref(),
        "vllm_rerank",
    )
    .ok_or_else(|| "No matching API config for rerank test.".to_string())?;
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provider = memory_create_rerank_provider(provider_kind, &provider_cfg, model_name)?;
    let query = input
        .query
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("用户偏好什么风格？")
        .to_string();
    let documents = input.documents.unwrap_or_else(|| {
        vec![
            "用户偏好简洁回答，尽量直接结论。".to_string(),
            "用户喜欢复杂铺垫和长篇解释。".to_string(),
            "今天主要讨论了记忆系统检索。".to_string(),
        ]
    });
    let results = provider.rerank(&query, &documents, Some(3))?;
    let top = results.iter().max_by(|a, b| {
        a.relevance_score
            .partial_cmp(&b.relevance_score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ide_chat_serialize(TestMemoryRerankProviderResult {
        provider_kind: format!("{provider_kind:?}"),
        model_name: model_name.unwrap_or(provider_cfg.model.trim()).to_string(),
        elapsed_ms: started.elapsed().as_millis(),
        result_count: results.len(),
        top_index: top.map(|item| item.index),
        top_score: top.map(|item| item.relevance_score),
    })
}

fn ide_chat_save_memory_embedding_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveMemoryEmbeddingBindingInput>(params, "input")?;
    let api_id = input.api_config_id.trim();
    if api_id.is_empty() {
        let conn = memory_store_open(&state.data_path)?;
        let old_provider_id =
            memory_store_get_runtime_state(&conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID)?;
        memory_store_set_runtime_state(&conn, KB_STATE_EMBEDDING_API_CONFIG_ID, "")?;
        memory_store_set_runtime_state(&conn, KB_STATE_ACTIVE_INDEX_PROVIDER_ID, "")?;
        return ide_chat_serialize(MemoryStoreProviderSyncReport {
            status: "disabled".to_string(),
            old_provider_id,
            new_provider_id: String::new(),
            deleted: 0,
            added: 0,
            batch_count: 0,
        });
    }

    let app_config = read_config(&state.config_path)?;
    let api = app_config
        .api_configs
        .iter()
        .find(|item| item.id == api_id)
        .cloned()
        .ok_or_else(|| "Selected embedding API config not found.".to_string())?;
    let provider_kind = match api.request_format {
        RequestFormat::OpenAIEmbedding => MemoryProviderKind::OpenAIEmbedding,
        RequestFormat::GeminiEmbedding => MemoryProviderKind::GeminiEmbedding,
        _ => {
            return Err(format!(
                "request_format '{}' is not embedding protocol.",
                api.request_format
            ))
        }
    };
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(api.model.trim());
    if model_name.is_empty() {
        return Err("Embedding model is empty.".to_string());
    }
    let provider_cfg = MemoryProviderApiConfig {
        base_url: api.base_url.clone(),
        api_key: api.api_key.clone(),
        model: api.model.clone(),
    };
    let provider = memory_create_embedding_provider(provider_kind, &provider_cfg, Some(model_name))?;
    let provider_id = memory_binding_provider_id(&api.id, api.request_format.as_str(), model_name);
    let batch_size = input.batch_size.unwrap_or(64).max(1);
    let report = memory_store_sync_provider_index(
        &state.data_path,
        &provider_id,
        model_name,
        batch_size,
        false,
        |texts| provider.embed_batch(texts),
    )?;

    let conn = memory_store_open(&state.data_path)?;
    memory_store_set_runtime_state(&conn, KB_STATE_EMBEDDING_API_CONFIG_ID, &api.id)?;
    ide_chat_serialize(report)
}

fn ide_chat_save_memory_rerank_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveMemoryRerankBindingInput>(params, "input")?;
    let api_id = input.api_config_id.trim();
    if api_id.is_empty() {
        let conn = memory_store_open(&state.data_path)?;
        memory_store_set_runtime_state(&conn, KB_STATE_RERANK_API_CONFIG_ID, "")?;
        return ide_chat_serialize(SaveMemoryRerankBindingResult {
            status: "disabled".to_string(),
            rerank_api_config_id: None,
            model_name: String::new(),
        });
    }
    let app_config = read_config(&state.config_path)?;
    let api = app_config
        .api_configs
        .iter()
        .find(|item| item.id == api_id)
        .cloned()
        .ok_or_else(|| "Selected rerank API config not found.".to_string())?;
    if !matches!(api.request_format, RequestFormat::OpenAIRerank) {
        return Err(format!(
            "request_format '{}' is not rerank protocol.",
            api.request_format
        ));
    }
    let model_name = input
        .model_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(api.model.trim());
    if model_name.is_empty() {
        return Err("Rerank model is empty.".to_string());
    }

    let conn = memory_store_open(&state.data_path)?;
    memory_store_set_runtime_state(&conn, KB_STATE_RERANK_API_CONFIG_ID, &api.id)?;
    ide_chat_serialize(SaveMemoryRerankBindingResult {
        status: "saved".to_string(),
        rerank_api_config_id: Some(api.id),
        model_name: model_name.to_string(),
    })
}

fn ide_chat_get_agent_private_memory_count_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<AgentPrivateMemoryCountInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let config = read_config(&state.config_path)?;
    let agents = state_read_agents_cached(state)?;
    let (private_agent_ids, _) = runtime_private_organization_ids(&state.data_path, &config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    ide_chat_serialize(AgentPrivateMemoryCountResult {
        count: memory_store_count_private_memories_by_agent(&state.data_path, agent_id)?,
    })
}

fn ide_chat_set_agent_memory_recall_mode_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SetAgentMemoryRecallModeInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let mode = match input.mode.trim().to_ascii_lowercase().as_str() {
        MEMORY_RECALL_MODE_AUTO => MEMORY_RECALL_MODE_AUTO.to_string(),
        MEMORY_RECALL_MODE_MANUAL => MEMORY_RECALL_MODE_MANUAL.to_string(),
        MEMORY_RECALL_MODE_OFF => MEMORY_RECALL_MODE_OFF.to_string(),
        _ => return Err("memoryRecallMode must be auto, manual, or off".to_string()),
    };

    let mut agents = state_read_agents_cached(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|agent| agent.id == agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    if normalize_agent_memory_recall_mode(&agents[agent_idx].memory_recall_mode) != mode {
        agents[agent_idx].memory_recall_mode = mode.clone();
        state_write_agents_cached(state, &agents)?;
        runtime_log_info(format!(
            "[记忆] 完成，任务=切换人格回忆模式，agent_id={}，mode={}",
            agent_id, mode
        ));
    }
    ide_chat_serialize(SetAgentMemoryRecallModeResult {
        agent_id: agent_id.to_string(),
        mode,
    })
}

fn ide_chat_set_agent_private_memory_enabled_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SetAgentPrivateMemoryEnabledInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|agent| agent.id == agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    let current = agents[agent_idx].private_memory_enabled;
    if current == input.enabled {
        return ide_chat_serialize(SetAgentPrivateMemoryEnabledResult {
            agent_id: agent_id.to_string(),
            enabled: current,
            exported_count: 0,
            deleted_count: 0,
            export_path: None,
        });
    }

    if input.enabled {
        agents[agent_idx].private_memory_enabled = true;
        state_write_agents_cached(state, &agents)?;
        return ide_chat_serialize(SetAgentPrivateMemoryEnabledResult {
            agent_id: agent_id.to_string(),
            enabled: true,
            exported_count: 0,
            deleted_count: 0,
            export_path: None,
        });
    }

    let export = memory_store_export_agent_private_memories(&state.data_path, agent_id)?;
    let deleted = memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id)?;
    agents[agent_idx].private_memory_enabled = false;
    state_write_agents_cached(state, &agents)?;
    ide_chat_serialize(SetAgentPrivateMemoryEnabledResult {
        agent_id: agent_id.to_string(),
        enabled: false,
        exported_count: export.count,
        deleted_count: deleted,
        export_path: Some(export.path),
    })
}

fn ide_chat_export_agent_private_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ExportAgentPrivateMemoriesInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let config = read_config(&state.config_path)?;
    let agents = state_read_agents_cached(state)?;
    let (private_agent_ids, _) = runtime_private_organization_ids(&state.data_path, &config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    let export = memory_store_export_agent_private_memories(&state.data_path, agent_id)?;
    ide_chat_serialize(ExportAgentPrivateMemoriesResult {
        count: export.count,
        path: export.path,
    })
}

fn ide_chat_disable_agent_private_memory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<DisableAgentPrivateMemoryInput>(params, "input")?;
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|agent| agent.id == agent_id && !agent.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    if !agents[agent_idx].private_memory_enabled {
        return ide_chat_serialize(DisableAgentPrivateMemoryResult {
            agent_id: agent_id.to_string(),
            enabled: false,
            deleted_count: 0,
        });
    }

    let deleted = memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id)?;
    agents[agent_idx].private_memory_enabled = false;
    state_write_agents_cached(state, &agents)?;
    ide_chat_serialize(DisableAgentPrivateMemoryResult {
        agent_id: agent_id.to_string(),
        enabled: false,
        deleted_count: deleted,
    })
}

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

fn ide_chat_mcp_list_servers_for_web_settings(state: &AppState) -> Result<Value, String> {
    let mut out = load_workspace_mcp_servers(state)?;
    for item in &mut out {
        *item = overlay_runtime_state_on_server(item.clone());
    }
    ide_chat_serialize(out)
}

fn ide_chat_mcp_validate_definition_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpDefinitionValidateInput>(params, "input")?;
    let _schema = mcp_definition_json_schema();
    let result = match normalize_mcp_definition_for_validation(&input.definition_json) {
        Ok((normalized_value, migrated)) => {
            let normalized_text = serde_json::to_string(&normalized_value)
                .map_err(|err| format!("序列化标准化 MCP 定义失败：{err}"))?;
            let (name, parsed) = parse_mcp_server_definition(&normalized_text)?;
            let _ = migrated;
            McpDefinitionValidateResult {
                ok: true,
                transport: Some(parsed.transport.as_str().to_string()),
                server_name: Some(name),
                message: "MCP definition is valid".to_string(),
                schema_version: None,
                error_code: None,
                details: Vec::new(),
                migrated_definition_json: None,
            }
        }
        Err(err) => McpDefinitionValidateResult {
            ok: false,
            transport: None,
            server_name: None,
            message: err.message,
            schema_version: None,
            error_code: Some(err.code),
            details: err.details,
            migrated_definition_json: None,
        },
    };
    ide_chat_serialize(result)
}

fn ide_chat_mcp_save_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerInput>(params, "input")?;
    let next = normalize_mcp_server_input(input)?;
    save_workspace_mcp_server(state, &next)?;
    let mut saved = load_server_by_id(state, &next.id)?;
    saved = overlay_runtime_state_on_server(saved);
    ide_chat_serialize(saved)
}

async fn ide_chat_mcp_remove_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let removed = remove_workspace_mcp_server(state, server_id)?;
    if removed {
        mcp_disconnect_cached_client(server_id).await;
        mcp_runtime_state_remove(server_id);
    }
    ide_chat_serialize(removed)
}

async fn ide_chat_mcp_list_server_tools_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let server = load_server_by_id(state, server_id)?;
    let started = std::time::Instant::now();
    mcp_runtime_state_mark_starting(&server);
    let tools = match mcp_list_server_tools_runtime(&server).await {
        Ok(tools) => tools,
        Err(err) => {
            let status = mcp_status_from_runtime_error(&err);
            mcp_runtime_state_mark_probe_failure(&server, status, &err);
            return Err(err);
        }
    };
    let discovered_names = tools
        .iter()
        .map(|t| t.tool_name.clone())
        .collect::<Vec<_>>();
    let merged_policies =
        merge_workspace_mcp_tool_policies_with_new_tools(state, &server.id, &discovered_names)?;
    let mut server_with_policies = server.clone();
    server_with_policies.tool_policies = merged_policies;
    let final_tools = tools
        .into_iter()
        .map(|tool| {
            let enabled = mcp_policy_enabled_for_tool(&server_with_policies, &tool.tool_name)
                && mcp_tool_allowed_by_definition(&server_with_policies, &tool.tool_name);
            McpToolDescriptor { enabled, ..tool }
        })
        .collect::<Vec<_>>();
    mcp_runtime_state_set(&server.id, true, "ready", "", final_tools.clone());
    refresh_global_tool_schema_cache(state);
    mark_prompt_cache_rebuild_for_all_final_system_sources(state);
    ide_chat_serialize(McpListServerToolsResult {
        server_id: server.id,
        tools: final_tools,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn ide_chat_mcp_list_server_tools_cached_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let server = load_server_by_id(state, server_id)?;
    let started = std::time::Instant::now();
    let tools = list_tools_from_runtime_or_policy(&server);
    ide_chat_serialize(McpListServerToolsResult {
        server_id: server.id,
        tools,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

fn ide_chat_mcp_deploy_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    let server = {
        let server = load_server_by_id(state, server_id)?;
        set_workspace_mcp_policy_enabled(state, server_id, true)?;
        server
    };
    let started = std::time::Instant::now();
    mcp_runtime_state_mark_starting(&server);
    mcp_start_supervisor_probe_for_server(state.clone(), server.clone(), "manual_deploy");
    let server_id = server.id.clone();
    let tools = list_tools_from_runtime_or_policy(&server);
    ide_chat_serialize(McpListServerToolsResult {
        server_id,
        tools,
        elapsed_ms: started.elapsed().as_millis() as u64,
    })
}

async fn ide_chat_mcp_undeploy_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    let server_id = input.server_id.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    {
        let _ = load_server_by_id(state, server_id)?;
        set_workspace_mcp_policy_enabled(state, server_id, false)?;
    }
    mcp_disconnect_cached_client(server_id).await;
    mcp_runtime_state_set(server_id, false, "stopped", "", Vec::new());
    let mut out = load_server_by_id(state, server_id)?;
    out = overlay_runtime_state_on_server(out);
    ide_chat_serialize(out)
}

fn ide_chat_mcp_set_tool_enabled_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpSetToolEnabledInput>(params, "input")?;
    let server_id = input.server_id.trim();
    let tool_name = input.tool_name.trim();
    if server_id.is_empty() {
        return Err("serverId is required".to_string());
    }
    if tool_name.is_empty() {
        return Err("toolName is required".to_string());
    }
    let policies = {
        let _ = load_server_by_id(state, server_id)?;
        let mut policies = load_workspace_mcp_tool_policies(state, server_id)?;
        if let Some(policy) = policies.iter_mut().find(|p| p.tool_name == tool_name) {
            policy.enabled = input.enabled;
        } else {
            policies.push(McpToolPolicy {
                tool_name: tool_name.to_string(),
                enabled: input.enabled,
            });
        }
        save_workspace_mcp_tool_policies(state, server_id, &policies)?;
        policies
    };
    mcp_runtime_state_set_tool_enabled(server_id, tool_name, input.enabled);
    let mut server = load_server_by_id(state, server_id)?;
    server.tool_policies = policies;
    server = overlay_runtime_state_on_server(server);
    ide_chat_serialize(server)
}

fn ide_chat_mcp_open_workspace_dir_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(open_mcp_workspace_dir(state)?)
}

async fn ide_chat_mcp_refresh_mcp_and_skills_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(reload_workspace(state).await?)
}

fn ide_chat_mcp_list_skills_for_web_settings(state: &AppState) -> Result<Value, String> {
    let (skills, errors) = load_workspace_skill_summaries_with_errors(state)?;
    let _ = update_hidden_skill_snapshot_cache(state, &skills, None);
    ide_chat_serialize(SkillListResult { skills, errors })
}

fn ide_chat_skill_open_workspace_dir_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(open_skills_workspace_dir(state)?)
}

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

fn ide_chat_send_message(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatSendInput>(params)?;
    let conversation_id = input.conversation_id.trim().to_string();
    let text = input.text.trim().to_string();
    let attachment_entries = normalize_payload_attachments(Some(&input.attachments));
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    if text.is_empty()
        && input.extra_text_blocks.iter().all(|item| item.trim().is_empty())
        && input
            .images
            .iter()
            .all(|item| item.bytes_base64.trim().is_empty())
        && attachment_entries.is_empty()
    {
        return Err("消息内容为空".to_string());
    }
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, &conversation_id)?;
    let agent_id = conversation_meta.agent_id.trim().to_string();
    if agent_id.is_empty() {
        return Err("会话信息不完整".to_string());
    }
    let department_id = conversation_meta.department_id.trim().to_string();
    if department_id.is_empty() {
        return Err("会话部门为空，无法从侧边栏发送。".to_string());
    }
    let request_id = runtime_context_request_id_or_new(None, None, "vscode-sidebar");
    let mut parts = if text.is_empty() {
        Vec::new()
    } else {
        vec![MessagePart::Text { text: text.clone(),
                reasoning_content: None,
            }]
    };
    for image in input.images {
        let mime = image.mime.trim().to_ascii_lowercase();
        let bytes_base64 = image.bytes_base64.trim().to_string();
        if !mime.starts_with("image/") || bytes_base64.is_empty() {
            continue;
        }
        parts.push(MessagePart::Image {
            mime,
            bytes_base64,
            name: image.name.and_then(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() { None } else { Some(trimmed) }
            }),
            compressed: false,
        });
    }
    if parts.is_empty()
        && input.extra_text_blocks.iter().all(|item| item.trim().is_empty())
        && attachment_entries.is_empty()
    {
        return Err("消息内容为空".to_string());
    }
    let provider_meta = merge_provider_meta_with_attachments(
        Some(serde_json::json!({
            "requestId": request_id,
            "source": "vscode_sidebar",
        })),
        &attachment_entries,
    );
    let user_message_id = Uuid::new_v4().to_string();
    let user_message = ChatMessage {
        id: user_message_id.clone(),
        role: "user".to_string(),
        created_at: now_iso(),
        speaker_agent_id: None,
        parts,
        extra_text_blocks: input
            .extra_text_blocks
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .collect(),
        provider_meta,
        tool_call: None,
        mcp_call: None,
        meme_annotations: None,
    };
    let event_id = Uuid::new_v4().to_string();
    let mut runtime_context = runtime_context_new("user_message", "user_send");
    runtime_context.request_id = Some(request_id.clone());
    runtime_context.dispatch_id = Some(event_id.clone());
    runtime_context.origin_conversation_id = Some(conversation_id.clone());
    runtime_context.target_conversation_id = Some(conversation_id.clone());
    runtime_context.root_conversation_id = Some(conversation_id.clone());
    runtime_context.executor_agent_id = Some(agent_id.clone());
    runtime_context.executor_department_id = Some(department_id.clone());
    let assistant_message_id = Uuid::new_v4().to_string();
    let event = ChatPendingEvent {
        id: event_id.clone(),
        conversation_id: conversation_id.clone(),
        created_at: now_iso(),
        source: ChatEventSource::User,
        queue_mode: ChatQueueMode::Normal,
        messages: vec![user_message],
        activate_assistant: true,
        assistant_message_id: Some(assistant_message_id.clone()),
        session_info: ChatSessionInfo {
            department_id,
            agent_id,
        },
        runtime_context: Some(runtime_context),
        sender_info: None,
    };
    let ingress = ingress_chat_event(state, event)?;
    let (accepted, duplicate, ingress_label) = match &ingress {
        ChatEventIngress::Direct(_) => (true, false, "direct"),
        ChatEventIngress::Queued { .. } => (true, false, "queued"),
        ChatEventIngress::Duplicate { .. } => (false, true, "duplicate"),
    };
    let queued = ingress_label == "queued";
    trigger_chat_event_after_ingress(state, ingress);
    Ok(serde_json::json!({
        "accepted": accepted,
        "duplicate": duplicate,
        "conversationId": conversation_id,
        "eventId": event_id,
        "traceId": request_id,
        "requestId": request_id,
        "ingress": ingress_label,
        "queued": queued,
        "userMessageId": user_message_id,
        "assistantMessageId": assistant_message_id,
    }))
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
    let removed = recall_queue_event(state, event_id)?;
    let message_text = removed
        .as_ref()
        .and_then(|event| {
            event.messages.first().and_then(|msg| {
                msg.parts.iter().find_map(|part| match part {
                    MessagePart::Text { text, .. } => Some(text.clone()),
                    _ => None,
                })
            })
        })
        .unwrap_or_default();
    serde_json::to_value(ChatQueueRecallResult {
        removed: removed.is_some(),
        message_text,
    })
    .map_err(|err| format!("serialize queue recall failed: {err}"))
}

fn ide_chat_mark_queue_event_guided(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatQueueEventInput>(params)?;
    let event_id = input.event_id.trim();
    if event_id.is_empty() {
        return Err("eventId is required".to_string());
    }
    let conversation_id = mark_queue_event_guided(state, event_id)?;
    if let Some(conversation_id) = conversation_id {
        trigger_guided_queue_processing(state, &conversation_id);
        return serde_json::to_value(true)
            .map_err(|err| format!("serialize queue guided result failed: {err}"));
    }
    serde_json::to_value(false).map_err(|err| format!("serialize queue guided result failed: {err}"))
}

fn ide_chat_stop_conversation(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatStopInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let (department_id, _agent_id) = resolve_runtime_control_department_and_agent(
        state,
        Some(conversation_meta.department_id.as_str()),
        Some(conversation_meta.agent_id.as_str()),
        Some(conversation_id),
    )?;
    let chat_key = inflight_chat_key(&department_id, Some(conversation_id));
    let aborted_chat = {
        let mut inflight = state
            .inflight_chat_abort_handles
            .lock()
            .map_err(|_| "Failed to lock inflight chat abort handles".to_string())?;
        inflight.remove(&chat_key).map(|handle| {
            handle.abort();
            true
        }).unwrap_or(false)
    };
    let aborted_tool = abort_inflight_tool_abort_handle(state, &chat_key)?;
    let aborted_delegate_children =
        abort_delegate_runtime_descendants_by_parent_context(state, &chat_key, Some(conversation_id))?;
    let cleared_queue_count = clear_conversation_queue(
        state,
        conversation_id,
        "消息已因 VS Code 侧边栏中断被清出队列",
    )?;
    let _ = release_conversation_processing_claim(state, conversation_id);
    let _ = set_conversation_runtime_state_and_emit(state, conversation_id, MainSessionState::Idle);
    let _ = set_conversation_remote_im_activation_sources(state, conversation_id, Vec::new());
    runtime_log_info(format!(
        "[聊天流式块][侧边栏停止] 停止请求完成 session={} conversation_id={} aborted={} persisted=false cleared_queue_count={}",
        chat_key,
        conversation_id,
        aborted_chat || aborted_tool || aborted_delegate_children > 0,
        cleared_queue_count,
    ));
    let stop_result = StopChatResult {
        aborted: aborted_chat || aborted_tool || aborted_delegate_children > 0,
        persisted: false,
        conversation_id: Some(conversation_id.to_string()),
        assistant_text: String::new(),
        assistant_message: None,
    };
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
        "clearedQueueCount": cleared_queue_count,
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

fn ide_chat_open_settings(app: &AppHandle) -> Result<Value, String> {
    show_window(app, "main")?;
    Ok(serde_json::json!({ "opened": true }))
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

fn ide_chat_read_plan_file(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatReadPlanFileInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required.".to_string());
    }
    let resolved = resolve_plan_file_for_conversation_id(state, conversation_id, input.path.trim())?;
    let content = read_plan_markdown_file(&resolved.canonical_path)?;
    Ok(serde_json::json!({ "content": content }))
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
