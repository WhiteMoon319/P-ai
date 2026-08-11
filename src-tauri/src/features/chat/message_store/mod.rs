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
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::RwLock;
use super::*;

pub(crate) mod message_store {
    // 第一阶段先建立可独立测试的迁移边界，运行路径接入会在后续阶段完成。
    #![allow(dead_code)]
    // 嵌套 mod 内 glob import 不传递，直接引用 crate 根 pub(crate) 项
    use crate::*;
    // paths/manifest/index 已迁至 crates/pai-backend（阶段 4）。
    pub(crate) use pai_backend::message_store::paths::*;
    pub(crate) use pai_backend::message_store::manifest::*;
    pub(crate) use pai_backend::message_store::index::*;
    // jsonl_snapshot/verification 已迁至 crates/pai-backend（阶段 4）。
    pub(crate) use pai_backend::message_store::jsonl_snapshot::*;
    pub(crate) use pai_backend::message_store::verification::*;
    // active_plan 已迁至 crates/pai-backend（阶段 4）。
    pub(crate) use pai_backend::message_store::active_plan::*;
    // meta 已迁至 crates/pai-backend（阶段 4）。
    pub(crate) use pai_backend::message_store::meta::*;
    include!("sqlite.rs");
    include!("store.rs");
    include!("persist.rs");
    include!("migration.rs");
    include!("usage_trail.rs");
}
