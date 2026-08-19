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


use std::collections::{HashMap, HashSet};
#[allow(unused_imports)]
use pai_android_bridge::state_access::StateAccess;
use super::*;
// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐

// ==================== 独立图像生成模块导图 ====================
// 1) config: 供应商/模型配置归一化与端点解析
// 2) types: 独立请求、结果与运行时中间类型
// 3) storage: 图片校验、下载与 Assistant Space 落盘
// 4) providers/comfyui: 云端供应商与本地工作流适配
// 5) edit: 图像编辑输入解析与编辑 payload 适配
// 6) service/commands: 稳定服务入口与 Tauri 命令

// config/types 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::image_generation::config::*;
pub(crate) use pai_backend::image_generation::types::*;

#[path = "image_generation/storage.rs"]
mod storage;
pub(crate) use storage::*;
#[path = "image_generation/providers.rs"]
mod providers;
pub(crate) use providers::*;
#[path = "image_generation/edit.rs"]
mod edit;
pub(crate) use edit::*;
#[path = "image_generation/comfyui.rs"]
mod comfyui;
pub(crate) use comfyui::*;
#[path = "image_generation/codex.rs"]
mod codex;
pub(crate) use codex::*;
#[path = "image_generation/service.rs"]
mod service;
pub(crate) use service::*;
