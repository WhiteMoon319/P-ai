pub(crate) fn default_image_generation_count() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ImageGenerationOperation {
    #[default]
    Generate,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenerationRequest {
    pub(crate) prompt: String,
    #[serde(default)]
    pub(crate) operation: ImageGenerationOperation,
    #[serde(default, alias = "model_id")]
    pub(crate) model_id: Option<String>,
    #[serde(default, alias = "negative_prompt")]
    pub(crate) negative_prompt: Option<String>,
    #[serde(default)]
    pub(crate) size: Option<String>,
    #[serde(default, alias = "aspect_ratio")]
    pub(crate) aspect_ratio: Option<String>,
    #[serde(default)]
    pub(crate) quality: Option<String>,
    #[serde(default = "default_image_generation_count")]
    pub(crate) n: u32,
    #[serde(default)]
    pub(crate) seed: Option<i64>,
    #[serde(default)]
    pub(crate) steps: Option<u32>,
    #[serde(default)]
    pub(crate) images: Vec<String>,
    #[serde(default)]
    pub(crate) mask: Option<String>,
}

impl Default for ImageGenerationRequest {
    fn default() -> Self {
        Self {
            prompt: String::new(),
            operation: ImageGenerationOperation::Generate,
            model_id: None,
            negative_prompt: None,
            size: None,
            aspect_ratio: None,
            quality: None,
            n: default_image_generation_count(),
            seed: None,
            steps: None,
            images: Vec::new(),
            mask: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GeneratedImageAsset {
    pub(crate) relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) remote_url: Option<String>,
    pub(crate) markdown: String,
    pub(crate) mime: String,
    pub(crate) width: u32,
    pub(crate) height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenerationResult {
    pub(crate) provider_id: String,
    pub(crate) provider_name: String,
    pub(crate) provider_type: ImageGenerationProviderKind,
    pub(crate) model_id: String,
    pub(crate) model: String,
    pub(crate) images: Vec<GeneratedImageAsset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provider_text: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedImageGenerationModel {
    pub(crate) endpoint_id: String,
    pub(crate) provider: ImageGenerationProviderConfig,
    pub(crate) model: ImageGenerationModelConfig,
}

#[derive(Debug, Clone)]
pub(crate) enum PendingImageSource {
    Bytes(Vec<u8>),
    RemoteUrl(String),
}

#[derive(Debug, Clone)]
pub(crate) struct PendingGeneratedImage {
    pub(crate) source: PendingImageSource,
    pub(crate) mime_hint: Option<String>,
    pub(crate) remote_url: Option<String>,
    pub(crate) revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ProviderImageGenerationOutput {
    pub(crate) images: Vec<PendingGeneratedImage>,
    pub(crate) text: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageEditInputImage {
    pub(crate) bytes: Vec<u8>,
    pub(crate) mime: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct ImageEditInputs {
    pub(crate) images: Vec<ImageEditInputImage>,
    pub(crate) mask: Option<ImageEditInputImage>,
}
