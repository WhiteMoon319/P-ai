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
use std::path::Path;
use super::*;
// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐


include!("types.rs");
include!("conversation_service_v2.rs");
include!("remote_im_sessions.rs");
include!("archive_lifecycle.rs");
include!("assistant_message_mutations.rs");
include!("scheduler_history_flush.rs");
include!("history_mutations.rs");
include!("prompt_prepare.rs");
include!("delegate_resolution.rs");
include!("conversation_reads.rs");
include!("context_reads.rs");
include!("preserved_dialogue.rs");
include!("foreground_lifecycle.rs");
include!("metadata_mutations.rs");
include!("persistence.rs");
include!("archive.rs");
include!("session_notification_support.rs");
include!("mutations.rs");
