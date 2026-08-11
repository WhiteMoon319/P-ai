pub(crate) const WEIXIN_OC_DEFAULT_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
pub(crate) const WEIXIN_OC_DEFAULT_CDN_BASE_URL: &str = "https://novac2c.cdn.weixin.qq.com/c2c";
pub(crate) const WEIXIN_OC_DEFAULT_LONG_POLL_TIMEOUT_MS: u64 = 35_000;
pub(crate) const WEIXIN_OC_DEFAULT_API_TIMEOUT_MS: u64 = 15_000;
pub(crate) const WEIXIN_OC_DEFAULT_BOT_TYPE: &str = "3";
pub(crate) const WEIXIN_OC_LOGIN_TTL_SECS: i64 = 5 * 60;
pub(crate) const WEIXIN_OC_TEXT_ITEM_TYPE: i64 = 1;
pub(crate) const WEIXIN_OC_IMAGE_ITEM_TYPE: i64 = 2;
pub(crate) const WEIXIN_OC_FILE_ITEM_TYPE: i64 = 4;
pub(crate) const WEIXIN_OC_VIDEO_ITEM_TYPE: i64 = 5;
pub(crate) const WEIXIN_OC_IMAGE_UPLOAD_TYPE: i64 = 1;
pub(crate) const WEIXIN_OC_FILE_UPLOAD_TYPE: i64 = 3;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcLoginSession {
    pub(crate) session_key: String,
    pub(crate) qrcode: String,
    pub(crate) qrcode_img_content: String,
    pub(crate) started_at: String,
    pub(crate) status: String,
    #[serde(default)]
    pub(crate) error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcLoginStartInput {
    pub(crate) channel_id: String,
    #[serde(default)]
    pub(crate) force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcLoginStatusInput {
    pub(crate) channel_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcLoginStartResult {
    pub(crate) channel_id: String,
    pub(crate) session_key: String,
    pub(crate) qrcode: String,
    pub(crate) qrcode_img_content: String,
    pub(crate) status: String,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcLoginStatusResult {
    pub(crate) channel_id: String,
    pub(crate) connected: bool,
    pub(crate) status: String,
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) session_key: String,
    #[serde(default)]
    pub(crate) qrcode: String,
    #[serde(default)]
    pub(crate) qrcode_img_content: String,
    #[serde(default)]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) last_error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcSyncContactsResult {
    pub(crate) channel_id: String,
    pub(crate) synced_count: usize,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WeixinOcCredentials {
    #[serde(default)]
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) cdn_base_url: String,
    #[serde(default)]
    pub(crate) bot_type: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) qr_poll_interval: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) long_poll_timeout_ms: Option<u64>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) api_timeout_ms: Option<u64>,
    #[serde(default)]
    pub(crate) token: String,
    #[serde(default)]
    pub(crate) account_id: String,
    #[serde(default)]
    pub(crate) user_id: String,
    #[serde(default)]
    pub(crate) sync_buf: String,
}

impl WeixinOcCredentials {
    pub(crate) fn from_value(value: &Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    pub(crate) fn normalized_base_url(&self) -> String {
        let base = self.base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            WEIXIN_OC_DEFAULT_BASE_URL.to_string()
        } else {
            base.to_string()
        }
    }

    pub(crate) fn normalized_cdn_base_url(&self) -> String {
        let base = self.cdn_base_url.trim().trim_end_matches('/');
        if base.is_empty() {
            WEIXIN_OC_DEFAULT_CDN_BASE_URL.to_string()
        } else {
            base.to_string()
        }
    }

    pub(crate) fn normalized_bot_type(&self) -> String {
        let out = self.bot_type.trim();
        if out.is_empty() {
            WEIXIN_OC_DEFAULT_BOT_TYPE.to_string()
        } else {
            out.to_string()
        }
    }

    pub(crate) fn normalized_long_poll_timeout_ms(&self) -> u64 {
        self.long_poll_timeout_ms
            .unwrap_or(WEIXIN_OC_DEFAULT_LONG_POLL_TIMEOUT_MS)
            .clamp(5_000, 60_000)
    }

