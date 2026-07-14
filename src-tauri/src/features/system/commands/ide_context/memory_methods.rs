fn ide_chat_list_memories_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(list_memories_inner(state)?)
}

fn ide_chat_delete_memory_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<DeleteMemoryInput>(params, "input")?;
    ide_chat_serialize(delete_memory_inner(state, input)?)
}

fn ide_chat_preview_export_memories_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(preview_export_memories_inner(state)?)
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
            }),
        _ => None,
    };
    ide_chat_serialize(export_memories_inner(state, selected_scopes)?)
}

fn ide_chat_import_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ImportMemoriesInput>(params, "input")?;
    ide_chat_serialize(import_memories_inner(state, input)?)
}

fn ide_chat_preview_import_angel_memories_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<PreviewImportAngelMemoriesInput>(params, "input")?;
    ide_chat_serialize(preview_import_angel_memories_inner(input)?)
}

fn ide_chat_import_angel_memories_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<ImportAngelMemoriesInput>(params, "input")?;
    ide_chat_serialize(import_angel_memories_inner(state, input)?)
}

fn ide_chat_search_memories_mixed_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SearchMemoriesMixedInput>(params, "input")?;
    ide_chat_serialize(search_memories_mixed_inner(state, input)?)
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
    ide_chat_serialize(get_memory_provider_bindings_inner(state)?)
}

fn ide_chat_get_memory_embedding_sync_progress_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(get_memory_embedding_sync_progress_inner(state)?)
}

fn ide_chat_test_memory_embedding_provider_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestMemoryEmbeddingProviderInput>(params, "input")?;
    ide_chat_serialize(test_memory_embedding_provider_inner(input, state)?)
}

fn ide_chat_test_memory_rerank_provider_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<TestMemoryRerankProviderInput>(params, "input")?;
    ide_chat_serialize(test_memory_rerank_provider_inner(input, state)?)
}

fn ide_chat_save_memory_embedding_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveMemoryEmbeddingBindingInput>(params, "input")?;
    ide_chat_serialize(save_memory_embedding_binding_inner(input, state)?)
}

fn ide_chat_save_memory_rerank_binding_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<SaveMemoryRerankBindingInput>(params, "input")?;
    ide_chat_serialize(save_memory_rerank_binding_inner(input, state)?)
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
