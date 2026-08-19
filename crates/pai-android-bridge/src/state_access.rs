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

use std::collections::HashMap;
use std::path::Path;

use pai_backend::core::domain::types_storage::RuntimeStateFile;
use pai_backend::core::domain::types_config::AppConfig;
use pai_backend::core::domain::types_chat::{AgentProfile, Conversation, DelegateRuntimeThread};
use pai_backend::core::domain::runtime_types::{RemoteImContactRuntimeState, RemoteImReplyDelegateRuntime};
use pai_backend::message_store::meta::ConversationShardMeta;
use pai_backend::message_store::sqlite::ChatIndexFile;

/// 运行时状态访问接口。
///
/// 实现方必须是 `Clone + Send + Sync`（src-tauri AppState 已实现 `Clone`，
/// 字段均为 Arc/Mutex 线程安全），以便在 `spawn_blocking` / async 任务中复制状态引用。
pub trait StateAccess: Clone + Send + Sync {
    // ── 配置 ──

    /// 读取缓存的 AppConfig（带文件 mtime 缓存，必要时从磁盘重读）。
    fn read_config_cached(&self) -> Result<AppConfig, String>;

    // ── 运行时状态 ──

    /// 读取缓存的 RuntimeStateFile。
    fn read_runtime_state_cached(&self) -> Result<RuntimeStateFile, String>;

    /// 读取缓存的会话索引（ChatIndexFile，带磁盘缓存）。
    fn read_chat_index_cached(&self) -> Result<ChatIndexFile, String>;

