//! StateAccess trait 的 AppState 实现（委托给现有的 state_read_*_cached 函数）。
//! 阶段 6：命令编排层参数化，使业务逻辑可脱离 AppState 编译。

use pai_android_bridge::state_access::StateAccess;
use pai_backend::core::domain::runtime_types::{
    RemoteImContactRuntimeState, RemoteImReplyDelegateRuntime,
};
use pai_backend::core::domain::types_chat::{AgentProfile, Conversation, DelegateRuntimeThread};
use pai_backend::core::domain::types_config::AppConfig;
use pai_backend::core::domain::types_storage::RuntimeStateFile;
use pai_backend::message_store::meta::ConversationShardMeta;
use pai_backend::message_store::sqlite::ChatIndexFile;

use crate::AppState;
use crate::features_system_commands::runtime_log_warn;
use crate::state_read_config_cached;
use crate::state_read_agents_cached;
use crate::state_read_runtime_state_cached;
use crate::state_read_chat_index_cached;
use crate::state_read_conversation_metadata_cached;
use crate::state_read_conversation_cached;
use crate::state_mutate_runtime_state_cached;
use crate::state_schedule_conversation_delete;
use crate::state_schedule_conversation_persist;
use crate::state_mark_conversation_metadata_direct_persisted;
use crate::state_mark_conversation_metadata_cached_persisted_unlocked;
use crate::flush_pending_persists_blocking;
use crate::with_conversation_mutation_for_data_path;
use crate::state_update_conversation_meta_cached_unlocked;
use crate::state_update_conversation_metadata_cached;
use crate::state_update_conversation_metadata_cached_unlocked;
use crate::state_upsert_chat_index_conversation_cached;

impl StateAccess for AppState {
    fn read_config_cached(&self) -> Result<AppConfig, String> {
        state_read_config_cached(self)
    }

    fn read_runtime_state_cached(&self) -> Result<RuntimeStateFile, String> {
        state_read_runtime_state_cached(self)
    }

    fn read_chat_index_cached(&self) -> Result<ChatIndexFile, String> {
        state_read_chat_index_cached(self)
    }

    fn read_conversation_metadata_cached(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationShardMeta, String> {
        state_read_conversation_metadata_cached(self, conversation_id)
    }

    fn read_conversation_cached(&self, conversation_id: &str) -> Result<Conversation, String> {
        state_read_conversation_cached(self, conversation_id)
    }

    fn schedule_conversation_delete(&self, conversation_id: &str) -> Result<u64, String> {
        state_schedule_conversation_delete(self, conversation_id)
    }

    fn mutate_runtime_state_cached<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut RuntimeStateFile) -> Result<T, String>,
    {
        state_mutate_runtime_state_cached(self, f)
    }

    fn read_agents_cached(&self) -> Result<Vec<AgentProfile>, String> {
        state_read_agents_cached(self)
    }

    fn shared_http_client(&self) -> &reqwest::Client {
        &self.shared_http_client
    }

    fn data_path(&self) -> &std::path::Path {
        &self.data_path
    }

    fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    fn llm_workspace_path(&self) -> &std::path::Path {
        &self.llm_workspace_path
    }

