use once_cell::sync::Lazy;

pub struct DingtalkStreamManager;

impl DingtalkStreamManager {
    pub fn new() -> Self {
        Self
    }

    pub(crate) async fn add_log(&self, _channel_id: &str, _level: &str, _message: &str) {}

    pub(crate) async fn add_contact_log(
        &self,
        _channel_id: &str,
        _level: &str,
        _message: &str,
        _contact_record_id: &str,
    ) {
    }

    pub(crate) async fn stop_channel(&self, _channel_id: &str) {}

    pub(crate) async fn reconcile_channel_runtime(
        &self,
        _channel: &RemoteImChannelConfig,
        _state: AppState,
    ) -> Result<(), String> {
        Ok(())
    }

    pub(crate) async fn start_channel(
        &self,
        _channel: RemoteImChannelConfig,
        _state: AppState,
    ) -> Result<(), String> {
        Ok(())
    }

    pub(crate) async fn get_channel_status(&self, channel_id: &str) -> ChannelConnectionStatus {
        ChannelConnectionStatus {
            channel_id: channel_id.to_string(),
            connected: false,
            peer_addr: None,
            connected_at: None,
            listen_addr: String::new(),
            status_text: Some("Android 版未启用钉钉 Stream".to_string()),
            last_error: Some("Android 版未启用钉钉 Stream".to_string()),
            account_id: None,
            base_url: None,
            login_session_key: None,
            qrcode_url: None,
        }
    }

    pub(crate) async fn get_logs(&self, _channel_id: &str) -> Vec<ChannelLogEntry> {
        Vec::new()
    }
}

impl Default for DingtalkStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

static DINGTALK_STREAM_MANAGER: Lazy<Arc<DingtalkStreamManager>> =
    Lazy::new(|| Arc::new(DingtalkStreamManager::new()));

pub fn dingtalk_stream_manager() -> Arc<DingtalkStreamManager> {
    DINGTALK_STREAM_MANAGER.clone()
}