    /// 读取会话轻量元数据（带磁盘缓存，禁止整读正文）。
    fn read_conversation_metadata_cached(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationShardMeta, String>;

    /// 读取完整会话缓存（仅轻量元数据不可读时的降级回退路径）。
    fn read_conversation_cached(&self, conversation_id: &str) -> Result<Conversation, String>;

    /// 调度一次会话删除（排队持久化 + 清理缓存）。
    fn schedule_conversation_delete(&self, conversation_id: &str) -> Result<u64, String>;

    /// 可变操作 RuntimeStateFile：读 → 回调修改 → 标记脏。
    fn mutate_runtime_state_cached<T, F>(&self, f: F) -> Result<T, String>
    where
        F: FnOnce(&mut RuntimeStateFile) -> Result<T, String>;

    // ── 代理配置 ──

    /// 读取缓存的 AgentProfile 列表。
    fn read_agents_cached(&self) -> Result<Vec<AgentProfile>, String>;

    /// 共享 HTTP 客户端（用于调用供应商 API）。
    fn shared_http_client(&self) -> &reqwest::Client;

    // ── 运行时路径 ──

    /// 应用数据根目录。
    fn data_path(&self) -> &Path;
    /// 配置文件路径。
    fn config_path(&self) -> &Path;
    /// LLM 工作区路径。
    fn llm_workspace_path(&self) -> &Path;

    // ── 远程 IM 运行时状态 ──

    /// 锁远程 IM 联系人运行时状态表（毒锁自动恢复）。
    fn lock_remote_im_contact_runtime_states(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, HashMap<String, RemoteImContactRuntimeState>>,
        String,
    >;

    /// 锁远程 IM 应答委托运行时表（毒锁自动恢复）。
    fn lock_remote_im_reply_delegate_runtimes(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, HashMap<String, RemoteImReplyDelegateRuntime>>,
        String,
    >;

    /// 读取最后可信的配置缓存快照（不重读磁盘）。
    fn stale_cached_config_best_effort(&self) -> Option<AppConfig>;

    /// 广播应用级事件（桌面端转发 tauri emit，Android 原生模式为空操作）。
    fn emit_app_event<S: serde::Serialize + Clone>(&self, event: &str, payload: S) -> Result<(), String>;

    /// 取消指定 key 的进行中聊天（inflight abort handle），返回是否找到并取消。
    fn abort_inflight_chat(&self, key: &str) -> bool;

    // ── 会话持久化 ──

    /// 调度一次会话持久化（排队给 worker 异步落盘）。
    fn schedule_conversation_persist(
        &self,
        conversation: &Conversation,
    ) -> Result<u64, String>;

    /// 标记会话 metadata 已直接持久化（更新缓存 mtime）。
    fn mark_conversation_metadata_direct_persisted(
        &self,
        conversation_id: &str,
    ) -> Result<ConversationShardMeta, String>;

    /// 标记会话 metadata 缓存已持久化（清除待落盘标记）。
    fn mark_conversation_metadata_cached_persisted(
        &self,
        conversation_id: &str,
    ) -> Result<(), String>;

    /// 阻塞等待所有待落盘持久化完成（migration/恢复等同步场景）。
    fn flush_pending_persists_blocking(&self) -> Result<bool, String>;

    /// 在会话突变门控下执行闭包（防止并发写冲突）。
    fn with_conversation_mutation<T, F>(
        &self,
        conversation_id: &str,
        task_name: &str,
        f: F,
    ) -> Result<T, String>
    where
        F: FnOnce() -> Result<T, String>;

    /// 无锁更新会话轻量元数据缓存并排队落盘（调用方需已持有会话突变门控）。
    /// 返回 `(更新后的元数据, 回调结果, persist seq)`。
    fn update_conversation_meta_cached_unlocked<T, F>(
        &self,
        normalized_conversation_id: &str,
        updater: F,
    ) -> Result<(ConversationShardMeta, T, u64), String>
    where
        F: FnOnce(&mut ConversationShardMeta) -> Result<T, String>;

    /// 带门控更新会话元数据缓存并排队落盘（自动加会话突变门控）。
    /// 返回 `(更新后的会话快照, 回调结果, persist seq)`。
    fn update_conversation_metadata_cached<T, F>(
        &self,
        conversation_id: &str,
        updater: F,
    ) -> Result<(Conversation, T, u64), String>
    where
        F: FnOnce(&mut Conversation) -> Result<T, String>;

    /// 无锁更新会话元数据缓存并排队落盘（调用方需已持有会话突变门控）。
    /// 返回 `(更新后的会话快照, 回调结果, persist seq)`。
    fn update_conversation_metadata_cached_unlocked<T, F>(
        &self,
        normalized_conversation_id: &str,
        updater: F,
    ) -> Result<(Conversation, T, u64), String>
    where
        F: FnOnce(&mut Conversation) -> Result<T, String>;

    /// 会话是否处于前台活动视图（用于后台消息不计未读）。
    fn conversation_has_active_chat_view(&self, conversation_id: &str) -> bool;

    /// 将会话 upsert 进缓存会话索引（ChatIndexFile）。
    fn upsert_chat_index_conversation_cached(
        &self,
        conversation: &Conversation,
    ) -> Result<(), String>;

    // ── 应答委托运行时表 ──

    /// 锁应答委托运行时线程表（delegate_runtime_threads）。
    fn lock_delegate_runtime_threads(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::HashMap<String, DelegateRuntimeThread>>,
        String,
    >;

    /// 锁最近应答委托线程表（delegate_recent_threads）。
    fn lock_delegate_recent_threads(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::VecDeque<DelegateRuntimeThread>>,
        String,
    >;

    /// 锁已完成工具历史表（inflight_completed_tool_history）。
    fn lock_inflight_completed_tool_history(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, std::collections::HashMap<String, Vec<serde_json::Value>>>,
        String,
    >;

    /// 锁进行中工具中止句柄表（inflight_tool_abort_handles）。
    fn lock_inflight_tool_abort_handles(
        &self,
    ) -> Result<
        std::sync::MutexGuard<
            '_,
            std::collections::HashMap<String, futures_util::future::AbortHandle>,
        >,
        String,
    >;
}