pub(crate) use std::{
    fs,
    io::Cursor,
    path::{Path, PathBuf},
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


pub(crate) use super::*;
// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐

// 核心常量与远端客服默认文案已迁移至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::core::domain::constants::*;
pub(crate) use pai_backend::core::domain::remote_customer_service_defaults::*;
pub(crate) use pai_backend::core::domain::runtime_defaults::*;
pub(crate) use pai_backend::core::domain::runtime_types::*;
pub(crate) use pai_backend::core::domain::types_chat::*;
pub(crate) use pai_backend::core::domain::types_config::*;
pub(crate) use pai_backend::core::domain::types_foundation::*;
pub(crate) use pai_backend::core::domain::types_image_generation::*;
pub(crate) use pai_backend::core::domain::types_requests::*;
pub(crate) use pai_backend::core::domain::types_storage::*;

#[path = "domain/http_identity.rs"]
mod domain_http_identity;
pub(crate) use domain_http_identity::*;
#[path = "domain/runtime.rs"]
mod domain_runtime;
pub(crate) use domain_runtime::*;
