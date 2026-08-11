// ==================== WeixinOcStateAccess ====================
// weixin_oc 域对 AppState 的访问抽象。src-tauri 侧为 AppState 实现本 trait，
// platform 侧代码只依赖本 trait，不再直接接触 AppState。
// 对象安全设计：方法全部 &self、无泛型，patch 用 Box<dyn FnOnce>，
// 以便在 tokio::spawn 中以 Arc<dyn WeixinOcStateAccess> 传递。

use pai_backend::core::domain::types_config::{AppConfig, RemoteImChannelConfig};
use pai_backend::core::domain::types_storage::RemoteImChannelPrivateState;
use serde_json::Value;

pub trait WeixinOcStateAccess: Send + Sync {
    fn read_config(&self) -> Result<AppConfig, String>;
    fn read_private_state(&self, channel_id: &str) -> Result<RemoteImChannelPrivateState, String>;
    fn patch_private_state(
        &self,
        channel_id: &str,
        patch: Box<dyn FnOnce(&mut RemoteImChannelPrivateState) + Send>,
    ) -> Result<(), String>;
    fn delete_private_state(&self, channel_id: &str) -> Result<(), String>;
    fn effective_credentials(&self, channel: &RemoteImChannelConfig) -> Result<Value, String>;
    fn channel_with_effective_credentials(
        &self,
        channel: &RemoteImChannelConfig,
    ) -> Result<RemoteImChannelConfig, String>;
    fn upsert_contact(
        &self,
        channel: &RemoteImChannelConfig,
        user_id: &str,
    ) -> Result<(String, bool), String>;
    fn enqueue_message(
        &self,
        input: pai_backend::core::domain::types_requests::RemoteImEnqueueInput,
    ) -> Result<String, String>;
}
