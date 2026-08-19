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


use std::path::Path;
use std::pin::Pin;
use std::collections::{HashMap, HashSet};
use super::*;
// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐

// ==================== 运行时共享抽象 ====================

#[path = "model_runtime/runtime_migration_guard.rs"]
mod runtime_migration_guard;
pub(crate) use runtime_migration_guard::*;
// provider_resolution 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::provider_resolution::*;
// runtime_abstractions 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::model_runtime::*;

// ==================== 工具定义与内置能力 ====================
#[path = "model_runtime/tools_and_builtin.rs"]
mod tools_and_builtin;
pub(crate) use tools_and_builtin::*;

// ==================== Provider 调用与流式处理 ====================
#[path = "model_runtime/provider_and_stream.rs"]
mod provider_and_stream;
pub(crate) use provider_and_stream::*;
