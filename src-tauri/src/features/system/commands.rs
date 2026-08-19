use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use futures_util::{future::AbortHandle, future::join_all, future::BoxFuture, StreamExt};
use image::ImageFormat;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rmcp::{schemars, ServiceExt};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use uuid::Uuid;

// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐

// ==================== 配置与人格命令 ====================

use std::collections::{HashMap, HashSet};
use std::path::Path;
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use super::*;
#[path = "commands/ide_context.rs"]
mod commands_ide_context;
pub(crate) use commands_ide_context::*;
#[path = "commands/config_and_persona.rs"]
mod commands_config_and_persona;
pub(crate) use commands_config_and_persona::*;

// ==================== 工具审查命令 ====================
#[path = "commands/tool_review.rs"]
mod commands_tool_review;
pub(crate) use commands_tool_review::*;

// ==================== 远程前端模式通知命令 ====================
#[path = "commands/remote_live_update.rs"]
mod commands_remote_live_update;
pub(crate) use commands_remote_live_update::*;

// ==================== Codex OAuth 命令 ====================
#[path = "commands/codex_auth.rs"]
mod commands_codex_auth;
pub(crate) use commands_codex_auth::*;
#[path = "commands/codex_usage.rs"]
mod commands_codex_usage;
pub(crate) use commands_codex_usage::*;

// ==================== 提示词组装层 ====================
#[path = "commands/prompt_assembly.rs"]
mod commands_prompt_assembly;
pub(crate) use commands_prompt_assembly::*;

// ==================== 调试日志命令 ====================
#[path = "commands/debug_log_commands.rs"]
mod commands_debug_log_commands;
pub(crate) use commands_debug_log_commands::*;

// ==================== Android 沙盒工作区 ====================
// android_workspace_paths 已迁至 crates/pai-android-platform（阶段 5）。
pub(crate) use pai_android_platform::android_workspace::paths::*;
#[allow(unused_imports)]
pub(crate) use pai_android_bridge::state_access::StateAccess;
#[path = "commands/android_workspace.rs"]
mod commands_android_workspace;
pub(crate) use commands_android_workspace::*;

// ==================== 分享导出命令 ====================
// share_export_commands 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::share_export::*;

// ==================== 记忆整理（独立模块） ====================
#[path = "commands/memory_curation/prompt_contract.rs"]
mod commands_memory_curation_prompt_contract;
pub(crate) use commands_memory_curation_prompt_contract::*;

// ==================== JSON提取工具 ====================
// json_extractor 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::json_extractor::*;

// ==================== 归档JSON解析层 ====================
// archive_summary_parser 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::archive_summary_parser::*;

// ==================== 推理网关层 ====================
#[path = "commands/inference_gateway.rs"]
mod commands_inference_gateway;
pub(crate) use commands_inference_gateway::*;

// ==================== 记忆命令 ====================
#[path = "commands/memory_commands.rs"]
mod commands_memory_commands;
pub(crate) use commands_memory_commands::*;

// ==================== 记忆供应商命令 ====================
#[path = "commands/memory_provider_commands.rs"]
mod commands_memory_provider_commands;
pub(crate) use commands_memory_provider_commands::*;

// ==================== 归档命令 ====================
#[path = "commands/archive_commands.rs"]
mod commands_archive_commands;
pub(crate) use commands_archive_commands::*;

// ==================== 归档导入导出命令 ====================
#[path = "commands/archive_io_commands.rs"]
mod commands_archive_io_commands;
pub(crate) use commands_archive_io_commands::*;

// ==================== 归档主持人格选择 ====================
// archive_host_selector 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::archive_host_selector::*;

// ==================== 会话归档与压缩入口 ====================
#[path = "commands/conversation_archive.rs"]
mod commands_conversation_archive;
pub(crate) use commands_conversation_archive::*;
#[path = "commands/conversation_compaction.rs"]
mod commands_conversation_compaction;
pub(crate) use commands_conversation_compaction::*;

// ==================== PDF文本服务 ====================
#[path = "services/pdf_text_service.rs"]
mod services_pdf_text_service;
pub(crate) use services_pdf_text_service::*;

// ==================== 归档执行流水线 ====================
#[path = "commands/archive_pipeline.rs"]
mod commands_archive_pipeline;
pub(crate) use commands_archive_pipeline::*;

// ==================== 对话与运行时命令 ====================
#[path = "commands/chat_and_runtime.rs"]
mod commands_chat_and_runtime;
pub(crate) use commands_chat_and_runtime::*;

// ==================== 桌面工具命令 ====================
#[path = "commands/desktop_tools.rs"]
mod commands_desktop_tools;
pub(crate) use commands_desktop_tools::*;
