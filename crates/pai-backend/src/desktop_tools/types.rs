use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DesktopToolErrorCode {
    InvalidParams,
    Timeout,
    TargetNotFound,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DesktopToolError {
    pub code: DesktopToolErrorCode,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

impl DesktopToolError {
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: DesktopToolErrorCode::InvalidParams,
            message: message.into(),
            details: None,
        }
    }

    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: DesktopToolErrorCode::InternalError,
            message: message.into(),
            details: None,
        }
    }
}

pub type DesktopToolResult<T> = Result<T, DesktopToolError>;

pub fn to_tool_err_string(err: &DesktopToolError) -> String {
    serde_json::to_string(err).unwrap_or_else(|_| err.message.clone())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScreenshotMode {
    Desktop,
    Monitor,
    Region,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenBounds {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotRequest {
    #[serde(default = "default_screenshot_mode")]
    pub mode: ScreenshotMode,
    #[serde(default)]
    pub monitor_id: Option<u32>,
    #[serde(default)]
    pub region: Option<ScreenBounds>,
    #[serde(default)]
    pub save_path: Option<String>,
    #[serde(default = "default_webp_quality")]
    pub webp_quality: f32,
    #[serde(default = "default_include_screenshot_base64")]
    pub include_base64: bool,
}

pub fn default_screenshot_mode() -> ScreenshotMode {
    ScreenshotMode::Desktop
}

pub fn default_webp_quality() -> f32 {
    75.0
}

pub fn default_include_screenshot_base64() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreenshotResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub image_mime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_base64: Option<String>,
    pub width: u32,
    pub height: u32,
    pub bounds: ScreenBounds,
    pub elapsed_ms: u64,
    pub capture_ms: u64,
    pub encode_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub save_ms: Option<u64>,
    pub timestamp: String,
}
