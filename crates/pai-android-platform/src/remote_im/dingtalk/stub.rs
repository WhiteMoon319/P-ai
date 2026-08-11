use once_cell::sync::Lazy;
use std::sync::Arc;

use pai_backend::core::domain::types_config::RemoteImChannelConfig;
use pai_backend::core::domain::types_storage::ChannelConnectionStatus;

use crate::local_port_service::ChannelLogEntry;

pub struct DingtalkStreamManager;

impl DingtalkStreamManager {
    pub fn new() -> Self {
        Self
    }

    pub async fn add_log(&self, _channel_id: &str, _level: &str, _message: &str) {}

    pub async fn add_contact_log(
        &self,
        _channel_id: &str,
        _level: &str,
        _message: &str,
        _contact_record_id: &str,
    ) {
    }

    pub async fn stop_channel(&self, _channel_id: &str) {}

    pub async fn reconcile_channel_runtime(
        &self,
        _channel: &RemoteImChannelConfig,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn start_channel(
        &self,
        _channel: RemoteImChannelConfig,
    ) -> Result<(), String> {
        Ok(())
    }

    pub async fn get_channel_status(&self, channel_id: &str) -> ChannelConnectionStatus {
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

    pub async fn get_logs(&self, _channel_id: &str) -> Vec<ChannelLogEntry> {
        Vec::new()
    }
}

impl Default for DingtalkStreamManager {
    fn default() -> Self {
        Self::new()
    }
}

pub static DINGTALK_STREAM_MANAGER: Lazy<Arc<DingtalkStreamManager>> =
    Lazy::new(|| Arc::new(DingtalkStreamManager::new()));

pub fn dingtalk_stream_manager() -> Arc<DingtalkStreamManager> {
    DINGTALK_STREAM_MANAGER.clone()
}
