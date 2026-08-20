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


pub(crate) use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
pub(crate) use std::collections::{HashSet};
pub(crate) use super::*;
pub(crate) use pai_backend::task::domain::*;
// task/migration.rs 与 task/store.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::task::migration::*;
pub(crate) use pai_backend::task::store::*;
#[path = "task/scheduler.rs"]
mod task_scheduler;
pub(crate) use task_scheduler::*;
#[path = "task/commands.rs"]
mod task_commands;
pub(crate) use task_commands::*;
