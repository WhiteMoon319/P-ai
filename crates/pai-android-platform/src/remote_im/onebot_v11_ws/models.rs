use super::*;
// OneBot v11 反向 WebSocket 服务器
// 实现 OneBot v11 协议的反向 WebSocket 连接（基于 axum WebSocket）

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

// 以下 import 供 ide_context.rs 等其他 include! 文件使用（它们仍用 tokio-tungstenite 做独立 WS 服务器）
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

/// OneBot v11 凭证配置
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnebotV11WsCredentials {
    #[serde(default = "default_ws_host")]
    pub ws_host: String,
    #[serde(default = "default_ws_port")]
    pub ws_port: u16,
    #[serde(default)]
    pub ws_token: Option<String>,
}

pub fn default_ws_host() -> String {
    "0.0.0.0".to_string()
}

pub fn default_ws_port() -> u16 {
    6199
}

pub const NAPCAT_RECONNECT_INTERVAL_SECS: u64 = 30;
pub const NAPCAT_MAX_MEDIA_DOWNLOAD_SIZE_BYTES: u64 = 20 * 1024 * 1024;
pub const NAPCAT_ACTIVE_CONNECTION_REPLACE_TIMEOUT_MS: u64 = 1500;

impl OnebotV11WsCredentials {
    pub fn from_credentials(credentials: &Value) -> Self {
        serde_json::from_value(credentials.clone()).unwrap_or_default()
    }
}

impl Default for OnebotV11WsCredentials {
    fn default() -> Self {
        Self {
            ws_host: default_ws_host(),
            ws_port: default_ws_port(),
            ws_token: None,
        }
    }
}

/// OneBot v11 API 请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotApiRequest {
    pub action: String,
    pub params: Value,
    #[serde(default)]
    pub echo: Option<Value>,
}

/// OneBot v11 API 响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OneBotApiResponse {
    pub status: String,
    pub retcode: i64,
    pub data: Value,
    #[serde(default)]
    pub echo: Option<Value>,
}

/// WebSocket 连接信息
pub struct WsConnection {
    /// 发送请求的通道
    pub tx: broadcast::Sender<String>,
    /// 等待响应的 oneshot 映射: echo -> sender
    pub pending_responses: Arc<RwLock<HashMap<String, oneshot::Sender<OneBotApiResponse>>>>,
    /// 连接的对端地址
    pub peer_addr: Option<String>,
    /// 连接时间
    pub connected_at: Option<DateTime<Utc>>,
}

#[derive(Clone)]
pub struct OnebotChannelRuntime {
    #[allow(dead_code)] // 用于测试中的 runtime 匹配验证
    pub id: String,
    pub cancel: CancellationToken,
    pub tasks: TaskTracker,
}

/// axum WebSocket handler 所需的共享状态
#[derive(Clone)]
pub struct OnebotAxumState {
    pub channel_id: String,
    pub expected_token: Option<String>,
    pub conn_tx: broadcast::Sender<String>,
    pub pending_responses: Arc<RwLock<HashMap<String, oneshot::Sender<OneBotApiResponse>>>>,
    pub event_tx: broadcast::Sender<Value>,
    pub connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    pub connection_stop_senders: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    pub port_service: Arc<LocalPortServiceCore>,
    pub active_connection_gate: Arc<std::sync::atomic::AtomicBool>,
    pub cancel: CancellationToken,
}

/// OneBot v11 WebSocket 服务器管理器
#[derive(Clone)]
pub struct OnebotV11WsManager {
    /// 活跃连接: channel_id -> 连接信息
    pub connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    /// 活跃连接停止信号: channel_id -> stop sender
    pub connection_stop_senders: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    /// 每个渠道独立的事件总线: channel_id -> event sender
    pub channel_event_senders: Arc<RwLock<HashMap<String, broadcast::Sender<Value>>>>,
    /// 渠道本地端口服务共享状态
    pub port_service: Arc<LocalPortServiceCore>,
    /// 渠道 axum serve 任务的 JoinHandle
    pub channel_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
    /// 渠道派生任务组，用于 stop 时收割所有连接任务
    pub channel_runtimes: Arc<RwLock<HashMap<String, OnebotChannelRuntime>>>,
    /// OneBot 事件消费器停止信号: channel_id -> stop sender
    pub event_consumer_stop_senders: Arc<RwLock<HashMap<String, watch::Sender<bool>>>>,
    /// OneBot 事件消费器任务: channel_id -> JoinHandle
    pub event_consumer_tasks: Arc<RwLock<HashMap<String, tokio::task::JoinHandle<()>>>>,
}

pub type AxumWsSender = SplitSink<WebSocket, AxumWsMessage>;
pub type AxumWsReceiver = SplitStream<WebSocket>;
