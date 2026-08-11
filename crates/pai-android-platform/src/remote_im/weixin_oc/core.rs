use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

use pai_backend::core::domain::types_requests::ChatIngressPart;

use crate::local_port_service::LocalPortServiceCore;

pub const WEIXIN_OC_DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
pub const WEIXIN_OC_DEFAULT_CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c";
pub const WEIXIN_OC_DEFAULT_LONG_POLL_TIMEOUT_MS: u64 = 35_000;
pub const WEIXIN_OC_DEFAULT_API_TIMEOUT_MS: u64 = 15_000;
pub const WEIXIN_OC_DEFAULT_BOT_TYPE: &str = "3";
pub const WEIXIN_OC_LOGIN_TTL_SECS: i64 = 5 * 60;
pub const WEIXIN_OC_TEXT_ITEM_TYPE: i64 = 1;
pub const WEIXIN_OC_IMAGE_ITEM_TYPE: i64 = 2;
pub const WEIXIN_OC_FILE_ITEM_TYPE: i64 = 4;
pub const WEIXIN_OC_VIDEO_ITEM_TYPE: i64 = 5;
pub const WEIXIN_OC_IMAGE_UPLOAD_TYPE: i64 = 1;
pub const WEIXIN_OC_FILE_UPLOAD_TYPE: i64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcLoginSession {
    pub session_key: String,
    pub qrcode: String,
    pub qrcode_img_content: String,
    pub started_at: String,
    pub status: String,
    #[serde(default)]
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcLoginStartInput {
    pub channel_id: String,
    #[serde(default)]
    pub force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcLoginStatusInput {
    pub channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcLoginStartResult {
    pub channel_id: String,
    pub session_key: String,
    pub qrcode: String,
    pub qrcode_img_content: String,
    pub status: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcLoginStatusResult {
    pub channel_id: String,
    pub connected: bool,
    pub status: String,
    pub message: String,
    #[serde(default)]
    pub session_key: String,
    #[serde(default)]
    pub qrcode: String,
    #[serde(default)]
    pub qrcode_img_content: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub last_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcSyncContactsResult {
    pub channel_id: String,
    pub synced_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct WeixinOcCredentials {
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub cdn_base_url: String,
    #[serde(default)]
    pub bot_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_poll_interval: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_poll_timeout_ms: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_timeout_ms: Option<u64>,
    #[serde(default)]
    pub token: String,
    #[serde(default)]
    pub account_id: String,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub sync_buf: String,
}

impl WeixinOcCredentials {
    pub fn from_value(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    pub fn normalized_base_url(&self) -> String {
        let base = self.base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            WEIXIN_OC_DEFAULT_BASE_URL.to_string()
        } else {
            base.to_string()
        }
    }

    pub fn normalized_cdn_base_url(&self) -> String {
        let base = self.cdn_base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            WEIXIN_OC_DEFAULT_CDN_BASE_URL.to_string()
        } else {
            base.to_string()
        }
    }

    pub fn normalized_bot_type(&self) -> String {
        let out = self.bot_type.trim();
        if out.is_empty() {
            WEIXIN_OC_DEFAULT_BOT_TYPE.to_string()
        } else {
            out.to_string()
        }
    }

    pub fn normalized_long_poll_timeout_ms(&self) -> u64 {
        self.long_poll_timeout_ms
            .unwrap_or(WEIXIN_OC_DEFAULT_LONG_POLL_TIMEOUT_MS)
            .clamp(5_000, 60_000)
    }

    pub fn normalized_api_timeout_ms(&self) -> u64 {
        self.api_timeout_ms
            .unwrap_or(WEIXIN_OC_DEFAULT_API_TIMEOUT_MS)
            .clamp(5_000, 60_000)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcGetBotQrCodeResp {
    pub qrcode: Option<String>,
    pub qrcode_img_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcQrStatusResp {
    pub ret: Option<i64>,
    pub errcode: Option<i64>,
    pub errmsg: Option<String>,
    pub status: Option<String>,
    #[serde(alias = "botToken")]
    pub bot_token: Option<String>,
    #[serde(alias = "ilinkBotId")]
    pub ilink_bot_id: Option<String>,
    #[serde(alias = "ilinkUserId")]
    pub ilink_user_id: Option<String>,
    #[serde(alias = "baseUrl")]
    pub baseurl: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcGetUpdatesResp {
    pub ret: Option<i64>,
    pub errcode: Option<i64>,
    pub errmsg: Option<String>,
    pub msgs: Option<Vec<WeixinOcInboundMessage>>,
    pub get_updates_buf: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcInboundMessage {
    pub message_id: Option<Value>,
    pub msg_id: Option<Value>,
    pub from_user_id: Option<String>,
    pub context_token: Option<String>,
    pub item_list: Option<Vec<WeixinOcMessageItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcMessageItem {
    #[serde(rename = "type")]
    pub item_type: Option<i64>,
    pub text_item: Option<WeixinOcTextItem>,
    pub image_item: Option<WeixinOcImageItem>,
    pub voice_item: Option<WeixinOcVoiceItem>,
    pub file_item: Option<WeixinOcFileItem>,
    pub video_item: Option<WeixinOcVideoItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcTextItem {
    pub text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcImageItem {
    pub media: Option<WeixinOcMediaPayload>,
    pub aeskey: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcMediaPayload {
    pub encrypt_query_param: Option<String>,
    pub aes_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcVoiceItem {
    pub media: Option<WeixinOcMediaPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcFileItem {
    pub media: Option<WeixinOcMediaPayload>,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeixinOcVideoItem {
    pub media: Option<WeixinOcMediaPayload>,
}

#[derive(Debug, Clone)]
pub struct WeixinOcCollectedMedia {
    pub parts: Vec<ChatIngressPart>,
}

#[derive(Debug, Clone)]
pub struct WeixinOcRuntimeState {
    pub connected: bool,
    pub connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub base_url: String,
    pub account_id: String,
    pub user_id: String,
    pub session_key: String,
    pub qrcode: String,
    pub qrcode_img_content: String,
    pub login_status: String,
    pub last_error: String,
}

impl Default for WeixinOcRuntimeState {
    fn default() -> Self {
        Self {
            connected: false,
            connected_at: None,
            base_url: WEIXIN_OC_DEFAULT_BASE_URL.to_string(),
            account_id: String::new(),
            user_id: String::new(),
            session_key: String::new(),
            qrcode: String::new(),
            qrcode_img_content: String::new(),
            login_status: "idle".to_string(),
            last_error: String::new(),
        }
    }
}

pub const WEIXIN_OC_TYPING_TICKET_TTL_SECS: u64 = 60;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct WeixinOcTypingTicketState {
    pub ilink_user_id: String,
    pub typing_ticket: String,
    pub ticket_context_token: String,
    pub ticket_refresh_after: std::time::Instant,
}

#[derive(Debug)]
pub struct WeixinOcTypingState {
    pub ticket_state: WeixinOcTypingTicketState,
    pub cancel_tx: tokio::sync::oneshot::Sender<()>,
}

pub struct WeixinOcManager {
    pub states: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcRuntimeState>>,
    >,
    pub login_sessions: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcLoginSession>>,
    >,
    pub stop_senders: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>,
    >,
    pub tasks: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
        >,
    >,
    pub context_tokens: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, String>>,
    >,
    pub typing_states: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcTypingState>>,
    >,
    pub port_service: std::sync::Arc<LocalPortServiceCore>,
}

impl WeixinOcManager {
    pub fn new() -> Self {
        Self {
            states: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            login_sessions: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            stop_senders: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            tasks: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            context_tokens: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            typing_states: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
            port_service: std::sync::Arc::new(LocalPortServiceCore::new()),
        }
    }
}

impl Default for WeixinOcManager {
    fn default() -> Self {
        Self::new()
    }
}

pub static WEIXIN_OC_MANAGER: once_cell::sync::Lazy<std::sync::Arc<WeixinOcManager>> =
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(WeixinOcManager::new()));

pub fn weixin_oc_manager() -> std::sync::Arc<WeixinOcManager> {
    WEIXIN_OC_MANAGER.clone()
}

pub fn login_session_is_fresh(login: &WeixinOcLoginSession) -> bool {
    chrono::DateTime::parse_from_rfc3339(login.started_at.trim())
        .map(|ts| {
            chrono::Utc::now()
                .signed_duration_since(ts.with_timezone(&chrono::Utc))
                .num_seconds()
                < WEIXIN_OC_LOGIN_TTL_SECS
        })
        .unwrap_or(false)
}
