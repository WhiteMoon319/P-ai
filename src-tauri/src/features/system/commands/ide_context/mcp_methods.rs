pub(crate) fn ide_chat_mcp_list_servers_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(mcp_list_servers_inner(state)?)
}

pub(crate) fn ide_chat_mcp_validate_definition_for_web_settings(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpDefinitionValidateInput>(params, "input")?;
    ide_chat_serialize(mcp_validate_definition_inner(input)?)
}

pub(crate) async fn ide_chat_mcp_fix_definition_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpFixDefinitionInput>(params, "input")?;
    ide_chat_serialize(mcp_fix_definition_inner(input, state).await?)
}

pub(crate) fn ide_chat_mcp_save_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerInput>(params, "input")?;
    ide_chat_serialize(mcp_save_server_inner(input, state)?)
}

pub(crate) async fn ide_chat_mcp_remove_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    ide_chat_serialize(mcp_remove_server_inner(input, state).await?)
}

pub(crate) async fn ide_chat_mcp_list_server_tools_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    ide_chat_serialize(mcp_list_server_tools_inner(input, state).await?)
}

pub(crate) fn ide_chat_mcp_list_server_tools_cached_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    ide_chat_serialize(mcp_list_server_tools_cached_inner(input, state)?)
}

pub(crate) async fn ide_chat_mcp_deploy_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    ide_chat_serialize(mcp_deploy_server_inner(input, state).await?)
}

pub(crate) async fn ide_chat_mcp_undeploy_server_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpServerIdInput>(params, "input")?;
    ide_chat_serialize(mcp_undeploy_server_inner(input, state).await?)
}

pub(crate) fn ide_chat_mcp_set_tool_enabled_for_web_settings(
    state: &AppState,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_param_field::<McpSetToolEnabledInput>(params, "input")?;
    ide_chat_serialize(mcp_set_tool_enabled_inner(input, state)?)
}

pub(crate) async fn ide_chat_mcp_refresh_mcp_and_skills_for_web_settings(
    state: &AppState,
) -> Result<Value, String> {
    ide_chat_serialize(features_skill::commands::mcp_refresh_mcp_and_skills_inner(state).await?)
}

pub(crate) fn ide_chat_mcp_list_skills_for_web_settings(state: &AppState) -> Result<Value, String> {
    ide_chat_serialize(features_skill::commands::mcp_list_skills_inner(state)?)
}
