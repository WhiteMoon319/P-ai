// weixin_oc 域已整体迁入 pai-android-platform（core/api/media/runtime/login/inbound 纯逻辑部分）。
// 本文件作为 src-tauri 桥接：
// 1. re-export platform 的 weixin_oc 符号；
// 2. 保留 3 个仍依赖 AppState 的函数（upsert_weixin_oc_contact /
//    sync_weixin_oc_contact_from_user_id / remote_im_weixin_oc_sync_contacts_inner）。

pub(crate) use pai_android_platform::remote_im::weixin_oc::*;

use super::*;

pub(crate) fn upsert_weixin_oc_contact(
    runtime: &mut RuntimeStateFile,
    channel: &RemoteImChannelConfig,
    user_id: &str,
) -> (String, bool) {
    let normalized_user_id = user_id.trim();
    let display_name = weixin_oc_contact_display_name(channel, normalized_user_id);
    if let Some(contact) = runtime.remote_im_contacts.iter_mut().find(|item| {
        item.channel_id == channel.id
            && item.remote_contact_type == "private"
            && item.remote_contact_id == normalized_user_id
    }) {
        let current_name = contact.remote_contact_name.trim();
        if current_name.is_empty() || current_name == normalized_user_id {
            contact.remote_contact_name = display_name;
        }
        return (contact.id.clone(), false);
    }

    let contact_id = Uuid::new_v4().to_string();
    runtime.remote_im_contacts.push(RemoteImContact {
        id: contact_id.clone(),
        channel_id: channel.id.clone(),
        platform: RemoteImPlatform::WeixinOc,
        remote_contact_type: "private".to_string(),
        remote_contact_id: normalized_user_id.to_string(),
        remote_contact_name: display_name,
        avatar_url: String::new(),
        remark_name: String::new(),
        allow_send: true,
        allow_send_files: false,
        allow_receive: true,
        activation_mode: "never".to_string(),
        activation_keywords: Vec::new(),
        mute_keywords: default_remote_im_contact_mute_keywords(),
        unmute_keywords: default_remote_im_contact_unmute_keywords(),
        patience_seconds: default_remote_im_contact_patience_seconds(),
        mute_duration_seconds: default_remote_im_contact_mute_duration_seconds(),
        activation_cooldown_seconds: 0,
        route_mode: "dedicated_contact_conversation".to_string(),
        bound_department_id: None,
        bound_agent_id: None,
        bound_conversation_id: None,
        processing_mode: "continuous".to_string(),
        response_strategy: default_remote_im_contact_response_strategy(),
        response_guidance: default_remote_im_contact_response_guidance(),
        blocked_message_prefixes: default_remote_im_contact_blocked_message_prefixes(),
        group_reply_pacing: RemoteImGroupReplyPacing::default(),
        last_activated_at: None,
        last_message_at: None,
        dingtalk_session_webhook: None,
        dingtalk_session_webhook_expired_time: None,
        onebot_group_members: Vec::new(),
        shell_workspaces: Vec::new(),
    });
    (contact_id, true)
}

pub(crate) fn sync_weixin_oc_contact_from_user_id(
    state: &AppState,
    channel: &RemoteImChannelConfig,
    user_id: &str,
) -> Result<(String, bool), String> {
    let normalized_user_id = user_id.trim();
    if normalized_user_id.is_empty() {
        return Err("当前登录状态没有返回联系人 user_id，暂时无法补录联系人".to_string());
    }
    state_mutate_runtime_state_cached(state, |runtime| {
        Ok(upsert_weixin_oc_contact(
            runtime,
            channel,
            normalized_user_id,
        ))
    })
}

pub(crate) fn remote_im_weixin_oc_sync_contacts_inner(
    state: &AppState,
    input: WeixinOcLoginStatusInput,
) -> Result<WeixinOcSyncContactsResult, String> {
    let config = state_read_config_cached(state)?;
    let channel = remote_im_channel_by_id(&config, &input.channel_id)
        .ok_or_else(|| format!("渠道不存在: {}", input.channel_id))?;
    if channel.platform != RemoteImPlatform::WeixinOc {
        return Err("该渠道不是个人微信渠道".to_string());
    }
    let credentials = remote_im_effective_credentials(state, channel)?;
    let creds = WeixinOcCredentials::from_value(&credentials);
    if creds.account_id.trim().is_empty() || creds.token.trim().is_empty() {
        return Ok(WeixinOcSyncContactsResult {
            channel_id: input.channel_id,
            synced_count: 0,
            message: "当前还没有完成扫码登录，请先登录后再同步联系人。".to_string(),
        });
    }
    let user_id = creds.user_id.trim().to_string();
    let (_, created) = sync_weixin_oc_contact_from_user_id(state, channel, &user_id)?;
    Ok(WeixinOcSyncContactsResult {
        channel_id: input.channel_id,
        synced_count: 1,
        message: if created {
            format!("已同步个人微信联系人：{}", user_id)
        } else {
            format!("联系人已存在，无需重复同步：{}", user_id)
        },
    })
}
