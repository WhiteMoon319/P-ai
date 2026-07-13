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
