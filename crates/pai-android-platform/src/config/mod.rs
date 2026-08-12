//! P-AI 配置读写与规范化（阶段 6 迁入）。
//!
//! 零 tauri 依赖的纯配置逻辑：TOML 读写、配置规范化、媒体引用、
//! STT / 图片缓存辅助。resolve_api_config 及其 Codex 凭证依赖仍留在
//! src-tauri（依赖桌面 ProjectDirs / portable 全局路径）。

pub mod storage_and_stt;

pub use storage_and_stt::*;