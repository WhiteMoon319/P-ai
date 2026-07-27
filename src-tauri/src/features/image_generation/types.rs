fn default_image_generation_count() -> u32 {
    1
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageGenerationRequest {
    prompt: String,
    #[serde(default, alias = "model_id")]
    model_id: Option<String>,
    #[serde(default, alias = "negative_prompt")]
    negative_prompt: Option<String>,
    #[serde(default)]
    size: Option<String>,
    #[serde(default, alias = "aspect_ratio")]
    aspect_ratio: Option<String>,
    #[serde(default)]
    quality: Option<String>,
    #[serde(default = "default_image_generation_count")]
    n: u32,
    #[serde(default)]
    seed: Option<i64>,
    #[serde(default)]
    steps: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeneratedImageAsset {
    relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    remote_url: Option<String>,
    markdown: String,
    mime: String,
    width: u32,
    height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ImageGenerationResult {
    provider_id: String,
    provider_name: String,
    provider_type: ImageGenerationProviderKind,
    model_id: String,
    model: String,
    images: Vec<GeneratedImageAsset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    provider_text: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedImageGenerationModel {
    endpoint_id: String,
    provider: ImageGenerationProviderConfig,
    model: ImageGenerationModelConfig,
}

#[derive(Debug, Clone)]
enum PendingImageSource {
    Bytes(Vec<u8>),
    RemoteUrl(String),
}

#[derive(Debug, Clone)]
struct PendingGeneratedImage {
    source: PendingImageSource,
    mime_hint: Option<String>,
    remote_url: Option<String>,
    revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct ProviderImageGenerationOutput {
    images: Vec<PendingGeneratedImage>,
    text: Option<String>,
}
