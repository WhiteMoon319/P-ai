#[derive(Debug, Clone)]
pub(crate) struct RuntimeOrganizationSnapshot {
    pub(crate) config: AppConfig,
    pub(crate) agents: Vec<AgentProfile>,
    pub(crate) departments_by_id: std::collections::HashMap<String, DepartmentConfig>,
    pub(crate) department_ids_by_agent: std::collections::HashMap<String, String>,
}

pub(crate) fn normalize_runtime_organization_department_children(config: &mut AppConfig) {
    let valid_department_ids = config
        .departments
        .iter()
        .map(|department| department.id.trim().to_string())
        .filter(|department_id| !department_id.is_empty())
        .collect::<std::collections::HashSet<_>>();
    for department in &mut config.departments {
        department.child_department_ids = normalize_department_child_ids(
            &department.child_department_ids,
            &department.id,
        )
        .into_iter()
        .filter(|child_id| valid_department_ids.contains(child_id))
        .collect::<Vec<_>>();
    }
    let removed = remove_cyclic_department_child_ids(&mut config.departments);
    if !removed.is_empty() {
        let edges = removed
            .iter()
            .map(|(parent_id, child_id)| format!("{parent_id}->{child_id}"))
            .collect::<Vec<_>>()
            .join(", ");
        runtime_log_warn(format!(
            "[运行组织] 跳过成环部门关系: count={}, edges={}",
            removed.len(),
            edges
        ));
    }
}

pub(crate) fn build_runtime_organization_snapshot_from_parts(
    data_path: &Path,
    base_config: &AppConfig,
    base_agents: &[AgentProfile],
) -> Result<RuntimeOrganizationSnapshot, String> {
    let mut config = base_config.clone();
    normalize_app_config(&mut config);
    let mut runtime_data = AppData::default();
    runtime_data.agents = base_agents.to_vec();
    ensure_required_builtin_agents_in_list(&mut runtime_data.agents);
    merge_private_organization_into_runtime_data(data_path, &mut config, &mut runtime_data)?;
    normalize_runtime_organization_department_children(&mut config);

    let mut departments_by_id = std::collections::HashMap::<String, DepartmentConfig>::new();
    let mut department_ids_by_agent = std::collections::HashMap::<String, String>::new();
    for department in &config.departments {
        let department_id = department.id.trim();
        if department_id.is_empty() {
            continue;
        }
        departments_by_id.insert(department_id.to_string(), department.clone());
        for agent_id in &department.agent_ids {
            let agent_id = agent_id.trim();
            if agent_id.is_empty() {
                continue;
            }
            department_ids_by_agent
                .entry(agent_id.to_string())
                .or_insert_with(|| department_id.to_string());
        }
    }

    Ok(RuntimeOrganizationSnapshot {
        config,
        agents: runtime_data.agents,
        departments_by_id,
        department_ids_by_agent,
    })
}

pub(crate) fn load_runtime_organization_snapshot(
    state: &impl StateAccess,
) -> Result<RuntimeOrganizationSnapshot, String> {
    let config = state.read_config_cached()?;
    let agents = state.read_agents_cached()?;
    build_runtime_organization_snapshot_from_parts(state.data_path(), &config, &agents)
}

pub(crate) fn runtime_department_by_id<'a>(
    snapshot: &'a RuntimeOrganizationSnapshot,
    department_id: &str,
) -> Option<&'a DepartmentConfig> {
    let department_id = department_id.trim();
    if department_id.is_empty() {
        return None;
    }
    snapshot.departments_by_id.get(department_id)
}

pub(crate) fn runtime_department_for_agent<'a>(
    snapshot: &'a RuntimeOrganizationSnapshot,
    agent_id: &str,
) -> Option<&'a DepartmentConfig> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        return None;
    }
    snapshot
        .department_ids_by_agent
        .get(agent_id)
        .and_then(|department_id| runtime_department_by_id(snapshot, department_id))
        .or_else(|| {
            if agent_id == DEFAULT_AGENT_ID {
                runtime_department_by_id(snapshot, ASSISTANT_DEPARTMENT_ID)
            } else {
                None
            }
        })
}
