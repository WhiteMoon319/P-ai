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


use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use std::collections::{HashSet};
use super::*;
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