    fn lock_remote_im_contact_runtime_states(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::HashMap<String, RemoteImContactRuntimeState>>,
        String,
    > {
        match self.remote_im_contact_runtime_states.lock() {
            Ok(states) => Ok(states),
            Err(poisoned) => {
                runtime_log_warn(
                    "[远程联系人状态机] 运行时锁中毒，已恢复并继续处理当前业务".to_string(),
                );
                Ok(poisoned.into_inner())
            }
        }
    }

    fn lock_remote_im_reply_delegate_runtimes(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::HashMap<String, RemoteImReplyDelegateRuntime>>,
        String,
    > {
        match self.remote_im_reply_delegate_runtimes.lock() {
            Ok(runtimes) => Ok(runtimes),
            Err(poisoned) => {
                runtime_log_warn(
                    "[远程应答委托] 运行时锁中毒，已恢复并继续处理当前业务".to_string(),
                );
                Ok(poisoned.into_inner())
            }
        }
    }

    fn stale_cached_config_best_effort(&self) -> Option<AppConfig> {
        match self.cached_config.lock() {
            Ok(cached) => cached.clone(),
            Err(poisoned) => {
                runtime_log_warn(
                    "[远程IM] 渠道配置缓存锁中毒，恢复锁但不使用不确定权限快照".to_string(),
                );
                self.cached_config.clear_poison();
                drop(poisoned.into_inner());
                None
            }
        }
    }

    fn emit_app_event<S: serde::Serialize + Clone>(
        &self,
        event: &str,
        payload: S,
    ) -> Result<(), String> {
        match self.app_handle.lock() {
            Ok(guard) => {
                if let Some(app_handle) = guard.as_ref() {
                    app_handle.emit(event, payload)
                } else {
                    Ok(())
                }
            }
            Err(_) => Ok(()),
        }
    }

    fn abort_inflight_chat(&self, key: &str) -> bool {
        match self.inflight_chat_abort_handles.lock() {
            Ok(mut inflight) => {
                if let Some(handle) = inflight.remove(key) {
                    handle.abort();
                    true
                } else {
                    false
                }
            }
            Err(poisoned) => {
                runtime_log_warn(format!(
                    "[远程应答委托] 聊天取消句柄锁中毒，已恢复，key={}",
                    key
                ));
                let mut inflight = poisoned.into_inner();
                if let Some(handle) = inflight.remove(key) {
                    handle.abort();
                    true
                } else {
                    false
                }
            }
        }
    }

    fn schedule_conversation_persist(
        &self,
        conversation: &Conversation,
    ) -> Result<u64, String> {
        state_schedule_conversation_persist(self, conversation)
    }

    fn mark_conversation_metadata_direct_persisted(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationShardMeta, String> {
        state_mark_conversation_metadata_direct_persisted(self, conversation_id)
    }

    fn mark_conversation_metadata_cached_persisted(
        &self,
        conversation_id: &str,
    ) -> Result<(), String> {
        state_mark_conversation_metadata_cached_persisted_unlocked(self, conversation_id)
    }

    fn flush_pending_persists_blocking(&self) -> Result<bool, String> {
        flush_pending_persists_blocking(self)
    }

    fn with_conversation_mutation<T, F>(
        &self,
        conversation_id: &str,
        task_name: &str,
        f: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>,
    {
        with_conversation_mutation_for_data_path(&self.data_path(), conversation_id, task_name, f)
    }

    fn update_conversation_meta_cached_unlocked<T, F>(
        &self,
        normalized_conversation_id: &str,
        updater: F,
    ) -> Result<(ConversationShardMeta, T, u64), String>
    where
        F: FnOnce(&mut ConversationShardMeta) -> Result<T, String>,
    {
        state_update_conversation_meta_cached_unlocked(
            self,
            normalized_conversation_id,
            updater,
        )
    }

    fn update_conversation_metadata_cached<T, F>(
        &self,
        conversation_id: &str,
        updater: F,
    ) -> Result<(Conversation, T, u64), String>
    where
        F: FnOnce(&mut Conversation) -> Result<T, String>,
    {
        state_update_conversation_metadata_cached(self, conversation_id, updater)
    }

    fn update_conversation_metadata_cached_unlocked<T, F>(
        &self,
        normalized_conversation_id: &str,
        updater: F,
    ) -> Result<(Conversation, T, u64), String>
    where
        F: FnOnce(&mut Conversation) -> Result<T, String>,
    {
        state_update_conversation_metadata_cached_unlocked(
            self,
            normalized_conversation_id,
            updater,
        )
    }

    fn conversation_has_active_chat_view(&self, conversation_id: &str) -> bool {
        let target = conversation_id.trim();
        if target.is_empty() {
            return false;
        }
        match self.active_chat_view_bindings.lock() {
            Ok(bindings) => bindings.values().any(|binding| {
                let bound = binding.conversation_id.trim();
                !bound.is_empty() && bound != "*" && bound == target
            }),
            Err(poisoned) => {
                runtime_log_warn(
                    "[会话活动视图] 运行时锁中毒，已恢复并视作无活动视图".to_string(),
                );
                let bindings = poisoned.into_inner();
                bindings.values().any(|binding| {
                    let bound = binding.conversation_id.trim();
                    !bound.is_empty() && bound != "*" && bound == target
                })
            }
        }
    }

    fn upsert_chat_index_conversation_cached(
        &self,
        conversation: &Conversation,
    ) -> Result<(), String> {
        state_upsert_chat_index_conversation_cached(self, conversation)
    }

    fn lock_delegate_runtime_threads(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::HashMap<String, DelegateRuntimeThread>>,
        String,
    > {
        self.delegate_runtime_threads
            .lock()
            .map_err(|_| "Failed to lock delegate runtime threads".to_string())
    }

    fn lock_delegate_recent_threads(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::VecDeque<DelegateRuntimeThread>>,
        String,
    > {
        self.delegate_recent_threads
            .lock()
            .map_err(|_| "Failed to lock recent delegate runtime threads".to_string())
    }

    fn lock_inflight_completed_tool_history(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::HashMap<String, Vec<serde_json::Value>>>,
        String,
    > {
        self.inflight_completed_tool_history
            .lock()
            .map_err(|_| "Failed to lock inflight completed tool history".to_string())
    }

    fn lock_inflight_tool_abort_handles(
        &self,
    ) -> Result<
        std::sync::MutexGuard<
            '_,
            std::collections::HashMap<String, futures_util::future::AbortHandle>,
        >,
        String,
    > {
        self.inflight_tool_abort_handles
            .lock()
            .map_err(|_| "Failed to lock inflight tool abort handles".to_string())
    }
}