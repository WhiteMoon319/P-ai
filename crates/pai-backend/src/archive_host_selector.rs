//! 归档宿主选择（纯逻辑，无平台依赖）。

use crate::core::domain::types_chat::{AgentProfile, Conversation};
use crate::core::domain::types_config::{AppConfig, DepartmentConfig};
use crate::core::domain::types_storage::department_by_id;

/// 归档记忆归属人格解析（从 src-tauri archive_host_selector.rs 迁入）。
pub fn resolve_archive_owner_agent_id(
    config: &AppConfig,
    agents: &[AgentProfile],
    source: &Conversation,
) -> Result<String, String> {
    let department_id = source.department_id.trim();
    if department_id.is_empty() {
        return Err(format!(
            "会话缺少归属部门，无法确定归档记忆归属人格: conversation_id={}",
            source.id
        ));
    }

    let department = department_by_id(config, department_id).ok_or_else(|| {
        format!(
            "会话归属部门不存在，无法确定归档记忆归属人格: conversation_id={}, department_id={}",
            source.id, department_id
        )
    })?;

    let owner_agent_id = source.agent_id.trim();
    let owner_agent_id = if owner_agent_id.is_empty() {
        first_available_department_agent(department, agents)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| {
                format!(
                    "会话归属部门没有可用人格，无法确定归档记忆归属人格: conversation_id={}, department_id={}",
                    source.id, department_id
                )
            })?
    } else {
        if available_non_user_agent(agents, owner_agent_id).is_none() {
            return Err(format!(
                "归档记忆归属人格不存在: conversation_id={}, department_id={}, agent_id={}",
                source.id, department_id, owner_agent_id
            ));
        }
        owner_agent_id.to_string()
    };

    Ok(owner_agent_id)
}

/// 非用户人格查找（从 src-tauri conversation.rs 迁入）。
pub fn available_non_user_agent<'a>(
    agents: &'a [AgentProfile],
    agent_id: &str,
) -> Option<&'a AgentProfile> {
    let agent_id = agent_id.trim();
    if agent_id.is_empty() {
        return None;
    }
    agents
        .iter()
        .find(|agent| agent.id == agent_id && !agent.is_built_in_user)
}

/// 部门内第一个可用非用户人格（从 src-tauri conversation.rs 迁入）。
pub fn first_available_department_agent<'a>(
    department: &DepartmentConfig,
    agents: &'a [AgentProfile],
) -> Option<&'a AgentProfile> {
    department
        .agent_ids
        .iter()
        .map(|id| id.trim())
        .find_map(|agent_id| available_non_user_agent(agents, agent_id))
}
