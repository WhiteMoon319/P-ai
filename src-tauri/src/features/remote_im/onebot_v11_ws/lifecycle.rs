impl OnebotV11WsManager {
    /// 订阅事件流
    pub async fn subscribe_events(&self, channel_id: &str) -> Option<broadcast::Receiver<Value>> {
        let connections = self.connections.read().await;
        connections.get(channel_id).map(|c| c.event_tx.subscribe())
    }

    /// 关闭所有服务器
    pub async fn shutdown(&self) {
        let shutdowns = self.channel_shutdowns.write().await.drain().collect::<Vec<_>>();
        for (_, tx) in shutdowns {
            let _ = tx.send(());
        }
    }

    /// 检查渠道是否已连接
    pub async fn is_connected(&self, channel_id: &str) -> bool {
        self.connections.read().await.contains_key(channel_id)
    }

    /// 订阅渠道的关闭信号
    pub(crate) async fn subscribe_shutdown(&self, channel_id: &str) -> Option<broadcast::Receiver<()>> {
        let shutdowns = self.channel_shutdowns.read().await;
        shutdowns.get(channel_id).map(|tx| tx.subscribe())
    }
}

impl Default for OnebotV11WsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局 OneBot v11 WebSocket 管理器
static ONEBOT_V11_WS_MANAGER: once_cell::sync::Lazy<Arc<OnebotV11WsManager>> =
    once_cell::sync::Lazy::new(|| Arc::new(OnebotV11WsManager::new()));

pub fn onebot_v11_ws_manager() -> Arc<OnebotV11WsManager> {
    ONEBOT_V11_WS_MANAGER.clone()
}

/// 启动 OneBot v11 WebSocket 服务器
pub(crate) async fn onebot_v11_ws_server_start(
    channel: RemoteImChannelConfig,
) -> Result<(), String> {
    let credentials = OnebotV11WsCredentials::from_credentials(&channel.credentials);
    let manager = onebot_v11_ws_manager();
    let channel_id = channel.id.clone();
    manager.start(channel_id, credentials).await
}

/// 获取渠道连接状态
pub async fn get_channel_connection_status(channel_id: String) -> Result<ChannelConnectionStatus, String> {
    let manager = onebot_v11_ws_manager();
    Ok(manager.get_connection_status(&channel_id).await)
}

/// 获取渠道日志
pub async fn get_channel_logs(channel_id: String) -> Result<Vec<ChannelLogEntry>, String> {
    let manager = onebot_v11_ws_manager();
    Ok(manager.get_logs(&channel_id).await)
}

