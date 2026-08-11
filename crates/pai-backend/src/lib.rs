//! P-AI 平台无关业务后端（Android 原生重构阶段 4）。
//!
//! 职责：会话 / 消息 / 记忆 / 任务 / 远程 IM / 配置 / migration 等纯业务逻辑，
//! 不依赖 tauri / jni / Android 平台能力；平台接入由 pai-android-bridge 与
//! pai-android-platform 提供（阶段 5-6）。
//!
//! 迁移策略：从 src-tauri 逐步搬入零平台依赖的纯逻辑模块，
//! 每个模块迁移后保持 `cargo check -p pai-backend` 与 Android 交叉编译通过。

pub mod core;
pub mod delegate;
pub mod mcp;
pub mod skill;
pub mod task;
pub mod version_compare;
