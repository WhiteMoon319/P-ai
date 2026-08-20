pub(crate) use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

pub(crate) use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
pub(crate) use directories::ProjectDirs;
pub(crate) use futures_util::{future::AbortHandle, future::join_all, future::BoxFuture, StreamExt};
pub(crate) use image::ImageFormat;
pub(crate) use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
pub(crate) use rmcp::{schemars, ServiceExt};
pub(crate) use scraper::{Html, Selector};
pub(crate) use serde::{Deserialize, Serialize};
pub(crate) use serde_json::Value;
pub(crate) use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
pub(crate) use uuid::Uuid;

// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐


pub(crate) use std::collections::{HashMap, HashSet};
pub(crate) use std::path::Path;
pub(crate) use super::*;
// mcp/types.rs 与 mcp/parser.rs 已迁移至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::mcp::types::*;
pub(crate) use pai_backend::mcp::parser::*;
#[path = "mcp/workspace.rs"]
mod mcp_workspace;
pub(crate) use mcp_workspace::*;
#[path = "mcp/runtime_manager.rs"]
mod mcp_runtime_manager;
pub(crate) use mcp_runtime_manager::*;
#[path = "mcp/commands.rs"]
mod mcp_commands;
pub(crate) use mcp_commands::*;
