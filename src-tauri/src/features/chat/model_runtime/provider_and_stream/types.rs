#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScreenshotForwardImagePayload {
    pub(crate) mime: String,
    pub(crate) base64: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ScreenshotForwardPayload {
    pub(crate) images: Vec<ScreenshotForwardImagePayload>,
}

#[derive(Debug, Clone)]
pub(crate) struct ScreenshotArtifactEntry {
    pub(crate) images: Vec<ScreenshotForwardImagePayload>,
    pub(crate) created_seq: u64,
}

pub(crate) const SCREENSHOT_ARTIFACT_MAX_ITEMS: usize = 24;

pub struct RuntimeToolAssembly {
    tools: Vec<Box<dyn RuntimeToolDyn>>,
    pub tool_definitions: Vec<ProviderToolDefinition>,
    tool_manifest: Vec<Value>,
    unavailable_tool_notices: Vec<String>,
}
