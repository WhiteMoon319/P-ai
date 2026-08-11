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
include!("tools/types.rs");
include!("tools/image_normalizer_for_llm_request.rs");
include!("tools/operate_parser.rs");
#[cfg(target_os = "android")]
include!("tools/desktop_only_android_stub.rs");
include!("tools/operate_mcp.rs");
include!("tools/screenshot_mcp.rs");
include!("tools/terminal.rs");
include!("tools/text_codec.rs");
include!("tools/patch.rs");
include!("tools/patch_rewind.rs");
include!("tools/read_file.rs");
include!("tools/todo_mcp.rs");
