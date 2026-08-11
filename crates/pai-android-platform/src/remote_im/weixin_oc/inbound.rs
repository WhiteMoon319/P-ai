use pai_backend::core::domain::types_config::{AppConfig, RemoteImChannelConfig, RemoteImPlatform};
use pai_backend::core::domain::types_requests::{ChatIngressPart, ChatInputPayload, SessionSelector};
use pai_backend::core::domain::types_storage::RemoteImChannelPrivateState;
use pai_backend::logging::{runtime_log_debug, runtime_log_error, runtime_log_info, runtime_log_warn};
use serde_json::Value;
use uuid::Uuid;

use super::*;
use crate::remote_im_sdk::{RemoteImSdkSendError, remote_im_http_rejection_error};

pub fn weixin_oc_contact_display_name(
    channel: &RemoteImChannelConfig,
    user_id: &str,
) -> String {
    let channel_name = channel.name.trim();
    if !channel_name.is_empty() {
        return channel_name.to_string();
    }
    let normalized_user_id = user_id.trim();
    if !normalized_user_id.is_empty() {
        return normalized_user_id.to_string();
    }
    "个人微信".to_string()
}

pub async fn handle_weixin_oc_inbound_message(
    channel: &RemoteImChannelConfig,
    access: &dyn WeixinOcStateAccess,
    msg: WeixinOcInboundMessage,
) -> Result<(), String> {
    let from_user_id = msg
        .from_user_id
        .as_deref()
        .map(str::trim)
        .unwrap_or("");
    if from_user_id.is_empty() {
        return Ok(());
    }
    if let Some(token) = msg
        .context_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        weixin_oc_manager()
            .set_context_token(access, &channel.id, from_user_id, token)
            .await;
    }
    let item_list = msg.item_list.unwrap_or_default();
    let creds = WeixinOcCredentials::from_value(&channel.credentials);
    let media = match build_weixin_oc_http_client(creds.normalized_api_timeout_ms()) {
        Ok(client) => weixin_oc_collect_media(&client, &creds, &item_list).await,
        Err(err) => {
            runtime_log_warn(format!(
                "[远程IM][个人微信事件] 媒体客户端初始化失败，保留文本并跳过附件继续，error={err}"
            ));
            let mut parts = Vec::<ChatIngressPart>::new();
            for item in &item_list {
                if item.item_type.unwrap_or(0) == 1 {
                    if let Some(text) = item
                        .text_item
                        .as_ref()
                        .and_then(|value| value.text.as_deref())
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        parts.push(ChatIngressPart::Text {
                            text: text.to_string(),
                        });
                    }
                } else {
                    parts.push(ChatIngressPart::Text {
                        text: "[附件不可用：微信媒体处理暂不可用，已跳过附件并继续]".to_string(),
                    });
                }
            }
            WeixinOcCollectedMedia { parts }
        }
    };
    let final_text = media.parts.iter().filter_map(|part| match part {
        ChatIngressPart::Text { text } => Some(text.trim()),
        ChatIngressPart::Attachment { .. } => None,
    }).filter(|text| !text.is_empty()).collect::<Vec<_>>().join("\n");
    let display_name = weixin_oc_contact_display_name(channel, from_user_id);
    let message_id = msg
        .message_id
        .or(msg.msg_id)
        .map(|value| value.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    access.enqueue_message(
        pai_backend::core::domain::types_requests::RemoteImEnqueueInput {
            channel_id: channel.id.clone(),
            platform: RemoteImPlatform::WeixinOc,
            im_name: "weixin".to_string(),
            remote_contact_type: "private".to_string(),
            remote_contact_id: from_user_id.to_string(),
            remote_contact_name: Some(display_name.clone()),
            sender_id: from_user_id.to_string(),
            sender_name: display_name,
            sender_avatar_url: None,
            platform_message_id: Some(message_id),
            dingtalk_session_webhook: None,
            dingtalk_session_webhook_expired_time: None,
            session: pai_backend::core::domain::types_requests::SessionSelector {
                api_config_id: None,
                conversation_id: None,
                department_id: None,
                agent_id: String::new(),
            },
            payload: pai_backend::core::domain::types_requests::ChatInputPayload {
                text: if final_text.is_empty() {
                    None
                } else {
                    Some(final_text.clone())
                },
                display_text: if final_text.is_empty() {
                    None
                } else {
                    Some(final_text)
                },
                parts: if media.parts.is_empty() { None } else { Some(media.parts) },
                images: None,
                audios: None,
                attachments: None,
                model: None,
                extra_text_blocks: None,
                mentions: None,
                provider_meta: msg.context_token.map(|token| {
                    serde_json::json!({
                        "contextToken": token,
                    })
                }),
            },
        },
    )?;
    Ok(())
}

