//! StateAccess trait 的 AppState 实现（委托给现有的 state_read_*_cached 函数）。
//! 阶段 6：命令编排层参数化，使业务逻辑可脱离 AppState 编译。

use pai_android_bridge::state_access::StateAccess;
use pai_backend::core::domain::runtime_types::{
    RemoteImContactRuntimeState, RemoteImReplyDelegateRuntime,
};
use pai_backend::core::domain::types_chat::AgentProfile;
use pai_backend::core::domain::types_config::AppConfig;
use pai_backend::core::domain::types_storage::RuntimeStateFile;

use crate::AppState;
use crate::features_system_commands::runtime_log_warn;
use crate::state_read_config_cached;
use crate::state_read_agents_cached;
use crate::state_read_runtime_state_cached;
use crate::state_mutate_runtime_state_cached;

impl StateAccess for AppState {
    fn read_config_cached(&self) -> Result<AppConfig, String> {
        state_read_config_cached(self)
    }

    fn read_runtime_state_cached(&self) -> Result<RuntimeStateFile, String> {
        state_read_runtime_state_cached(self)
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
}