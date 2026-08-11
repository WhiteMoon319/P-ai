// ==================== OnebotV11StateAccess ====================
// onebot_v11_ws 域对 AppState 的访问抽象。src-tauri 侧为 AppState 实现本 trait，
// platform 侧代码只依赖本 trait，不再直接接触 AppState。
// 对象安全设计：方法全部 &self、无泛型，以便在 tokio::spawn 中以
// Arc<dyn OnebotV11StateAccess> 传递。

use pai_backend::core::domain::types_config::RemoteImChannelConfig;
use pai_backend::core::domain::types_requests::{RemoteImEnqueueInput, RemoteImEnqueueResult};
use pai_backend::core::domain::types_storage::RemoteImGroupMemberInfo;
use std::collections::HashMap;

pub trait OnebotV11StateAccess: Send + Sync {
    fn read_channel_config(
        &self,
        channel_id: &str,
    ) -> Result<Option<RemoteImChannelConfig>, String>;
    fn group_member_cache_for_contact(
        &self,
        channel_id: &str,
        group_id: Option<u64>,
    ) -> HashMap<String, RemoteImGroupMemberInfo>;
    fn persist_group_member_cache(
        &self,
        contact_id: &str,
        members: Vec<RemoteImGroupMemberInfo>,
    ) -> Result<(), String>;
    fn enqueue_message(
        &self,
        input: RemoteImEnqueueInput,
    ) -> Result<RemoteImEnqueueResult, String>;
}
