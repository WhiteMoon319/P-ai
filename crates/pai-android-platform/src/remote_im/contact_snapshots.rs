//! 远程 IM 联系人快照与仪表盘纯类型（阶段 5 迁入）。

use pai_backend::core::domain::runtime_types::RemoteImPresenceState;
use pai_backend::core::domain::types_storage::RemoteImContact;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImContactBindingSnapshot {
    pub bound_department_id: Option<String>,
    pub bound_agent_id: Option<String>,
    pub bound_conversation_id: Option<String>,
    pub route_mode: String,
}

pub fn remote_im_contact_binding_snapshot(contact: &RemoteImContact) -> RemoteImContactBindingSnapshot {
    RemoteImContactBindingSnapshot {
        bound_department_id: contact.bound_department_id.clone(),
        bound_agent_id: contact.bound_agent_id.clone(),
        bound_conversation_id: contact.bound_conversation_id.clone(),
        route_mode: contact.route_mode.clone(),
    }
}

pub fn remote_im_contact_binding_matches(
    contact: &RemoteImContact,
    snapshot: &RemoteImContactBindingSnapshot,
) -> bool {
    contact.bound_department_id == snapshot.bound_department_id
        && contact.bound_agent_id == snapshot.bound_agent_id
        && contact.bound_conversation_id == snapshot.bound_conversation_id
        && contact.route_mode == snapshot.route_mode
}

pub fn remote_im_apply_contact_binding_snapshot(
    contact: &mut RemoteImContact,
    snapshot: &RemoteImContactBindingSnapshot,
) {
    contact.bound_department_id = snapshot.bound_department_id.clone();
    contact.bound_agent_id = snapshot.bound_agent_id.clone();
    contact.bound_conversation_id = snapshot.bound_conversation_id.clone();
    contact.route_mode = snapshot.route_mode.clone();
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImContactDashboardSnapshot {
    pub contact_id: String,
    pub energy: f64,
    pub maximum_energy: f64,
    pub energy_percent: f64,
    pub energy_recovery_per_second: f64,
    pub presence: String,
    pub last_presence_at: Option<String>,
    pub watermark: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImContactDashboardInput {
    pub contact_id: String,
    #[serde(default)]
    pub known_watermark: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteImContactDashboardSyncResult {
    pub snapshot: RemoteImContactDashboardSnapshot,
    pub changed: bool,
}

pub fn remote_im_contact_dashboard_presence_label(state: RemoteImPresenceState) -> &'static str {
    match state {
        RemoteImPresenceState::Away => "away",
        RemoteImPresenceState::Present => "present",
    }
}
