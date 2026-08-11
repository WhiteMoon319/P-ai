#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteImGroupReplyPhase {
    NonMentionScheduled,
    SecretaryJudging,
    MentionScheduled,
    AssistantDispatching,
    CommitPending,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteImGroupReplySettlementStatus {
    Delivered,
    Uncertain,
}

#[derive(Clone)]
pub(crate) struct RemoteImGroupReplySettlement {
    pub(crate) boundary_message_id: String,
    pub(crate) final_text: Option<String>,
    pub(crate) outbound_key: Option<String>,
    pub(crate) platform_message_id: Option<String>,
    pub(crate) status: RemoteImGroupReplySettlementStatus,
}

#[derive(Clone)]
pub(crate) struct RemoteImGroupReplyState {
    pub(crate) generation: u64,
    pub(crate) phase: RemoteImGroupReplyPhase,
    pub(crate) start_message_id: String,
    pub(crate) decision_end_message_id: Option<String>,
    pub(crate) focus: bool,
    pub(crate) energy_settled: bool,
    pub(crate) next_round_mention: bool,
    pub(crate) event: ChatPendingEvent,
    pub(crate) due_at: std::time::Instant,
    pub(crate) inspection_kind: RemoteImGroupReplyTimerKind,
    pub(crate) pending_settlement: Option<RemoteImGroupReplySettlement>,
}

#[derive(Default)]
pub(crate) struct RemoteImGroupReplyStateStore {
    pub(crate) next_generation: u64,
    pub(crate) by_contact: std::collections::HashMap<String, RemoteImGroupReplyState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteImGroupReplyTimerKind {
    NonMention,
    Mention,
    Commit,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct RemoteImGroupReplyDispatchPolicy {
    pub(crate) generation: u64,
    pub(crate) focus: bool,
    pub(crate) max_chars: u32,
}

#[derive(Clone)]
pub(crate) struct RemoteImGroupReplyTimerAction {
    pub(crate) state_key: String,
    pub(crate) contact_id: String,
    pub(crate) generation: u64,
    pub(crate) kind: RemoteImGroupReplyTimerKind,
    pub(crate) delay: std::time::Duration,
}

pub(crate) fn remote_im_group_reply_state_store(
) -> &'static std::sync::Mutex<RemoteImGroupReplyStateStore> {
    static STORE: std::sync::OnceLock<std::sync::Mutex<RemoteImGroupReplyStateStore>> =
        std::sync::OnceLock::new();
    STORE.get_or_init(|| std::sync::Mutex::new(RemoteImGroupReplyStateStore::default()))
}

pub(crate) fn lock_remote_im_group_reply_state_store(
) -> std::sync::MutexGuard<'static, RemoteImGroupReplyStateStore> {
    match remote_im_group_reply_state_store().lock() {
        Ok(store) => store,
        Err(poisoned) => {
            runtime_log_warn("[群聊巡检] 状态锁中毒，已恢复并保留现有批次".to_string());
            poisoned.into_inner()
        }
    }
}

pub(crate) fn remote_im_group_reply_state_key(state: &AppState, contact_id: &str) -> String {
    format!("{}::{}", state.data_path.to_string_lossy(), contact_id.trim())
}

pub(crate) fn remote_im_group_reply_next_generation(store: &mut RemoteImGroupReplyStateStore) -> u64 {
    store.next_generation = store.next_generation.saturating_add(1).max(1);
    store.next_generation
}

pub(crate) fn remote_im_group_reply_inspection_delay(
    pacing: &RemoteImGroupReplyPacing,
    sample: f64,
) -> std::time::Duration {
    let sample = sample.clamp(0.0, 1.0);
    let centered = sample * 2.0 - 1.0;
    let seconds = pacing.secretary_inspection_seconds as f64
        * (1.0 + centered * pacing.inspection_jitter_ratio);
    std::time::Duration::from_secs_f64(seconds.max(1.0))
}

pub(crate) fn remote_im_group_reply_random_sample() -> f64 {
    let value = Uuid::new_v4().as_u128();
    (value as f64 / u128::MAX as f64).clamp(0.0, 1.0)
}

pub(crate) fn remote_im_group_reply_phase_matches_timer(
    phase: RemoteImGroupReplyPhase,
    kind: RemoteImGroupReplyTimerKind,
) -> bool {
    matches!(
        (phase, kind),
        (
            RemoteImGroupReplyPhase::NonMentionScheduled,
            RemoteImGroupReplyTimerKind::NonMention
        ) | (
            RemoteImGroupReplyPhase::MentionScheduled,
            RemoteImGroupReplyTimerKind::Mention
        ) | (
            RemoteImGroupReplyPhase::CommitPending,
            RemoteImGroupReplyTimerKind::Commit
        )
    )
}

pub(crate) fn remote_im_group_reply_generation_is_current(
    state: &AppState,
    contact_id: &str,
    generation: u64,
) -> bool {
    let key = remote_im_group_reply_state_key(state, contact_id);
    lock_remote_im_group_reply_state_store()
        .by_contact
        .get(&key)
        .map(|entry| entry.generation == generation)
        .unwrap_or(false)
}

pub(crate) fn remote_im_group_reply_has_active_batch(state: &AppState, contact_id: &str) -> bool {
    let key = remote_im_group_reply_state_key(state, contact_id);
    lock_remote_im_group_reply_state_store()
        .by_contact
        .contains_key(&key)
}

#[cfg(test)]
pub(crate) mod remote_im_group_reply_state_tests {
    use super::*;

    #[test]
    fn inspection_delay_should_stay_inside_jitter_bounds() {
        let pacing = RemoteImGroupReplyPacing {
            secretary_inspection_seconds: 10,
            inspection_jitter_ratio: 0.2,
            ..RemoteImGroupReplyPacing::default()
        };
        assert_eq!(remote_im_group_reply_inspection_delay(&pacing, 0.0).as_secs(), 8);
        assert_eq!(remote_im_group_reply_inspection_delay(&pacing, 0.5).as_secs(), 10);
        assert_eq!(remote_im_group_reply_inspection_delay(&pacing, 1.0).as_secs(), 12);
    }
}
