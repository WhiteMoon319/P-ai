use std::{
    fs,
    io::Cursor,
    path::PathBuf,
    sync::{Arc, Mutex, OnceLock},
};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use directories::ProjectDirs;
use futures_util::{future::AbortHandle, future::join_all, future::BoxFuture, StreamExt};
use image::ImageFormat;
use reqwest::header::{HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use rmcp::{schemars, ServiceExt};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};
use uuid::Uuid;

// Android 下 updater.rs / xcap_screenshot.rs 被 stub 替换，其头部 use 需在此补齐

use std::collections::{HashMap};
use super::*;

pub(crate) static DETACHED_CHAT_WINDOWS: OnceLock<Mutex<std::collections::HashMap<String, String>>> =
    OnceLock::new();

pub(crate) fn detached_chat_windows() -> &'static Mutex<std::collections::HashMap<String, String>> {
    DETACHED_CHAT_WINDOWS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

pub(crate) fn detached_chat_window_for_conversation(conversation_id: &str) -> Option<String> {
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return None;
    }
    let guard = detached_chat_windows().lock().unwrap_or_else(|poison| {
        runtime_log_info(format!(
            "[独立聊天窗口] 会话到窗口映射锁已中毒，继续恢复读取：error={:?}",
            poison
        ));
        poison.into_inner()
    });
    guard.get(cid).cloned()
}

pub(crate) fn register_detached_chat_window(conversation_id: &str, label: &str) -> Result<(), String> {
    let cid = conversation_id.trim();
    let window_label = label.trim();
    if cid.is_empty() || window_label.is_empty() {
        return Err("conversationId 和 windowLabel 不能为空".to_string());
    }
    let mut guard = detached_chat_windows()
        .lock()
        .map_err(|err| format!("锁定独立聊天窗口映射失败：{err}"))?;
    guard.insert(cid.to_string(), window_label.to_string());
    Ok(())
}

pub(crate) fn unregister_detached_chat_window_by_label(label: &str) -> Option<String> {
    let window_label = label.trim();
    if window_label.is_empty() {
        return None;
    }
    let mut guard = detached_chat_windows().lock().ok()?;
    let conversation_id = guard
        .iter()
        .find_map(|(conversation_id, mapped_label)| {
            if mapped_label == window_label {
                Some(conversation_id.clone())
            } else {
                None
            }
        })?;
    guard.remove(&conversation_id);
    Some(conversation_id)
}

#[cfg(target_os = "android")]
pub(crate) fn toggle_window_maximize_with_default_restore(
    _app: &NativeAppHandle,
    _label: &str,
) -> Result<bool, String> {
    Ok(false)
}

#[cfg(target_os = "android")]
pub(crate) fn start_window_drag_with_default_restore(_app: &NativeAppHandle, _label: &str) -> Result<(), String> {
    Ok(())
}

pub(crate) fn default_hotkey_label() -> String {
    "Alt+·".to_string()
}

pub(crate) fn normalize_hotkey_label(value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return default_hotkey_label();
    }
    let normalized = raw.replace('＋', "+").replace('`', "·");
    let upper = normalized.to_uppercase();
    if upper.contains("BACKQUOTE") {
        return normalized
            .replace("Backquote", "·")
            .replace("BACKQUOTE", "·")
            .replace("backquote", "·");
    }
    normalized
}

pub(crate) fn ensure_hotkey_config_normalized(config: &mut AppConfig) {
    config.hotkey = normalize_hotkey_label(&config.hotkey);
    if config.hotkey.trim().is_empty() {
        config.hotkey = default_hotkey_label();
    }
}
