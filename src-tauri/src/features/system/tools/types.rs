#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(crate) enum DesktopToolErrorCode {
    InvalidParams,
    Timeout,
    TargetNotFound,
    InternalError,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopToolError {
    pub(crate) code: DesktopToolErrorCode,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) details: Option<Value>,
}

impl DesktopToolError {
    pub(crate) fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: DesktopToolErrorCode::InvalidParams,
            message: message.into(),
            details: None,
        }
    }

    pub(crate) fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: DesktopToolErrorCode::InternalError,
            message: message.into(),
            details: None,
        }
    }
}

pub(crate) type DesktopToolResult<T> = Result<T, DesktopToolError>;

pub(crate) fn to_tool_err_string(err: &DesktopToolError) -> String {
    serde_json::to_string(err).unwrap_or_else(|_| err.message.clone())
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ScreenshotMode {
    Desktop,
    Monitor,
    Region,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenBounds {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenshotRequest {
    #[serde(default = "default_screenshot_mode")]
    pub(crate) mode: ScreenshotMode,
    #[serde(default)]
    pub(crate) monitor_id: Option<u32>,
    #[serde(default)]
    pub(crate) region: Option<ScreenBounds>,
    #[serde(default)]
    pub(crate) save_path: Option<String>,
    #[serde(default = "default_webp_quality")]
    pub(crate) webp_quality: f32,
    #[serde(default = "default_include_screenshot_base64")]
    pub(crate) include_base64: bool,
}

pub(crate) fn default_screenshot_mode() -> ScreenshotMode {
    ScreenshotMode::Desktop
}

pub(crate) fn default_webp_quality() -> f32 {
    75.0
}

pub(crate) fn default_include_screenshot_base64() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScreenshotResponse {
    pub(crate) ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) path: Option<String>,
    pub(crate) image_mime: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) image_base64: Option<String>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) bounds: ScreenBounds,
    pub(crate) elapsed_ms: u64,
    pub(crate) capture_ms: u64,
    pub(crate) encode_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) save_ms: Option<u64>,
    pub(crate) timestamp: String,
}
