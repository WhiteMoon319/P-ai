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

include!("model_runtime/runtime_abstractions.rs");
include!("model_runtime/runtime_migration_guard.rs");
include!("model_runtime/provider_resolution.rs");

// ==================== 工具定义与内置能力 ====================
include!("model_runtime/tools_and_builtin.rs");

// ==================== Provider 调用与流式处理 ====================
include!("model_runtime/provider_and_stream.rs");
