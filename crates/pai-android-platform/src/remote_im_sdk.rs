//! 远程 IM SDK 基础设施（纯逻辑，无平台依赖）。

use serde_json::Value;

use pai_backend::core::domain::types_config::{RemoteImChannelConfig, RemoteImPlatform};
use pai_backend::core::domain::types_storage::RemoteImContact;

/// SDK 发送错误类型（从 src-tauri remote_im_adapters.rs 迁入）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemoteImSdkSendErrorKind {
    DefinitelyNotSent,
    Uncertain,
}

#[derive(Debug, Clone)]
pub struct RemoteImSdkSendError {
    pub kind: RemoteImSdkSendErrorKind,
    pub message: String,
}

impl RemoteImSdkSendError {
    pub fn definitely_not_sent(message: impl Into<String>) -> Self {
        Self {
            kind: RemoteImSdkSendErrorKind::DefinitelyNotSent,
            message: message.into(),
        }
    }

    pub fn uncertain(message: impl Into<String>) -> Self {
        Self {
            kind: RemoteImSdkSendErrorKind::Uncertain,
            message: message.into(),
        }
    }

    pub fn after_confirmed_partial_delivery(mut self, delivered_any: bool) -> Self {
        if delivered_any {
            self.kind = RemoteImSdkSendErrorKind::Uncertain;
        }
        self
    }

    pub fn is_definitely_not_sent(&self) -> bool {
        self.kind == RemoteImSdkSendErrorKind::DefinitelyNotSent
    }
}

/// HTTP 拒绝转 SDK 错误（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_http_rejection_error(
    status: reqwest::StatusCode,
    message: impl Into<String>,
) -> RemoteImSdkSendError {
    let message = message.into();
    if status == reqwest::StatusCode::REQUEST_TIMEOUT || status.is_server_error() {
        RemoteImSdkSendError::uncertain(message)
    } else {
        RemoteImSdkSendError::definitely_not_sent(message)
    }
}

/// 内容项名称（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_content_item_name(item: &Value, default_name: &str) -> String {
    item.get("name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(default_name)
        .to_string()
}

/// 内容项 MIME（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_content_item_mime(item: &Value, default_mime: &str) -> String {
    item.get("mime")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(default_mime)
        .to_string()
}

/// 内容项字节（从 src-tauri remote_im_adapters.rs 迁入）。
pub async fn remote_im_content_item_bytes(item: &Value) -> Result<Vec<u8>, String> {
    if let Some(b64) = item
        .get("bytesBase64")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|v| !v.is_empty())
    {
        use base64::Engine as _;
        return base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|err| format!("解析内容项 bytesBase64 失败: {err}"));
    }
    Err("内容项缺少 bytesBase64".to_string())
}

/// 凭据文本（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_credential_text(credentials: &Value, key: &str) -> String {
    credentials
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("")
        .to_string()
}

/// 是否群聊联系人（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_is_group_contact(contact: &RemoteImContact) -> bool {
    contact.remote_contact_type.trim().eq_ignore_ascii_case("group")
}

/// payload 内容项列表（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_payload_content_items(payload: &Value) -> Vec<Value> {
    payload
        .get("content")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// payload 媒体摘要（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_payload_media_summary(payload: &Value) -> Value {
    let items = remote_im_payload_content_items(payload);
    let mut text_count = 0usize;
    let mut image_count = 0usize;
    let mut file_count = 0usize;
    let mut unknown_count = 0usize;
    let mut image_mimes = Vec::<String>::new();
    let mut file_names = Vec::<String>::new();
    for item in items {
        match item.get("type").and_then(Value::as_str).unwrap_or("") {
            "text" => text_count += 1,
            "image" => {
                image_count += 1;
                image_mimes.push(remote_im_content_item_mime(&item, "image"));
            }
            "file" | "audio" | "video" => {
                file_count += 1;
                file_names.push(remote_im_content_item_name(&item, "file"));
            }
            _ => unknown_count += 1,
        }
    }
    serde_json::json!({
        "textCount": text_count,
        "imageCount": image_count,
        "fileCount": file_count,
        "unknownCount": unknown_count,
        "imageMimes": image_mimes,
        "fileNames": file_names,
    })
}

/// SDK trait（从 src-tauri remote_im_adapters.rs 迁入）。
pub trait RemoteImSdk: Send + Sync {
    fn platform(&self) -> RemoteImPlatform;
    fn validate_channel(&self, channel: &RemoteImChannelConfig) -> Result<(), String>;
    fn send_outbound<'a>(
        &'a self,
        channel: &'a RemoteImChannelConfig,
        contact: &'a RemoteImContact,
        payload: &'a Value,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, RemoteImSdkSendError>> + Send + 'a>,
    >;
}

/// 远程 IM 日志（从 src-tauri remote_im_adapters.rs 迁入）。
pub fn remote_im_log(level: &str, event: &str, fields: Value) {
    pai_backend::logging::runtime_log_info(format!(
        "{}",
        serde_json::json!({
            "level": level,
            "event": event,
            "fields": fields
        })
        .to_string()
    ));
}
