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


use std::collections::{HashMap, HashSet};
use std::path::Path;
use super::*;
// tools/types.rs 与 tools/operate_parser.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::desktop_tools::operate_parser::*;
pub(crate) use pai_backend::desktop_tools::types::*;
// image_normalizer 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::image_normalizer::*;
#[cfg(target_os = "android")]
#[path = "tools/desktop_only_android_stub.rs"]
mod desktop_only_android_stub;
#[cfg(target_os = "android")]
pub(crate) use desktop_only_android_stub::*;
#[path = "tools/operate_mcp.rs"]
mod tools_operate_mcp;
pub(crate) use tools_operate_mcp::*;
#[path = "tools/screenshot_mcp.rs"]
mod tools_screenshot_mcp;
pub(crate) use tools_screenshot_mcp::*;
#[path = "tools/terminal.rs"]
mod tools_terminal;
pub(crate) use tools_terminal::*;
// text_codec 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::text_codec::*;
#[path = "tools/patch.rs"]
mod tools_patch;
pub(crate) use tools_patch::*;
#[path = "tools/patch_rewind.rs"]
mod tools_patch_rewind;
pub(crate) use tools_patch_rewind::*;
#[path = "tools/read_file.rs"]
mod tools_read_file;
pub(crate) use tools_read_file::*;
#[path = "tools/todo_mcp.rs"]
mod tools_todo_mcp;
pub(crate) use tools_todo_mcp::*;
