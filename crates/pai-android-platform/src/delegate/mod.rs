//! P-AI 委托（delegate）域。
//!
//! `store`：委托 SQLite 记录库 + 委托会话目录仓库 + 快照缓存（从 src-tauri 迁入）。
//! 平台无关，仅依赖 `data_path` 与文件/数据库工具。

pub mod store;

pub use store::*;