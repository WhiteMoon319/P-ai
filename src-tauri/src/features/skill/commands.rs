use super::*;

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub(crate) async fn mcp_refresh_mcp_and_skills(
    state: State<'_, AppState>,
) -> Result<RefreshMcpAndSkillsResult, String> {
    mcp_refresh_mcp_and_skills_inner(state.inner()).await
}

pub(crate) async fn mcp_refresh_mcp_and_skills_inner(
    state: &AppState,
) -> Result<RefreshMcpAndSkillsResult, String> {
    reload_workspace(state).await
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub(crate) fn mcp_list_skills(state: State<'_, AppState>) -> Result<SkillListResult, String> {
    mcp_list_skills_inner(state.inner())
}

pub(crate) fn mcp_list_skills_inner(state: &AppState) -> Result<SkillListResult, String> {
    let (skills, errors) = load_workspace_skill_summaries_with_errors(state)?;
    let _ = update_hidden_skill_snapshot_cache(state, &skills, None);
    Ok(SkillListResult { skills, errors })
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
pub(crate) fn skill_open_workspace_dir(state: State<'_, AppState>) -> Result<String, String> {
    open_skills_workspace_dir(&state)
}

