//! 记忆持久化存储（SQLite，纯逻辑 + 文件系统，无平台依赖）。

pub mod archive_feedback;
pub mod crud;
pub mod db;
pub mod import_export;
pub mod maintenance;
pub mod ownership;
pub mod provider_index;
pub mod types;

pub use archive_feedback::*;
pub use crud::*;
pub use db::*;
pub use import_export::*;
pub use maintenance::*;
pub use ownership::*;
pub use provider_index::*;
pub use types::*;

/// 使记忆匹配缓存失效（pai-backend 占位；src-tauri matcher.rs 保留完整实现，
/// 因此这里用内部名避免跨 crate 歧义）。
pub fn invalidate_memory_matcher_cache_internal() {}
