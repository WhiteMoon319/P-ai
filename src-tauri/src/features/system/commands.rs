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
include!("commands/ide_context.rs");
include!("commands/config_and_persona.rs");

// ==================== 工具审查命令 ====================
include!("commands/tool_review.rs");

// ==================== 远程前端模式通知命令 ====================
include!("commands/remote_live_update.rs");

// ==================== Codex OAuth 命令 ====================
include!("commands/codex_auth.rs");
include!("commands/codex_usage.rs");

// ==================== 提示词组装层 ====================
include!("commands/prompt_assembly.rs");

// ==================== 调试日志命令 ====================
include!("commands/debug_log_commands.rs");

// ==================== Android 沙盒工作区 ====================
// android_workspace_paths 已迁至 crates/pai-android-platform（阶段 5）。
pub(crate) use pai_android_platform::android_workspace::paths::*;
#[allow(unused_imports)]
pub(crate) use pai_android_bridge::state_access::StateAccess;
include!("commands/android_workspace.rs");

// ==================== 分享导出命令 ====================
// share_export_commands 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::share_export::*;

// ==================== 记忆整理（独立模块） ====================
include!("commands/memory_curation/prompt_contract.rs");

// ==================== JSON提取工具 ====================
// json_extractor 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::json_extractor::*;

// ==================== 归档JSON解析层 ====================
// archive_summary_parser 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::archive_summary_parser::*;

// ==================== 推理网关层 ====================
include!("commands/inference_gateway.rs");

// ==================== 记忆命令 ====================
include!("commands/memory_commands.rs");

// ==================== 记忆供应商命令 ====================
include!("commands/memory_provider_commands.rs");

// ==================== 归档命令 ====================
include!("commands/archive_commands.rs");

// ==================== 归档导入导出命令 ====================
include!("commands/archive_io_commands.rs");

// ==================== 归档主持人格选择 ====================
// archive_host_selector 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::archive_host_selector::*;

// ==================== 会话归档与压缩入口 ====================
include!("commands/conversation_archive.rs");
include!("commands/conversation_compaction.rs");

// ==================== PDF文本服务 ====================
include!("services/pdf_text_service.rs");

// ==================== 归档执行流水线 ====================
include!("commands/archive_pipeline.rs");

// ==================== 对话与运行时命令 ====================
include!("commands/chat_and_runtime.rs");

// ==================== 桌面工具命令 ====================
include!("commands/desktop_tools.rs");
