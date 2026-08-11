pub(crate) const DELEGATE_DB_FILE_NAME: &str = "delegate_store.db";
pub(crate) const DELEGATE_STATUS_RUNNING: &str = "running";
pub(crate) const DELEGATE_STATUS_DELIVERED: &str = "delivered";
pub(crate) const DELEGATE_STATUS_COMPLETED: &str = "completed";
pub(crate) const DELEGATE_STATUS_FAILED: &str = "failed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DelegateEntry {
    pub(crate) delegate_id: String,
    pub(crate) kind: String,
    pub(crate) conversation_id: String,
    #[serde(default)]
    pub(crate) parent_delegate_id: Option<String>,
    pub(crate) source_department_id: String,
    pub(crate) target_department_id: String,
    pub(crate) source_agent_id: String,
    pub(crate) target_agent_id: String,
    pub(crate) title: String,
    #[serde(default)]
    pub(crate) why: String,
    #[serde(default)]
    pub(crate) goal: String,
    #[serde(default)]
    pub(crate) todo: String,
    pub(crate) notify_assistant_when_done: bool,
    pub(crate) call_stack: Vec<String>,
    pub(crate) status: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
    #[serde(default)]
    pub(crate) delivered_at: Option<String>,
    #[serde(default)]
    pub(crate) completed_at: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct DelegateCreateInput {
    pub(crate) kind: String,
    pub(crate) conversation_id: String,
    pub(crate) parent_delegate_id: Option<String>,
    pub(crate) source_department_id: String,
    pub(crate) target_department_id: String,
    pub(crate) source_agent_id: String,
    pub(crate) target_agent_id: String,
    pub(crate) title: String,
    pub(crate) why: String,
    pub(crate) goal: String,
    pub(crate) todo: String,
    pub(crate) notify_assistant_when_done: bool,
    pub(crate) call_stack: Vec<String>,
}
