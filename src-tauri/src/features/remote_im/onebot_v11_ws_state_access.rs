// ==================== OnebotV11StateAccess 实现（AppState） ====================
// onebot_v11_ws 域迁入 pai-android-platform 后，AppState 侧通过本文件实现
// platform 定义的 OnebotV11StateAccess trait，把配置读取/群成员缓存/入队等
// 操作桥接到 src-tauri 现有 channel_store / runtime_cache 基础设施。

use pai_android_platform::remote_im::onebot_v11_ws::state_access::OnebotV11StateAccess;
use pai_backend::core::domain::types_config::RemoteImChannelConfig;
use pai_backend::core::domain::types_requests::{RemoteImEnqueueInput, RemoteImEnqueueResult};
use pai_backend::core::domain::types_storage::RemoteImGroupMemberInfo;

use super::*;

pub(crate) struct AppStateOnebotAccess {
    pub(crate) state: AppState,
}

impl AppStateOnebotAccess {
    pub(crate) fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl OnebotV11StateAccess for AppStateOnebotAccess {
    fn read_channel_config(
        &self,
        channel_id: &str,
    ) -> Result<Option<RemoteImChannelConfig>, String> {
        let config = state_read_config_cached(&self.state)?;
        Ok(remote_im_channel_by_id(&config, channel_id).cloned())
    }

    fn group_member_cache_for_contact(
        &self,
        channel_id: &str,
        group_id: Option<u64>,
    ) -> std::collections::HashMap<String, RemoteImGroupMemberInfo> {
        let Some(group_id) = group_id else {
            return std::collections::HashMap::new();
        };
        let group_id = group_id.to_string();
        let Ok(runtime) = state_read_runtime_state_cached(&self.state) else {
            return std::collections::HashMap::new();
        };
        let Some(contact) = runtime.remote_im_contacts.iter().find(|item| {
            item.channel_id == channel_id
                && item.remote_contact_type == "group"
                && item.remote_contact_id == group_id
        }) else {
            return std::collections::HashMap::new();
        };
        contact
            .onebot_group_members
            .iter()
            .filter(|item| !item.user_id.trim().is_empty())
            .map(|item| (item.user_id.trim().to_string(), item.clone()))
            .collect()
    }

    fn persist_group_member_cache(
        &self,
        contact_id: &str,
        members: Vec<RemoteImGroupMemberInfo>,
    ) -> Result<(), String> {
        if members.is_empty() {
            return Ok(());
        }
        state_mutate_runtime_state_cached(&self.state, |runtime| {
            let Some(contact) = runtime
                .remote_im_contacts
                .iter_mut()
                .find(|item| item.id == contact_id)
            else {
                return Ok(());
            };
            for member in members {
                let user_id = member.user_id.trim();
                if user_id.is_empty() {
                    continue;
                }
                if let Some(existing) = contact
                    .onebot_group_members
                    .iter_mut()
                    .find(|item| item.user_id.trim() == user_id)
                {
                    if existing != &member {
                        *existing = member;
                    }
                } else {
                    contact.onebot_group_members.push(member);
                }
            }
            Ok(())
        })
    }

    fn enqueue_message(
        &self,
        input: RemoteImEnqueueInput,
    ) -> Result<RemoteImEnqueueResult, String> {
        remote_im_enqueue_message_internal(input, &self.state)
    }
}
