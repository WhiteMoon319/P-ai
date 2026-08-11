pub(crate) fn goal_active_goal_from_conversation(
    conversation: &Conversation,
) -> Option<ConversationGoalState> {
    conversation
        .active_goal
        .as_ref()
        .filter(|goal| conversation_goal_is_active(goal))
        .cloned()
}

pub(crate) struct ConversationTodosUpdateResult {
    pub(crate) current_todo: Option<String>,
}

pub(crate) struct CreateUnarchivedConversationMutationResult {
    pub(crate) conversation_id: String,
    pub(crate) overview_payload: UnarchivedConversationOverviewUpdatedPayload,
}

pub(crate) struct BranchUnarchivedConversationMutationResult {
    pub(crate) conversation_id: String,
    pub(crate) title: String,
    pub(crate) selected_count: usize,
    pub(crate) has_compaction_seed: bool,
    pub(crate) overview_payload: UnarchivedConversationOverviewUpdatedPayload,
}

pub(crate) struct ForwardUnarchivedConversationMutationResult {
    pub(crate) target_conversation_id: String,
    pub(crate) forwarded_count: usize,
    pub(crate) overview_payload: UnarchivedConversationOverviewUpdatedPayload,
}

pub(crate) struct ForwardSelectionToRemoteImContactMutationResult {
    pub(crate) target_conversation_id: String,
    pub(crate) remote_contact_id: String,
    pub(crate) forwarded_count: usize,
    pub(crate) overview_payload: UnarchivedConversationOverviewUpdatedPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolSessionTargetSummary {
    pub(crate) session_id: String,
    pub(crate) kind: String,
    pub(crate) title: String,
    pub(crate) department_name: Option<String>,
    pub(crate) persona_name: Option<String>,
    pub(crate) remote_contact_id: Option<String>,
    pub(crate) remote_contact_name: Option<String>,
    pub(crate) channel_name: Option<String>,
    pub(crate) updated_at: String,
}

pub(crate) struct InformSessionMutationResult {
    pub(crate) target_conversation_id: String,
    pub(crate) target_kind: String,
    pub(crate) remote_contact_id: Option<String>,
    pub(crate) pushed_to_remote: bool,
    pub(crate) message: ChatMessage,
}

pub(crate) struct DeleteUnarchivedConversationMutationResult {
    pub(crate) deleted_conversation_id: String,
    pub(crate) active_conversation_id: String,
    pub(crate) overview_payload: UnarchivedConversationOverviewUpdatedPayload,
}

pub(crate) struct ToggleUnarchivedConversationPinMutationResult {
    pub(crate) conversation_id: String,
    pub(crate) is_pinned: bool,
    pub(crate) pin_index: Option<usize>,
}

pub(crate) struct PromptPrepareConversationResolution {
    pub(crate) conversation_before: Conversation,
    pub(crate) last_archive_summary: Option<String>,
    pub(crate) is_remote_im_contact_conversation: bool,
    pub(crate) remote_im_contact_processing_mode: String,
    pub(crate) response_style_id: String,
    pub(crate) user_name: String,
    pub(crate) user_intro: String,
    pub(crate) is_runtime_conversation: bool,
}

pub(crate) struct SchedulerHistoryFlushCommitResult {
    pub(crate) persisted_batch_messages: Vec<ChatMessage>,
    pub(crate) event_activate_flags: Vec<bool>,
}

pub(crate) struct DelegateResultTargetConversationResolution {
    pub(crate) department_id: String,
    pub(crate) agent_id: String,
    pub(crate) target_conversation_id: String,
}

pub(crate) struct DelegateContextResolution {
    pub(crate) config: AppConfig,
    pub(crate) agents: Vec<AgentProfile>,
    pub(crate) source_department: DepartmentConfig,
    pub(crate) target_department: DepartmentConfig,
    pub(crate) target_agent_id: String,
    pub(crate) source_conversation_id: String,
    pub(crate) thread_context: Option<DelegateRuntimeThread>,
}

pub(crate) struct SwitchActiveConversationSnapshotMutationResult {
    pub(crate) snapshot: ForegroundConversationSnapshotCore,
    pub(crate) unarchived_conversations: Vec<UnarchivedConversationSummary>,
}

pub(crate) struct MarkConversationReadResult {
    pub(crate) conversation: Option<Conversation>,
}

pub(crate) struct RewindConversationMutationResult {
    pub(crate) conversation_id: String,
    pub(crate) removed_count: usize,
    pub(crate) remaining_count: usize,
    pub(crate) current_todo: Option<String>,
    pub(crate) current_todos: Vec<ConversationTodoItem>,
    pub(crate) recalled_user_message: Option<ChatMessage>,
    pub(crate) git_snapshot: Option<git_ghost_snapshot::UserMessageGitGhostSnapshotRecord>,
}

pub(crate) struct RewindConversationPreviewResult {
    pub(crate) conversation_id: String,
    pub(crate) can_undo_patch: bool,
    pub(crate) hint: String,
}

pub(crate) struct StopChatPersistResult {
    pub(crate) persisted: bool,
    pub(crate) conversation_id: Option<String>,
    pub(crate) assistant_message: Option<ChatMessage>,
}

pub(crate) struct ImportArchivesMutationResult {
    pub(crate) imported_count: usize,
    pub(crate) replaced_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) total_count: usize,
    pub(crate) selected_archive_id: Option<String>,
}

pub(crate) struct ConversationBlockSummaryResult {
    pub(crate) block_id: u32,
    pub(crate) message_count: usize,
    pub(crate) first_message_id: String,
    pub(crate) last_message_id: String,
    pub(crate) first_created_at: Option<String>,
    pub(crate) last_created_at: Option<String>,
    pub(crate) is_latest: bool,
}

pub(crate) struct ConversationBlockPageResult {
    pub(crate) blocks: Vec<ConversationBlockSummaryResult>,
    pub(crate) selected_block_id: u32,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) has_prev_block: bool,
    pub(crate) has_next_block: bool,
}

pub(crate) struct CompactionMessagePersistResult {
    pub(crate) active_conversation_id: Option<String>,
    pub(crate) compression_message_id: String,
}

pub(crate) struct ListUnarchivedConversationsMutationResult {
    pub(crate) summaries: Vec<UnarchivedConversationSummary>,
}

pub(crate) struct InstantArchiveConversationMutationResult {
    pub(crate) active_conversation_id: String,
    pub(crate) overview_payload: UnarchivedConversationOverviewUpdatedPayload,
    pub(crate) already_archived: bool,
}