pub async fn run_single_weixin_oc_poll_cycle(
    channel_id: &str,
    access: std::sync::Arc<dyn WeixinOcStateAccess>,
) -> Result<(), String> {
    let config = access.read_config()?;
    let channel = config
        .remote_im_channels
        .iter()
        .find(|item| item.id == channel_id)
        .cloned()
        .ok_or_else(|| format!("个人微信渠道不存在: {channel_id}"))?;
    let channel = access.channel_with_effective_credentials(&channel)?;
    let creds = WeixinOcCredentials::from_value(&channel.credentials);
    let token = creds.token.trim().to_string();
    if token.is_empty() {
        return Err("缺少 token，请先扫码登录".to_string());
    }
    let body = serde_json::json!({
        "base_info": {
            "channel_version": "easy_call_ai"
        },
        "get_updates_buf": creds.sync_buf,
    });
    let body_text = serde_json::to_string(&body)
        .map_err(|err| format!("序列化 getupdates 请求失败: {err}"))?;
    let headers = weixin_oc_request_headers(&body_text, Some(&token))?;
    let client = build_weixin_oc_http_client(creds.normalized_long_poll_timeout_ms())?;
    let resp = client
        .post(format!(
            "{}/ilink/bot/getupdates",
            creds.normalized_base_url().trim_end_matches('/')
        ))
        .headers(headers)
        .body(body_text)
        .send()
        .await
        .map_err(|err| format!("请求 getupdates 失败: {err}"))?;
    let status_code = resp.status();
    if !status_code.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("请求 getupdates 失败: status={} body={}", status_code, text));
    }
    let data = resp
        .json::<WeixinOcGetUpdatesResp>()
        .await
        .map_err(|err| format!("解析 getupdates 响应失败: {err}"))?;
    if data.ret.unwrap_or(0) != 0 || data.errcode.unwrap_or(0) != 0 {
        return Err(format!(
            "getupdates 返回错误: ret={} errcode={} errmsg={}",
            data.ret.unwrap_or(0),
            data.errcode.unwrap_or(0),
            data.errmsg.unwrap_or_default()
        ));
    }
    if let Some(next_sync_buf) = data
        .get_updates_buf
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        if creds.sync_buf.trim() != next_sync_buf {
            let sync_buf_value = next_sync_buf.to_string();
            access.patch_private_state(
                &channel.id,
                Box::new(move |private| {
                    private.sync_buf = sync_buf_value;
                }),
            )?;
        }
    }
    for msg in data.msgs.unwrap_or_default() {
        handle_weixin_oc_inbound_message(&channel, access.as_ref(), msg).await?;
    }
    Ok(())
}

pub async fn weixin_oc_send_text_message(
    credentials: WeixinOcCredentials,
    to_user_id: &str,
    text: &str,
    context_token: Option<&str>,
) -> Result<String, RemoteImSdkSendError> {
    let item_list = vec![serde_json::json!({
        "type": WEIXIN_OC_TEXT_ITEM_TYPE,
        "text_item": {
            "text": text
        }
    })];
    weixin_oc_send_message_items(credentials, to_user_id, item_list, context_token).await
}

pub async fn weixin_oc_send_message_items(
    credentials: WeixinOcCredentials,
    to_user_id: &str,
    item_list: Vec<Value>,
    context_token: Option<&str>,
) -> Result<String, RemoteImSdkSendError> {
    let client = build_weixin_oc_http_client(credentials.normalized_api_timeout_ms())
        .map_err(RemoteImSdkSendError::definitely_not_sent)?;
    let client_id = Uuid::new_v4().simple().to_string();
    let body = serde_json::json!({
        "base_info": {
            "channel_version": "easy_call_ai"
        },
        "msg": {
            "from_user_id": "",
            "to_user_id": to_user_id,
            "client_id": client_id,
            "message_type": 2,
            "message_state": 2,
            "context_token": context_token.map(str::trim).filter(|value| !value.is_empty()),
            "item_list": item_list
        }
    });
    let body_text = serde_json::to_string(&body)
        .map_err(|err| {
            RemoteImSdkSendError::definitely_not_sent(format!(
                "序列化 sendmessage 请求失败: {err}"
            ))
        })?;
    let headers = weixin_oc_request_headers(&body_text, Some(credentials.token.as_str()))
        .map_err(RemoteImSdkSendError::definitely_not_sent)?;
    let resp = client
        .post(format!(
            "{}/ilink/bot/sendmessage",
            credentials.normalized_base_url().trim_end_matches('/')
        ))
        .headers(headers)
        .body(body_text)
        .send()
        .await
        .map_err(|err| {
            RemoteImSdkSendError::uncertain(format!("请求 sendmessage 失败: {err}"))
        })?;
    let status_code = resp.status();
    if !status_code.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(remote_im_http_rejection_error(
            status_code,
            format!(
                "请求 sendmessage 失败: status={} body={}",
                status_code, body
            ),
        ));
    }
    let resp_body: serde_json::Value = resp
        .json()
        .await
        .map_err(|err| {
            RemoteImSdkSendError::uncertain(format!(
                "解析 sendmessage 响应失败: {err}"
            ))
        })?;
    let ret = resp_body
        .get("ret")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    let errcode = resp_body
        .get("errcode")
        .and_then(serde_json::Value::as_i64)
        .unwrap_or(0);
    if ret != 0 || errcode != 0 {
        let errmsg = resp_body
            .get("errmsg")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        return Err(RemoteImSdkSendError::definitely_not_sent(format!(
            "请求 sendmessage 失败: ret={} errcode={} errmsg={} resp={}",
            ret, errcode, errmsg, resp_body
        )));
    }
    Ok(client_id)
}

