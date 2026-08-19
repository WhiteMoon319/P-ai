//! StateAccess trait：抽象 AppState 的缓存+持久化接口。
//!
//! 本 trait 是「命令编排层」（22 个大文件）脱离 AppState 编译的关键：
//! 业务逻辑只依赖 `&impl StateAccess`，不依赖 concrete AppState。
//!
//! 实现方（src-tauri AppState）委托给现有的 `state_read_*_cached` 函数。
//!
//! ## 扩展说明
//!
//! 当前包含最常用的方法。以下方法待对应类型迁入 crates 后加入：
//! - `read_conversation_cached`（需要 `ChatIndexFile` 先迁入 pai-backend）
//! - `read_conversation_metadata_cached`
//! - `read_chat_index_cached`
//! - `schedule_conversation_persist` / `delete_conversation_cached`

use std::path::Path;

use pai_backend::core::domain::types_storage::RuntimeStateFile;
use pai_backend::core::domain::types_config::AppConfig;
use pai_backend::core::domain::types_chat::AgentProfile;

/// 运行时状态访问接口。
pub trait StateAccess {
    // ── 配置 ──

    /// 读取缓存的 AppConfig（带文件 mtime 缓存，必要时从磁盘重读）。
    fn read_config_cached(&self) -> Result<AppConfig, String>;

    // ── 运行时状态 ──

    /// 读取缓存的 RuntimeStateFile。
    fn read_runtime_state_cached(&self) -> Result<RuntimeStateFile, String>;

    /// 可变操作 RuntimeStateFile：读 → 回调修改 → 标记脏。
    fn mutate_runtime_state_cached<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut RuntimeStateFile) -> Result<T, String>;

    // ── 代理配置 ──

    /// 读取缓存的 AgentProfile 列表。
    fn read_agents_cached(&self) -> Result<Vec<AgentProfile>, String>;

    // ── 运行时路径 ──

    /// 应用数据根目录。
    fn data_path(&self) -> &Path;
    /// 配置文件路径。
    fn config_path(&self) -> &Path;
    /// LLM 工作区路径。
    fn llm_workspace_path(&self) -> &Path;
}