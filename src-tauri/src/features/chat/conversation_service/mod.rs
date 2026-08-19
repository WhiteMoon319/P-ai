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
#[allow(unused_imports)]
use pai_android_bridge::state_access::StateAccess;
use super::*;
// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐


use super::*;

#[path = "types.rs"]
mod conversation_service_types;
pub(crate) use conversation_service_types::*;
#[path = "conversation_service_v2.rs"]
mod conversation_service_v2;
pub(crate) use conversation_service_v2::*;
#[path = "remote_im_sessions.rs"]
mod remote_im_sessions;
pub(crate) use remote_im_sessions::*;
#[path = "archive_lifecycle.rs"]
mod archive_lifecycle;
pub(crate) use archive_lifecycle::*;
#[path = "assistant_message_mutations.rs"]
mod assistant_message_mutations;
pub(crate) use assistant_message_mutations::*;
#[path = "scheduler_history_flush.rs"]
mod scheduler_history_flush;
pub(crate) use scheduler_history_flush::*;
#[path = "history_mutations.rs"]
mod history_mutations;
pub(crate) use history_mutations::*;
#[path = "prompt_prepare.rs"]
mod prompt_prepare;
pub(crate) use prompt_prepare::*;
#[path = "delegate_resolution.rs"]
mod delegate_resolution;
pub(crate) use delegate_resolution::*;
#[path = "conversation_reads.rs"]
mod conversation_reads;
pub(crate) use conversation_reads::*;
#[path = "context_reads.rs"]
mod context_reads;
pub(crate) use context_reads::*;
#[path = "preserved_dialogue.rs"]
mod preserved_dialogue;
pub(crate) use preserved_dialogue::*;
#[path = "foreground_lifecycle.rs"]
mod foreground_lifecycle;
pub(crate) use foreground_lifecycle::*;
#[path = "metadata_mutations.rs"]
mod metadata_mutations;
pub(crate) use metadata_mutations::*;
#[path = "persistence.rs"]
mod persistence;
pub(crate) use persistence::*;
#[path = "archive.rs"]
mod archive;
pub(crate) use archive::*;
#[path = "session_notification_support.rs"]
mod session_notification_support;
pub(crate) use session_notification_support::*;
#[path = "mutations.rs"]
mod mutations;
pub(crate) use mutations::*;
