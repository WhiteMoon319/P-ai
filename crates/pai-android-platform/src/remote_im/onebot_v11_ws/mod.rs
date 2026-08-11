//! OneBot v11 反向 WebSocket 域（阶段 5 迁入）。
//!
//! 迁移自 src-tauri 的 include! 聚合。原聚合中 models.rs 顶部的 use 在
//! include! 顺序下对所有子文件可见；迁移后统一在此补共享 use，
//! 各子文件以 `use super::*` 引用。

pub mod models;
pub mod transport;
pub mod runtime;
pub mod api;
pub mod lifecycle;
pub mod parsing;
pub mod media;
pub mod inbound;
pub mod state_access;

pub use models::*;
pub use transport::*;
pub use runtime::*;
pub use api::*;
pub use lifecycle::*;
pub use parsing::*;
pub use media::*;
pub use inbound::*;
pub use state_access::*;

// ==================== 共享 use（原聚合 models.rs 头部） ====================
// 注意：这些 use 必须是普通 `use` 而非 `pub use`，否则会通过 src-tauri 的
// `pub(crate) use pai_android_platform::remote_im::onebot_v11_ws::*` 桥接
// 泄漏到 src-tauri 命名空间造成歧义。
use std::net::SocketAddr;
use std::time::Duration;

use axum::extract::ws::{Message as AxumWsMessage, WebSocket};
use chrono::{DateTime, Utc};
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::SinkExt;
use tokio::net::TcpListener;
use tokio::sync::{broadcast, oneshot, watch, RwLock};
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

use std::collections::HashMap;
use std::sync::Arc;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use futures_util::StreamExt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use pai_backend::core::domain::types_config::{
    RemoteImChannelConfig, RemoteImPlatform,
};
use pai_backend::core::domain::types_requests::{
    ChatIngressPart, ChatInputPayload, RemoteImEnqueueInput, RemoteImEnqueueResult, SessionSelector,
};
use pai_backend::core::domain::types_storage::{
    ChannelConnectionStatus, RemoteImContact, RemoteImGroupMemberInfo, RemoteImChannelPrivateState,
};
use pai_backend::core::time_semantics::now_iso;
use pai_backend::logging::{runtime_log_debug, runtime_log_error, runtime_log_info, runtime_log_warn};
use crate::remote_im_sdk::RemoteImSdkSendError;
use crate::local_port_service::{
    ChannelLogEntry, LocalPortServiceCore, LocalPortServiceStartOutcome,
};

// ==================== mime 辅助（与 weixin_oc 内置版一致） ====================
fn media_mime_from_path(path: &std::path::Path) -> Option<&'static str> {
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "pdf" => Some("application/pdf"),
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "bmp" => Some("image/bmp"),
        "heic" => Some("image/heic"),
        "heif" => Some("image/heif"),
        "svg" => Some("image/svg+xml"),
        "wav" | "wave" => Some("audio/wav"),
        "mp3" => Some("audio/mpeg"),
        "m4a" => Some("audio/mp4"),
        "aac" => Some("audio/aac"),
        "aiff" | "aif" => Some("audio/aiff"),
        "ogg" | "oga" => Some("audio/ogg"),
        "opus" => Some("audio/opus"),
        "flac" => Some("audio/flac"),
        "webm" => Some("audio/webm"),
        _ => None,
    }
}

fn image_mime_from_bytes(raw: &[u8]) -> Option<&'static str> {
    infer::get(raw)
        .map(|kind| kind.mime_type())
        .filter(|mime| mime.starts_with("image/"))
}
