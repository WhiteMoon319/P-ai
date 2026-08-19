//! StateAccess trait 的 AppState 实现（委托给现有的 state_read_*_cached 函数）。
//! 阶段 6：命令编排层参数化，使业务逻辑可脱离 AppState 编译。

use pai_android_bridge::state_access::StateAccess;
use pai_backend::core::domain::types_chat::AgentProfile;
use pai_backend::core::domain::types_config::AppConfig;
use pai_backend::core::domain::types_storage::RuntimeStateFile;

use crate::AppState;
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

    fn data_path(&self) -> &std::path::Path {
        &self.data_path
    }

    fn config_path(&self) -> &std::path::Path {
        &self.config_path
    }

    fn llm_workspace_path(&self) -> &std::path::Path {
        &self.llm_workspace_path
    }
}