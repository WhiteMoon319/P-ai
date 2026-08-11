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


use super::*;
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

include!("domain/http_identity.rs");
include!("domain/runtime.rs");
