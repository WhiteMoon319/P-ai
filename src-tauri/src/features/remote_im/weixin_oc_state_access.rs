// ==================== WeixinOcStateAccess 实现（AppState） ====================
// weixin_oc 域迁入 pai-android-platform 后，AppState 侧通过本文件实现
// platform 定义的 WeixinOcStateAccess trait，把私有状态读写/配置读取/联系人
// 写入等操作桥接到 src-tauri 现有 channel_store / runtime_cache 基础设施。

use pai_android_platform::remote_im::weixin_oc::state_access::WeixinOcStateAccess;
use pai_backend::core::domain::types_config::{AppConfig, RemoteImChannelConfig};
use pai_backend::core::domain::types_requests::RemoteImEnqueueInput;
use pai_backend::core::domain::types_storage::RemoteImChannelPrivateState;
use serde_json::Value;

use super::*;

pub(crate) struct AppStateWeixinOcAccess {
    pub(crate) state: AppState,
}

impl AppStateWeixinOcAccess {
    pub(crate) fn new(state: &AppState) -> Self {
        Self {
            state: state.clone(),
        }
    }
}

impl WeixinOcStateAccess for AppStateWeixinOcAccess {
    fn read_config(&self) -> Result<AppConfig, String> {
        state_read_config_cached(&self.state)
    }

    fn read_private_state(&self, channel_id: &str) -> Result<RemoteImChannelPrivateState, String> {
        remote_im_read_channel_private_state(
            &self.state,
            &RemoteImPlatform::WeixinOc,
            channel_id,
        )
    }

    fn patch_private_state(
        &self,
        channel_id: &str,
        patch: Box<dyn FnOnce(&mut RemoteImChannelPrivateState) + Send>,
    ) -> Result<(), String> {
        remote_im_patch_channel_private_state(
            &self.state,
            &RemoteImPlatform::WeixinOc,
            channel_id,
            patch,
        )
        .map(|_| ())
    }

    fn delete_private_state(&self, channel_id: &str) -> Result<(), String> {
        remote_im_delete_channel_private_state(
            &self.state,
            &RemoteImPlatform::WeixinOc,
            channel_id,
        )
    }

    fn effective_credentials(&self, channel: &RemoteImChannelConfig) -> Result<Value, String> {
        remote_im_effective_credentials(&self.state, channel)
    }

    fn channel_with_effective_credentials(
        &self,
        channel: &RemoteImChannelConfig,
    ) -> Result<RemoteImChannelConfig, String> {
        remote_im_channel_with_effective_credentials(&self.state, channel)
    }

    fn upsert_contact(
        &self,
        channel: &RemoteImChannelConfig,
        user_id: &str,
    ) -> Result<(String, bool), String> {
        let normalized_user_id = user_id.trim();
        if normalized_user_id.is_empty() {
            return Err("当前登录状态没有返回联系人 user_id，暂时无法补录联系人".to_string());
        }
        state_mutate_runtime_state_cached(&self.state, |runtime| {
            Ok(upsert_weixin_oc_contact(
                runtime,
                channel,
                normalized_user_id,
            ))
        })
    }

    fn enqueue_message(&self, input: RemoteImEnqueueInput) -> Result<String, String> {
        remote_im_enqueue_message_internal(input, &self.state)
            .map(|result| result.event_id)
    }
}
