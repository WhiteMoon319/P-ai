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
