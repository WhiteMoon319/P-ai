use serde::{Deserialize, Serialize};

use crate::core::domain::types_image_generation::{
    ImageGenerationModelConfig, ImageGenerationProviderConfig, ImageGenerationProviderKind,
};

pub fn default_image_generation_count() -> u32 {
    1
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ImageGenerationOperation {
    #[default]
    Generate,
    Edit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationRequest {
    pub prompt: String,
    #[serde(default)]
    pub operation: ImageGenerationOperation,
    #[serde(default, alias = "model_id")]
    pub model_id: Option<String>,
    #[serde(default, alias = "negative_prompt")]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default, alias = "aspect_ratio")]
    pub aspect_ratio: Option<String>,
    #[serde(default)]
    pub quality: Option<String>,
    #[serde(default = "default_image_generation_count")]
    pub n: u32,
    #[serde(default)]
    pub seed: Option<i64>,
    #[serde(default)]
    pub steps: Option<u32>,
    #[serde(default)]
    pub images: Vec<String>,
    #[serde(default)]
    pub mask: Option<String>,
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
pub struct GeneratedImageAsset {
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remote_url: Option<String>,
    pub markdown: String,
    pub mime: String,
    pub width: u32,
    pub height: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationResult {
    pub provider_id: String,
    pub provider_name: String,
    pub provider_type: ImageGenerationProviderKind,
    pub model_id: String,
    pub model: String,
    pub images: Vec<GeneratedImageAsset>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ResolvedImageGenerationModel {
    pub endpoint_id: String,
    pub provider: ImageGenerationProviderConfig,
    pub model: ImageGenerationModelConfig,
}

#[derive(Debug, Clone)]
pub enum PendingImageSource {
    Bytes(Vec<u8>),
    RemoteUrl(String),
}

#[derive(Debug, Clone)]
pub struct PendingGeneratedImage {
    pub source: PendingImageSource,
    pub mime_hint: Option<String>,
    pub remote_url: Option<String>,
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ProviderImageGenerationOutput {
    pub images: Vec<PendingGeneratedImage>,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ImageEditInputImage {
    pub bytes: Vec<u8>,
    pub mime: String,
}

#[derive(Debug, Clone, Default)]
pub struct ImageEditInputs {
    pub images: Vec<ImageEditInputImage>,
    pub mask: Option<ImageEditInputImage>,
}