    pub(crate) fn normalized_api_timeout_ms(&self) -> u64 {
        self.api_timeout_ms
            .unwrap_or(WEIXIN_OC_DEFAULT_API_TIMEOUT_MS)
            .clamp(5_000, 60_000)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcGetBotQrCodeResp {
    pub(crate) qrcode: Option<String>,
    pub(crate) qrcode_img_content: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcQrStatusResp {
    pub(crate) ret: Option<i64>,
    pub(crate) errcode: Option<i64>,
    pub(crate) errmsg: Option<String>,
    pub(crate) status: Option<String>,
    #[serde(alias = "botToken")]
    pub(crate) bot_token: Option<String>,
    #[serde(alias = "ilinkBotId")]
    pub(crate) ilink_bot_id: Option<String>,
    #[serde(alias = "ilinkUserId")]
    pub(crate) ilink_user_id: Option<String>,
    #[serde(alias = "baseUrl")]
    pub(crate) baseurl: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcGetUpdatesResp {
    pub(crate) ret: Option<i64>,
    pub(crate) errcode: Option<i64>,
    pub(crate) errmsg: Option<String>,
    pub(crate) msgs: Option<Vec<WeixinOcInboundMessage>>,
    pub(crate) get_updates_buf: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcInboundMessage {
    pub(crate) message_id: Option<Value>,
    pub(crate) msg_id: Option<Value>,
    pub(crate) from_user_id: Option<String>,
    pub(crate) context_token: Option<String>,
    pub(crate) item_list: Option<Vec<WeixinOcMessageItem>>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcMessageItem {
    #[serde(rename = "type")]
    pub(crate) item_type: Option<i64>,
    pub(crate) text_item: Option<WeixinOcTextItem>,
    pub(crate) image_item: Option<WeixinOcImageItem>,
    pub(crate) voice_item: Option<WeixinOcVoiceItem>,
    pub(crate) file_item: Option<WeixinOcFileItem>,
    pub(crate) video_item: Option<WeixinOcVideoItem>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcTextItem {
    pub(crate) text: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcImageItem {
    pub(crate) media: Option<WeixinOcMediaPayload>,
    pub(crate) aeskey: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcMediaPayload {
    pub(crate) encrypt_query_param: Option<String>,
    pub(crate) aes_key: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcVoiceItem {
    pub(crate) media: Option<WeixinOcMediaPayload>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcFileItem {
    pub(crate) media: Option<WeixinOcMediaPayload>,
    pub(crate) file_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct WeixinOcVideoItem {
    pub(crate) media: Option<WeixinOcMediaPayload>,
}

#[derive(Debug, Clone)]
pub(crate) struct WeixinOcCollectedMedia {
    pub(crate) parts: Vec<ChatIngressPart>,
}

#[derive(Debug, Clone)]
pub(crate) struct WeixinOcRuntimeState {
    pub(crate) connected: bool,
    pub(crate) connected_at: Option<chrono::DateTime<chrono::Utc>>,
    pub(crate) base_url: String,
    pub(crate) account_id: String,
    pub(crate) user_id: String,
    pub(crate) session_key: String,
    pub(crate) qrcode: String,
    pub(crate) qrcode_img_content: String,
    pub(crate) login_status: String,
    pub(crate) last_error: String,
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

pub(crate) const WEIXIN_OC_TYPING_TICKET_TTL_SECS: u64 = 60;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub(crate) struct WeixinOcTypingTicketState {
    pub(crate) ilink_user_id: String,
    pub(crate) typing_ticket: String,
    pub(crate) ticket_context_token: String,
    pub(crate) ticket_refresh_after: std::time::Instant,
}

#[derive(Debug)]
pub(crate) struct WeixinOcTypingState {
    pub(crate) ticket_state: WeixinOcTypingTicketState,
    pub(crate) cancel_tx: tokio::sync::oneshot::Sender<()>,
}

pub struct WeixinOcManager {
    states: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcRuntimeState>>,
    >,
    login_sessions: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcLoginSession>>,
    >,
    stop_senders: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, tokio::sync::watch::Sender<bool>>>,
    >,
    tasks: std::sync::Arc<
        tokio::sync::RwLock<
            std::collections::HashMap<String, tokio::task::JoinHandle<()>>,
        >,
    >,
    context_tokens: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, String>>,
    >,
    typing_states: std::sync::Arc<
        tokio::sync::RwLock<std::collections::HashMap<String, WeixinOcTypingState>>,
    >,
    port_service: std::sync::Arc<LocalPortServiceCore>,
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

pub(crate) static WEIXIN_OC_MANAGER: once_cell::sync::Lazy<std::sync::Arc<WeixinOcManager>> =
    once_cell::sync::Lazy::new(|| std::sync::Arc::new(WeixinOcManager::new()));

pub(crate) fn weixin_oc_manager() -> std::sync::Arc<WeixinOcManager> {
    WEIXIN_OC_MANAGER.clone()
}

pub(crate) fn login_session_is_fresh(login: &WeixinOcLoginSession) -> bool {
    chrono::DateTime::parse_from_rfc3339(login.started_at.trim())
        .map(|ts| {
            chrono::Utc::now()
                .signed_duration_since(ts.with_timezone(&chrono::Utc))
                .num_seconds()
                < WEIXIN_OC_LOGIN_TTL_SECS
        })
        .unwrap_or(false)
}
