
pub(crate) fn load_agents_inner(state: &AppState) -> Result<Vec<AgentProfile>, String> {
    let config = state_read_config_cached(&state)?;
    let data = state_read_agents_runtime_snapshot(&state)?;
    build_runtime_organization_snapshot_from_parts(&state.data_path, &config, &data.agents)
        .map(|snapshot| snapshot.agents)
}


#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConvertPrivateAgentToMainInput {
    pub(crate) agent_id: String,
}


pub(crate) fn convert_private_agent_to_main_inner(
    input: ConvertPrivateAgentToMainInput,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<Vec<AgentProfile>, String> {
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut runtime_agents = load_agents_inner(state)?;
    let target_idx = runtime_agents
        .iter()
        .position(|a| a.id == agent_id)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    if !is_private_workspace_source(&runtime_agents[target_idx].source) {
        return Err(format!("Agent '{}' is not a private-workspace persona.", agent_id));
    }

    runtime_agents[target_idx].source = "main_config".to_string();
    runtime_agents[target_idx].scope = "global".to_string();
    runtime_agents[target_idx].updated_at = now_iso();

    save_agents_inner(
        SaveAgentsInput {
            agents: runtime_agents,
        },
        app,
        state,
    )
}

pub(crate) fn save_agents_inner(
    input: SaveAgentsInput,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<Vec<AgentProfile>, String> {
    if input.agents.is_empty() {
        return Err("At least one agent is required.".to_string());
    }

    let base_config = read_config(&state.config_path)?;
    let mut previous_runtime_data = state_read_agents_runtime_snapshot(&state)?;
    previous_runtime_data.agents = state_read_agents_cached(&state)?;
    let previous_runtime_config =
        runtime_config_with_private_organization(&state, &base_config, &previous_runtime_data)?;
    let runtime = state_read_runtime_state_cached(&state)?;
    let mut data = AppData::default();
    data.agents = state_read_agents_cached(&state)?;
    apply_runtime_state_to_app_data(&mut data, &runtime);
    let previous_agents = data.agents.clone();
    let existing_user_persona = data
        .agents
        .iter()
        .find(|a| a.id == USER_PERSONA_ID)
        .cloned();
    let existing_system_persona = data
        .agents
        .iter()
        .find(|a| a.id == SYSTEM_PERSONA_ID)
        .cloned();
    let desired_agents = input.agents.clone();
    data.agents = input
        .agents
        .into_iter()
        .filter(|agent| !is_private_workspace_source(&agent.source))
        .collect();
    for agent in &mut data.agents {
        agent.memory_recall_mode = normalize_agent_memory_recall_mode(&agent.memory_recall_mode);
    }
    if !data.agents.iter().any(|a| a.id == USER_PERSONA_ID) {
        if let Some(user_persona) = existing_user_persona {
            data.agents.push(user_persona);
        }
    }
    if !data.agents.iter().any(|a| a.id == SYSTEM_PERSONA_ID) {
        if let Some(system_persona) = existing_system_persona {
            data.agents.push(system_persona);
        }
    }
    let next_ids = data
        .agents
        .iter()
        .map(|a| a.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let previous_by_id = previous_agents
        .iter()
        .map(|a| (a.id.clone(), a))
        .collect::<std::collections::HashMap<_, _>>();
    let removed_agent_ids = previous_agents
        .iter()
        .filter(|a| !a.is_built_in_user && !a.is_built_in_system && a.id != USER_PERSONA_ID && a.id != SYSTEM_PERSONA_ID)
        .filter(|a| !next_ids.contains(&a.id))
        .map(|a| a.id.clone())
        .collect::<Vec<_>>();
    let disabled_private_memory_agent_ids = data
        .agents
        .iter()
        .filter(|a| !a.is_built_in_user && !a.is_built_in_system && a.id != USER_PERSONA_ID && a.id != SYSTEM_PERSONA_ID)
        .filter(|a| {
            previous_by_id
                .get(&a.id)
                .map(|old| old.private_memory_enabled && !a.private_memory_enabled)
                .unwrap_or(false)
        })
        .map(|a| a.id.clone())
        .collect::<Vec<_>>();

    for agent_id in &removed_agent_ids {
        let started_at = std::time::Instant::now();
        runtime_log_info(format!(
            "[会话] 开始，任务=导出并删除私有记忆，status=开始，agent_id={}，trigger=agent_removed",
            agent_id
        ));
        let export = match memory_store_export_agent_private_memories(&state.data_path, agent_id) {
            Ok(export) => export,
            Err(error) => {
                let elapsed_ms = started_at.elapsed().as_millis();
                runtime_log_error(format!(
                    "[会话] 失败，任务=导出并删除私有记忆，status=失败，agent_id={}，trigger=agent_removed，stage=export，duration_ms={}，error={}",
                    agent_id, elapsed_ms, error
                ));
                return Err(error);
            }
        };
        let deleted = match memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id) {
            Ok(deleted) => deleted,
            Err(error) => {
                let elapsed_ms = started_at.elapsed().as_millis();
                runtime_log_error(format!(
                    "[会话] 失败，任务=导出并删除私有记忆，status=失败，agent_id={}，trigger=agent_removed，stage=delete，duration_ms={}，error={}",
                    agent_id, elapsed_ms, error
                ));
                return Err(error);
            }
        };
        let elapsed_ms = started_at.elapsed().as_millis();
        runtime_log_info(format!(
            "[会话] 完成，任务=导出并删除私有记忆，status=完成，agent_id={}，export.count={}，export.path={}，deleted={}，duration_ms={}",
            agent_id,
            export.count,
            export.path,
            deleted,
            elapsed_ms
        ));
    }
    for agent_id in &disabled_private_memory_agent_ids {
        let started_at = std::time::Instant::now();
        runtime_log_info(format!(
            "[会话] 开始，任务=导出并删除私有记忆，status=开始，agent_id={}，trigger=private_memory_disabled",
            agent_id
        ));
        let export = match memory_store_export_agent_private_memories(&state.data_path, agent_id) {
            Ok(export) => export,
            Err(error) => {
                let elapsed_ms = started_at.elapsed().as_millis();
                runtime_log_error(format!(
                    "[会话] 失败，任务=导出并删除私有记忆，status=失败，agent_id={}，trigger=private_memory_disabled，stage=export，duration_ms={}，error={}",
                    agent_id, elapsed_ms, error
                ));
                return Err(error);
            }
        };
        let deleted = match memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id) {
            Ok(deleted) => deleted,
            Err(error) => {
                let elapsed_ms = started_at.elapsed().as_millis();
                runtime_log_error(format!(
                    "[会话] 失败，任务=导出并删除私有记忆，status=失败，agent_id={}，trigger=private_memory_disabled，stage=delete，duration_ms={}，error={}",
                    agent_id, elapsed_ms, error
                ));
                return Err(error);
            }
        };
        let elapsed_ms = started_at.elapsed().as_millis();
        runtime_log_info(format!(
            "[会话] 完成，任务=导出并删除私有记忆，status=完成，agent_id={}，export.count={}，export.path={}，deleted={}，duration_ms={}",
            agent_id,
            export.count,
            export.path,
            deleted,
            elapsed_ms
        ));
    }

    let affected_agent_ids = previous_agents
        .iter()
        .chain(data.agents.iter())
        .filter(|a| !a.is_built_in_user && !a.is_built_in_system && a.id != USER_PERSONA_ID && a.id != SYSTEM_PERSONA_ID)
        .map(|a| a.id.clone())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    sync_private_agents_to_workspace(&state.data_path, &base_config, &desired_agents)?;
    state_write_agents_cached(&state, &data.agents)?;
    if !affected_agent_ids.is_empty() {
        mark_prompt_cache_rebuild_for_system_sources_by_agents(state, &affected_agent_ids);
    }
    let mut config = state_read_config_cached(&state)?;
    let runtime_agents = runtime_agents_with_private_organization(&state, &config, &data)?;
    let valid_agent_ids = runtime_agents
        .iter()
        .filter(|a| !a.is_built_in_user)
        .map(|a| a.id.clone())
        .collect::<std::collections::HashSet<_>>();
    let mut runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
    let mut config_changed = false;
    for dept in &mut runtime_config.departments {
        let original_agent_ids = dept.agent_ids.clone();
        dept.agent_ids.retain(|id| valid_agent_ids.contains(id));
        if dept.id == ASSISTANT_DEPARTMENT_ID
            && !data.assistant_department_agent_id.trim().is_empty()
            && valid_agent_ids.contains(&data.assistant_department_agent_id)
            && !dept
                .agent_ids
                .iter()
                .any(|id| id.trim() == data.assistant_department_agent_id)
        {
            dept.agent_ids.push(data.assistant_department_agent_id.clone());
        }
        if dept.agent_ids != original_agent_ids {
            config_changed = true;
            dept.updated_at = now_iso();
        }
    }
    if config_changed {
        let changed_departments = changed_department_ids(&previous_runtime_config, &runtime_config);
        validate_department_names_unique(&runtime_config)?;
        normalize_app_config(&mut runtime_config);
        config = persist_departments_by_source(&state, &runtime_config)?;
        if !changed_departments.is_empty() {
            mark_prompt_cache_rebuild_for_system_sources_by_departments(
                state,
                &changed_departments,
            );
        }
        let runtime_config = runtime_config_with_private_organization(&state, &config, &data)?;
        let _ = app.emit("easy-call:config-updated", &runtime_config);
        broadcast_sidebar_department_changed();
    }
    broadcast_sidebar_persona_changed();
    let runtime_agents = runtime_agents_with_private_organization(&state, &config, &data)?;
    Ok(runtime_agents)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportAgentMemoriesInput {
    pub(crate) agent_id: String,
    pub(crate) memories: Vec<MemoryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportAgentMemoriesResult {
    pub(crate) imported_count: usize,
    pub(crate) created_count: usize,
    pub(crate) merged_count: usize,
    pub(crate) total_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentPrivateMemoryCountInput {
    pub(crate) agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentPrivateMemoryCountResult {
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetAgentPrivateMemoryEnabledInput {
    pub(crate) agent_id: String,
    pub(crate) enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetAgentPrivateMemoryEnabledResult {
    pub(crate) agent_id: String,
    pub(crate) enabled: bool,
    pub(crate) exported_count: usize,
    pub(crate) deleted_count: usize,
    pub(crate) export_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetAgentMemoryRecallModeInput {
    pub(crate) agent_id: String,
    pub(crate) mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetAgentMemoryRecallModeResult {
    pub(crate) agent_id: String,
    pub(crate) mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportAgentPrivateMemoriesInput {
    pub(crate) agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportAgentPrivateMemoriesResult {
    pub(crate) count: usize,
    pub(crate) path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DisableAgentPrivateMemoryInput {
    pub(crate) agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DisableAgentPrivateMemoryResult {
    pub(crate) agent_id: String,
    pub(crate) enabled: bool,
    pub(crate) deleted_count: usize,
}

pub(crate) fn get_agent_private_memory_count_inner(
    input: AgentPrivateMemoryCountInput,
    state: &AppState,
) -> Result<AgentPrivateMemoryCountResult, String> {
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let config = read_config(&state.config_path)?;
    let agents = state_read_agents_cached(&state)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    Ok(AgentPrivateMemoryCountResult {
        count: memory_store_count_private_memories_by_agent(&state.data_path, agent_id)?,
    })
}

pub(crate) fn set_agent_memory_recall_mode_inner(
    input: SetAgentMemoryRecallModeInput,
    state: &AppState,
) -> Result<SetAgentMemoryRecallModeResult, String> {
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

    let mut agents = state_read_agents_cached(&state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|a| a.id == agent_id && !a.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;
    if normalize_agent_memory_recall_mode(&agents[agent_idx].memory_recall_mode) != mode {
        agents[agent_idx].memory_recall_mode = mode.clone();
        state_write_agents_cached(&state, &agents)?;
        runtime_log_info(format!(
            "[记忆] 完成，任务=切换人格回忆模式，agent_id={}，mode={}",
            agent_id, mode
        ));
    }
    Ok(SetAgentMemoryRecallModeResult {
        agent_id: agent_id.to_string(),
        mode,
    })
}

pub(crate) fn set_agent_private_memory_enabled_inner(
    input: SetAgentPrivateMemoryEnabledInput,
    state: &AppState,
) -> Result<SetAgentPrivateMemoryEnabledResult, String> {
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(&state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|a| a.id == agent_id && !a.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;

    let current = agents[agent_idx].private_memory_enabled;
    if current == input.enabled {
        return Ok(SetAgentPrivateMemoryEnabledResult {
            agent_id: agent_id.to_string(),
            enabled: current,
            exported_count: 0,
            deleted_count: 0,
            export_path: None,
        });
    }

    if input.enabled {
        agents[agent_idx].private_memory_enabled = true;
        state_write_agents_cached(&state, &agents)?;
        return Ok(SetAgentPrivateMemoryEnabledResult {
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
    state_write_agents_cached(&state, &agents)?;

    Ok(SetAgentPrivateMemoryEnabledResult {
        agent_id: agent_id.to_string(),
        enabled: false,
        exported_count: export.count,
        deleted_count: deleted,
        export_path: Some(export.path),
    })
}

pub(crate) fn export_agent_private_memories_inner(
    input: ExportAgentPrivateMemoriesInput,
    state: &AppState,
) -> Result<ExportAgentPrivateMemoriesResult, String> {
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }
    let config = read_config(&state.config_path)?;
    let agents = state_read_agents_cached(&state)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    let export = memory_store_export_agent_private_memories(&state.data_path, agent_id)?;
    Ok(ExportAgentPrivateMemoriesResult {
        count: export.count,
        path: export.path,
    })
}

pub(crate) fn disable_agent_private_memory_inner(
    input: DisableAgentPrivateMemoryInput,
    state: &AppState,
) -> Result<DisableAgentPrivateMemoryResult, String> {
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(&state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }

    let agent_idx = agents
        .iter()
        .position(|a| a.id == agent_id && !a.is_built_in_user)
        .ok_or_else(|| format!("Agent '{}' not found.", agent_id))?;

    if !agents[agent_idx].private_memory_enabled {
        return Ok(DisableAgentPrivateMemoryResult {
            agent_id: agent_id.to_string(),
            enabled: false,
            deleted_count: 0,
        });
    }

    let deleted = memory_store_delete_memories_by_owner_agent_id(&state.data_path, agent_id)?;
    agents[agent_idx].private_memory_enabled = false;
    state_write_agents_cached(&state, &agents)?;

    Ok(DisableAgentPrivateMemoryResult {
        agent_id: agent_id.to_string(),
        enabled: false,
        deleted_count: deleted,
    })
}







pub(crate) fn import_agent_memories_inner(
    input: ImportAgentMemoriesInput,
    state: &AppState,
) -> Result<ImportAgentMemoriesResult, String> {
    let agent_id = input.agent_id.trim();
    if agent_id.is_empty() {
        return Err("agentId is required".to_string());
    }

    let data = state_read_agents_runtime_snapshot(state)?;
    let base_config = read_config(&state.config_path)?;
    let (private_agent_ids, _) =
        runtime_private_organization_ids(&state.data_path, &base_config, &data.agents)?;
    if private_agent_ids.contains(agent_id) {
        return Err(private_agent_operation_error(agent_id));
    }
    if !data
        .agents
        .iter()
        .any(|a| a.id == agent_id && !a.is_built_in_user)
    {
        return Err(format!("Agent '{}' not found.", agent_id));
    }

    let stats = memory_store_import_memories_for_agent(&state.data_path, &input.memories, agent_id)?;
    Ok(ImportAgentMemoriesResult {
        imported_count: stats.imported_count,
        created_count: stats.created_count,
        merged_count: stats.merged_count,
        total_count: stats.total_count,
    })
}


pub(crate) fn load_chat_settings_inner(state: &AppState) -> Result<ChatSettings, String> {
    let config = read_config(&state.config_path)?;
    let mut data = state_read_agents_runtime_snapshot(&state)?;
    let assistant_agent_id = assistant_department_agent_id(&config).unwrap_or_else(default_assistant_department_agent_id);
    let runtime_changed = if data.assistant_department_agent_id != assistant_agent_id {
        data.assistant_department_agent_id = assistant_agent_id.clone();
        true
    } else {
        false
    };
    if runtime_changed {
        state_write_runtime_state_cached(&state, &build_runtime_state_file(&data))?;
    }
    let runtime_snapshot =
        build_runtime_organization_snapshot_from_parts(&state.data_path, &config, &data.agents)?;
    let mut runtime_data = data.clone();
    runtime_data.agents = runtime_snapshot.agents;

    Ok(ChatSettings {
        assistant_department_agent_id: data.assistant_department_agent_id.clone(),
        user_alias: user_persona_name(&runtime_data),
        response_style_id: data.response_style_id.clone(),
        pdf_read_mode: data.pdf_read_mode.clone(),
        background_voice_screenshot_keywords: data.background_voice_screenshot_keywords.clone(),
        background_voice_screenshot_mode: data.background_voice_screenshot_mode.clone(),
        instruction_presets: data.instruction_presets.clone(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatSettingsPatch {
    #[serde(default)]
    pub(crate) assistant_department_agent_id: Option<String>,
    #[serde(default)]
    pub(crate) user_alias: Option<String>,
    #[serde(default)]
    pub(crate) response_style_id: Option<String>,
    #[serde(default)]
    pub(crate) pdf_read_mode: Option<String>,
    #[serde(default)]
    pub(crate) background_voice_screenshot_keywords: Option<String>,
    #[serde(default)]
    pub(crate) background_voice_screenshot_mode: Option<String>,
    #[serde(default)]
    pub(crate) instruction_presets: Option<Vec<PromptCommandPreset>>,
}

pub(crate) fn build_chat_settings_payload(state: &AppState, data: &AppData, config: &AppConfig) -> Result<ChatSettings, String> {
    let runtime_snapshot =
        build_runtime_organization_snapshot_from_parts(&state.data_path, config, &data.agents)?;
    let mut runtime_data = data.clone();
    runtime_data.agents = runtime_snapshot.agents;
    Ok(ChatSettings {
        assistant_department_agent_id: data.assistant_department_agent_id.clone(),
        user_alias: user_persona_name(&runtime_data),
        response_style_id: data.response_style_id.clone(),
        pdf_read_mode: data.pdf_read_mode.clone(),
        background_voice_screenshot_keywords: data.background_voice_screenshot_keywords.clone(),
        background_voice_screenshot_mode: data.background_voice_screenshot_mode.clone(),
        instruction_presets: data.instruction_presets.clone(),
    })
}

pub(crate) fn apply_chat_settings_patch(
    state: &AppState,
    agents: &mut Vec<AgentProfile>,
    runtime: &mut RuntimeStateFile,
    config: &AppConfig,
    input: ChatSettingsPatch,
) -> Result<ChatSettings, String> {
    let mut agents_changed = false;
    let mut runtime_changed = false;
    if let Some(agent_id) = input.assistant_department_agent_id {
        let fallback = runtime.assistant_department_agent_id.clone();
        let target_agent_id = assistant_department_agent_id(config).unwrap_or_else(|| {
            let trimmed = agent_id.trim();
            if trimmed.is_empty() {
                fallback.clone()
            } else {
                trimmed.to_string()
            }
        });
        let mut runtime_data = AppData::default();
        runtime_data.agents = agents.clone();
        apply_runtime_state_to_app_data(&mut runtime_data, runtime);
        let runtime_snapshot = build_runtime_organization_snapshot_from_parts(
            &state.data_path,
            config,
            &runtime_data.agents,
        )?;
        if !runtime_snapshot
            .agents
            .iter()
            .any(|a| a.id == target_agent_id && !a.is_built_in_user)
        {
            return Err("Selected agent not found.".to_string());
        }
        if runtime.assistant_department_agent_id != target_agent_id {
            runtime.assistant_department_agent_id = target_agent_id;
            runtime_changed = true;
        }
    }
    if let Some(response_style_id) = input.response_style_id {
        let next = normalize_response_style_id(&response_style_id);
        if runtime.response_style_id != next {
            runtime.response_style_id = next;
            runtime_changed = true;
        }
    }
    if let Some(pdf_read_mode) = input.pdf_read_mode {
        let next = normalize_pdf_read_mode(&pdf_read_mode);
        if runtime.pdf_read_mode != next {
            runtime.pdf_read_mode = next;
            runtime_changed = true;
        }
    }
    if let Some(background_voice_screenshot_keywords) = input.background_voice_screenshot_keywords {
        let next = background_voice_screenshot_keywords.trim().to_string();
        if runtime.background_voice_screenshot_keywords != next {
            runtime.background_voice_screenshot_keywords = next;
            runtime_changed = true;
        }
    }
    if let Some(background_voice_screenshot_mode) = input.background_voice_screenshot_mode {
        let next = normalize_background_voice_screenshot_mode(&background_voice_screenshot_mode);
        if runtime.background_voice_screenshot_mode != next {
            runtime.background_voice_screenshot_mode = next;
            runtime_changed = true;
        }
    }
    if let Some(instruction_presets) = input.instruction_presets {
        let next = instruction_presets
            .into_iter()
            .map(|item| PromptCommandPreset {
                id: item.id.trim().to_string(),
                name: item.name.trim().to_string(),
                prompt: item.prompt.trim().to_string(),
            })
            .filter(|item| !item.id.is_empty() && !item.name.is_empty() && !item.prompt.is_empty())
            .collect::<Vec<_>>();
        if runtime.instruction_presets != next {
            runtime.instruction_presets = next;
            runtime_changed = true;
        }
    }
    if let Some(user_alias) = input.user_alias {
        let trimmed = user_alias.trim();
        if !trimmed.is_empty() {
            if let Some(user_persona) = agents.iter_mut().find(|a| a.id == USER_PERSONA_ID) {
                user_persona.name = trimmed.to_string();
                user_persona.updated_at = now_iso();
                agents_changed = true;
            }
        }
    }
    if agents_changed {
        state_write_agents_cached(state, agents)?;
    }
    if runtime_changed {
        state_write_runtime_state_cached(state, runtime)?;
    }
    let mut data = AppData::default();
    data.agents = agents.clone();
    apply_runtime_state_to_app_data(&mut data, runtime);
    build_chat_settings_payload(state, &data, config)
}



pub(crate) fn patch_chat_settings_inner(
    input: ChatSettingsPatch,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<ChatSettings, String> {
    let mut data = state_read_agents_runtime_snapshot(&state)?;
    let config = read_config(&state.config_path)?;
    let mut runtime = build_runtime_state_file(&data);
    let payload = apply_chat_settings_patch(&state, &mut data.agents, &mut runtime, &config, input)?;

    let _ = app.emit("easy-call:chat-settings-updated", &payload);
    broadcast_sidebar_persona_changed();

    Ok(payload)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveAgentAvatarInput {
    pub(crate) agent_id: String,
    pub(crate) mime: String,
    pub(crate) bytes_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ClearAgentAvatarInput {
    pub(crate) agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AvatarDataPathInput {
    pub(crate) path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SyncTrayIconInput {
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AvatarMeta {
    pub(crate) path: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AvatarDataUrlOutput {
    pub(crate) data_url: String,
}

pub(crate) fn avatar_storage_dir(state: &AppState) -> Result<PathBuf, String> {
    Ok(app_root_from_data_path(&state.data_path).join("avatars"))
}

pub(crate) fn sanitize_avatar_key(value: &str) -> String {
    let trimmed = value.trim();
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    let normalized = out.trim_matches('_');
    if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized.to_string()
    }
}

pub(crate) fn normalize_avatar_bytes_to_webp(raw: &[u8]) -> Result<Vec<u8>, String> {
    let image = image::load_from_memory(raw)
        .map_err(|err| format!("Decode avatar image failed: {err}"))?;
    let resized = image.resize_to_fill(128, 128, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::<u8>::new();
    let mut cursor = Cursor::new(&mut out);
    resized
        .write_to(&mut cursor, ImageFormat::WebP)
        .map_err(|err| format!("Encode avatar as webp failed: {err}"))?;
    Ok(out)
}


pub(crate) fn save_agent_avatar_inner(
    input: SaveAgentAvatarInput,
    state: &AppState,
) -> Result<AvatarMeta, String> {
    if input.agent_id.trim().is_empty() {
        return Err("agentId is required".to_string());
    }
    if input.bytes_base64.trim().is_empty() {
        return Err("avatar payload is empty".to_string());
    }
    if !input.mime.trim().starts_with("image/") {
        return Err("avatar mime must be image/*".to_string());
    }

    let mut agents = state_read_agents_cached(&state)?;
    let base_config = read_config(&state.config_path)?;
    let mut runtime_data = state_read_agents_runtime_snapshot(&state)?;
    runtime_data.agents = agents.clone();
    let runtime_agents = runtime_agents_with_private_organization(&state, &base_config, &runtime_data)?;
    let target = runtime_agents
        .iter()
        .find(|a| a.id == input.agent_id)
        .ok_or_else(|| "Agent not found".to_string())?;

    let raw = B64
        .decode(input.bytes_base64.trim())
        .map_err(|err| format!("Decode avatar base64 failed: {err}"))?;
    let webp = normalize_avatar_bytes_to_webp(&raw)?;

    let dir = avatar_storage_dir(&state)?;
    fs::create_dir_all(&dir).map_err(|err| format!("Create avatar directory failed: {err}"))?;
    let safe_id = sanitize_avatar_key(&input.agent_id);
    let path = dir.join(format!("agent-{safe_id}.webp"));
    fs::write(&path, webp).map_err(|err| format!("Write avatar file failed: {err}"))?;

    let now = now_iso();
    let avatar_path = path.to_string_lossy().to_string();
    if is_private_workspace_source(&target.source) {
        let mut next_runtime_agents = runtime_agents.clone();
        let idx = next_runtime_agents
            .iter()
            .position(|a| a.id == input.agent_id)
            .ok_or_else(|| "Agent not found".to_string())?;
        next_runtime_agents[idx].avatar_path = Some(avatar_path.clone());
        next_runtime_agents[idx].avatar_updated_at = Some(now.clone());
        next_runtime_agents[idx].updated_at = now.clone();
        sync_private_agents_to_workspace(&state.data_path, &base_config, &next_runtime_agents)?;
    } else {
        let idx = agents
            .iter()
            .position(|a| a.id == input.agent_id)
            .ok_or_else(|| "Agent not found".to_string())?;
        agents[idx].avatar_path = Some(avatar_path.clone());
        agents[idx].avatar_updated_at = Some(now.clone());
        agents[idx].updated_at = now.clone();
        state_write_agents_cached(&state, &agents)?;
    }

    Ok(AvatarMeta {
        path: avatar_path,
        updated_at: now,
    })
}


pub(crate) fn clear_agent_avatar_inner(
    input: ClearAgentAvatarInput,
    state: &AppState,
) -> Result<(), String> {
    if input.agent_id.trim().is_empty() {
        return Err("agentId is required".to_string());
    }

    let mut agents = state_read_agents_cached(&state)?;
    let base_config = read_config(&state.config_path)?;
    let mut runtime_data = state_read_agents_runtime_snapshot(&state)?;
    runtime_data.agents = agents.clone();
    let runtime_agents = runtime_agents_with_private_organization(&state, &base_config, &runtime_data)?;
    let target = runtime_agents
        .iter()
        .find(|a| a.id == input.agent_id)
        .ok_or_else(|| "Agent not found".to_string())?;

    if let Some(path) = target.avatar_path.as_deref() {
        let p = PathBuf::from(path);
        if p.exists() {
            let _ = fs::remove_file(p);
        }
    }
    let now = now_iso();
    if is_private_workspace_source(&target.source) {
        let mut next_runtime_agents = runtime_agents.clone();
        let idx = next_runtime_agents
            .iter()
            .position(|a| a.id == input.agent_id)
            .ok_or_else(|| "Agent not found".to_string())?;
        next_runtime_agents[idx].avatar_path = None;
        next_runtime_agents[idx].avatar_updated_at = None;
        next_runtime_agents[idx].updated_at = now;
        sync_private_agents_to_workspace(&state.data_path, &base_config, &next_runtime_agents)?;
    } else {
        let idx = agents
            .iter()
            .position(|a| a.id == input.agent_id)
            .ok_or_else(|| "Agent not found".to_string())?;
        agents[idx].avatar_path = None;
        agents[idx].avatar_updated_at = None;
        agents[idx].updated_at = now;
        state_write_agents_cached(&state, &agents)?;
    }
    Ok(())
}


pub(crate) fn read_avatar_data_url_inner(
    input: AvatarDataPathInput,
    state: &AppState,
) -> Result<AvatarDataUrlOutput, String> {
    if input.path.trim().is_empty() {
        // 未设置头像时，返回内置品牌图标作为默认头像。
        let bytes: &[u8] = include_bytes!("../../../../../icons/128x128.png");
        let base64 = B64.encode(bytes);
        return Ok(AvatarDataUrlOutput {
            data_url: format!("data:image/png;base64,{base64}"),
        });
    }
    let avatars_dir = avatar_storage_dir(&state)?;
    let root = fs::canonicalize(&avatars_dir).map_err(|err| {
        format!(
            "Resolve avatar root failed ({}): {err}",
            avatars_dir.to_string_lossy()
        )
    })?;
    let target = fs::canonicalize(input.path.trim()).map_err(|err| {
        format!("Resolve avatar path failed ({}): {err}", input.path.trim())
    })?;
    if !target.starts_with(&root) {
        return Err("Avatar path is outside allowed avatar directory.".to_string());
    }
    let metadata = fs::metadata(&target)
        .map_err(|err| format!("Read avatar metadata failed: {err}"))?;
    if !metadata.is_file() {
        return Err("Avatar path must be a regular file.".to_string());
    }
    let ext = target
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "webp" => "image/webp",
        "png" => "image/png",
        _ => return Err("Avatar file type is not allowed (only .webp/.png).".to_string()),
    };
    let bytes = fs::read(&target)
        .map_err(|err| format!("Read avatar file failed: {err}"))?;
    let base64 = B64.encode(bytes);
    Ok(AvatarDataUrlOutput {
        data_url: format!("data:{mime};base64,{base64}"),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatImageDataUrlInput {
    pub(crate) media_ref: String,
    pub(crate) mime: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ChatImageDataUrlOutput {
    pub(crate) data_url: String,
}


pub(crate) fn read_chat_image_data_url_inner(
    input: ChatImageDataUrlInput,
    state: &AppState,
) -> Result<ChatImageDataUrlOutput, String> {
    let media_ref = input.media_ref.trim();
    if media_ref.is_empty() {
        return Ok(ChatImageDataUrlOutput {
            data_url: String::new(),
        });
    }
    if stored_binary_ref_from_marker(media_ref).is_none() {
        return Err("Chat image mediaRef is invalid.".to_string());
    }
    let mime = input.mime.trim().to_ascii_lowercase();
    if !mime.starts_with("image/") {
        return Err("Chat image mime is invalid.".to_string());
    }
    let base64 = resolve_stored_binary_base64(&state.data_path, media_ref)?;
    Ok(ChatImageDataUrlOutput {
        data_url: format!("data:{mime};base64,{base64}"),
    })
}


#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConversationApiSettingsPatch {
    #[serde(default)]
    pub(crate) assistant_department_api_config_id: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_patch")]
    pub(crate) vision_api_config_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_patch")]
    pub(crate) tool_review_api_config_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_nullable_string_patch")]
    pub(crate) stt_api_config_id: Option<Option<String>>,
    #[serde(default)]
    pub(crate) stt_auto_send: Option<bool>,
}

pub(crate) fn deserialize_nullable_string_patch<'de, D>(
    deserializer: D,
) -> Result<Option<Option<String>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = Option::<serde_json::Value>::deserialize(deserializer)?;
    match value {
        None | Some(serde_json::Value::Null) => Ok(Some(None)),
        Some(serde_json::Value::String(text)) => Ok(Some(Some(text))),
        Some(other) => Err(serde::de::Error::custom(format!(
            "expected string or null, got {other}"
        ))),
    }
}

#[cfg(test)]
mod conversation_api_settings_patch_tests {
    use super::*;

    #[test]
    fn conversation_api_settings_patch_distinguishes_missing_null_and_string() {
        let missing: ConversationApiSettingsPatch =
            serde_json::from_value(serde_json::json!({})).unwrap();
        assert_eq!(missing.vision_api_config_id, None);
        assert_eq!(missing.tool_review_api_config_id, None);
        assert_eq!(missing.stt_api_config_id, None);

        let cleared: ConversationApiSettingsPatch = serde_json::from_value(serde_json::json!({
            "visionApiConfigId": null,
            "toolReviewApiConfigId": null,
            "sttApiConfigId": null
        }))
        .unwrap();
        assert_eq!(cleared.vision_api_config_id, Some(None));
        assert_eq!(cleared.tool_review_api_config_id, Some(None));
        assert_eq!(cleared.stt_api_config_id, Some(None));

        let selected: ConversationApiSettingsPatch = serde_json::from_value(serde_json::json!({
            "visionApiConfigId": "vision-api",
            "toolReviewApiConfigId": "review-api",
            "sttApiConfigId": "stt-api"
        }))
        .unwrap();
        assert_eq!(
            selected.vision_api_config_id,
            Some(Some("vision-api".to_string()))
        );
        assert_eq!(
            selected.tool_review_api_config_id,
            Some(Some("review-api".to_string()))
        );
        assert_eq!(
            selected.stt_api_config_id,
            Some(Some("stt-api".to_string()))
        );
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SetDepartmentPrimaryApiConfigInput {
    pub(crate) department_id: String,
    pub(crate) api_config_id: String,
}

pub(crate) fn build_conversation_api_settings_payload(config: &AppConfig) -> ConversationApiSettings {
    ConversationApiSettings {
        assistant_department_api_config_id: config.assistant_department_api_config_id.clone(),
        vision_api_config_id: config.vision_api_config_id.clone(),
        tool_review_api_config_id: config.tool_review_api_config_id.clone(),
        stt_api_config_id: config.stt_api_config_id.clone(),
        stt_auto_send: config.stt_auto_send,
    }
}

pub(crate) fn apply_conversation_api_settings_patch(config: &mut AppConfig, input: ConversationApiSettingsPatch) {
    if let Some(assistant_department_api_config_id) = input.assistant_department_api_config_id {
        config.assistant_department_api_config_id = assistant_department_api_config_id;
    }
    if let Some(vision_api_config_id) = input.vision_api_config_id {
        config.vision_api_config_id = vision_api_config_id;
    }
    if let Some(tool_review_api_config_id) = input.tool_review_api_config_id {
        config.tool_review_api_config_id = tool_review_api_config_id;
    }
    if let Some(stt_api_config_id) = input.stt_api_config_id {
        config.stt_api_config_id = stt_api_config_id;
    }
    if let Some(stt_auto_send) = input.stt_auto_send {
        config.stt_auto_send = stt_auto_send;
    }
}



pub(crate) fn patch_conversation_api_settings_inner(
    input: ConversationApiSettingsPatch,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<ConversationApiSettings, String> {
    let mut config = state_read_config_cached(&state)?;
    apply_conversation_api_settings_patch(&mut config, input);
    normalize_app_config(&mut config);
    state_write_config_cached(&state, &config)?;

    let payload = build_conversation_api_settings_payload(&config);

    let _ = app.emit("easy-call:conversation-api-updated", &payload);
    broadcast_sidebar_provider_changed();

    Ok(payload)
}


pub(crate) fn set_department_primary_api_config_inner(
    input: SetDepartmentPrimaryApiConfigInput,
    app: &NativeAppHandle,
    state: &AppState,
) -> Result<AppConfig, String> {
    let department_id = input.department_id.trim();
    if department_id.is_empty() {
        return Err("Department ID is required.".to_string());
    }
    let api_config_id = input.api_config_id.trim();
    if api_config_id.is_empty() {
        return Err("API config ID is required.".to_string());
    }

    let mut config = state_read_config_cached(state)?;
    let selected_api = config
        .api_configs
        .iter()
        .find(|item| item.id.trim() == api_config_id)
        .ok_or_else(|| format!("API config '{api_config_id}' not found."))?;
    if !selected_api.enable_text {
        return Err(format!("API config '{api_config_id}' does not support chat text."));
    }

    {
        let Some(target_department) = config
            .departments
            .iter_mut()
            .find(|item| item.id.trim() == department_id)
        else {
            return Err(format!("Department '{department_id}' not found."));
        };

        let mut next_ids = department_api_config_ids(target_department);
        if next_ids.first().map(|item| item.trim()) == Some(api_config_id) {
            // 保持当前顺序，只同步全局选中模型即可。
        } else {
            next_ids.retain(|item| !item.trim().eq_ignore_ascii_case(api_config_id));
            if next_ids.is_empty() {
                next_ids.push(api_config_id.to_string());
            } else {
                next_ids[0] = api_config_id.to_string();
            }
        }

        let mut seen = std::collections::HashSet::<String>::new();
        target_department.api_config_ids = next_ids
            .into_iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .filter(|item| seen.insert(item.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        target_department.api_config_id = target_department
            .api_config_ids
            .first()
            .cloned()
            .unwrap_or_default();
        target_department.updated_at = now_iso();

    }
    config.selected_api_config_id = api_config_id.to_string();

    state_write_config_cached(state, &config)?;
    let data = state_read_agents_runtime_snapshot(state)?;
    let runtime_config = runtime_config_with_private_organization(state, &config, &data)?;

    let _ = app.emit("easy-call:config-updated", &runtime_config);
    broadcast_sidebar_department_changed();
    broadcast_sidebar_provider_changed();

    Ok(runtime_config)
}
