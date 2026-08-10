use super::*;


pub(crate) async fn mcp_refresh_mcp_and_skills_inner(
    state: &AppState,
) -> Result<RefreshMcpAndSkillsResult, String> {
    reload_workspace(state).await
}


pub(crate) fn mcp_list_skills_inner(state: &AppState) -> Result<SkillListResult, String> {
    let (skills, errors) = load_workspace_skill_summaries_with_errors(state)?;
    let _ = update_hidden_skill_snapshot_cache(state, &skills, None);
    Ok(SkillListResult { skills, errors })
}


