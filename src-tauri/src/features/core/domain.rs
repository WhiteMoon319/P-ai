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


include!("domain/constants.rs");
include!("domain/remote_customer_service_defaults.rs");
include!("domain/http_identity.rs");
include!("domain/types.rs");
include!("domain/runtime.rs");
