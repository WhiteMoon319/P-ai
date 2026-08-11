use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::model_runtime::{ProviderToolDefinition, RuntimeToolDyn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotForwardImagePayload {
    pub mime: String,
    pub base64: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenshotForwardPayload {
    pub images: Vec<ScreenshotForwardImagePayload>,
}

#[derive(Debug, Clone)]
pub struct ScreenshotArtifactEntry {
    pub images: Vec<ScreenshotForwardImagePayload>,
    pub created_seq: u64,
}

pub const SCREENSHOT_ARTIFACT_MAX_ITEMS: usize = 24;

pub struct RuntimeToolAssembly {
    pub tools: Vec<Box<dyn RuntimeToolDyn>>,
    pub tool_definitions: Vec<ProviderToolDefinition>,
    pub tool_manifest: Vec<Value>,
    pub unavailable_tool_notices: Vec<String>,
}
