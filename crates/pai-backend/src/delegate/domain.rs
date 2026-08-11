pub const DELEGATE_DB_FILE_NAME: &str = "delegate_store.db";
pub const DELEGATE_STATUS_RUNNING: &str = "running";
pub const DELEGATE_STATUS_DELIVERED: &str = "delivered";
pub const DELEGATE_STATUS_COMPLETED: &str = "completed";
pub const DELEGATE_STATUS_FAILED: &str = "failed";

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DelegateEntry {
    pub delegate_id: String,
    pub kind: String,
    pub conversation_id: String,
    #[serde(default)]
    pub parent_delegate_id: Option<String>,
    pub source_department_id: String,
    pub target_department_id: String,
    pub source_agent_id: String,
    pub target_agent_id: String,
    pub title: String,
    #[serde(default)]
    pub why: String,
    #[serde(default)]
    pub goal: String,
    #[serde(default)]
    pub todo: String,
    pub notify_assistant_when_done: bool,
    pub call_stack: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub delivered_at: Option<String>,
    #[serde(default)]
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DelegateCreateInput {
    pub kind: String,
    pub conversation_id: String,
    pub parent_delegate_id: Option<String>,
    pub source_department_id: String,
    pub target_department_id: String,
    pub source_agent_id: String,
    pub target_agent_id: String,
    pub title: String,
    pub why: String,
    pub goal: String,
    pub todo: String,
    pub notify_assistant_when_done: bool,
    pub call_stack: Vec<String>,
}
