use serde::{Deserialize, Serialize};

use crate::core::domain::types_config::default_true;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ImageGenerationProviderKind {
    Comfyui,
    Codex,
    Openai,
    Xai,
    Seedream,
    Gemini,
}

pub const CODEX_IMAGE_MAIN_MODEL: &str = "gpt-5.6-luna";
pub const CODEX_IMAGE_TOOL_MODEL: &str = "gpt-image-2";

impl Default for ImageGenerationProviderKind {
    fn default() -> Self {
        Self::Openai
    }
}

impl ImageGenerationProviderKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Comfyui => "comfyui",
            Self::Codex => "codex",
            Self::Openai => "openai",
            Self::Xai => "xai",
            Self::Seedream => "seedream",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationModelConfig {
    pub id: String,
    pub name: String,
    pub model: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub deprecated: bool,
    #[serde(default)]
    pub default_size: Option<String>,
    #[serde(default)]
    pub default_aspect_ratio: Option<String>,
    #[serde(default)]
    pub default_quality: Option<String>,
}

impl Default for ImageGenerationModelConfig {
    fn default() -> Self {
        Self {
            id: "gpt-image-2".to_string(),
            name: "GPT Image 2".to_string(),
            model: "gpt-image-2".to_string(),
            enabled: true,
            deprecated: false,
            default_size: Some("512x512".to_string()),
            default_aspect_ratio: Some("1:1".to_string()),
            default_quality: Some("medium".to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComfyUiNodeInputMapping {
    #[serde(default)]
    pub node_ids: Vec<String>,
    #[serde(default)]
    pub input_key: String,
}

impl ComfyUiNodeInputMapping {
    pub fn with_input_key(input_key: &str) -> Self {
        Self {
            node_ids: Vec::new(),
            input_key: input_key.to_string(),
        }
    }
}

impl Default for ComfyUiNodeInputMapping {
    fn default() -> Self {
        Self::with_input_key("")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ComfyUiWorkflowMapping {
    #[serde(default = "default_comfyui_prompt_mapping")]
    pub prompt: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_negative_prompt_mapping")]
    pub negative_prompt: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_model_mapping")]
    pub model: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_width_mapping")]
    pub width: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_height_mapping")]
    pub height: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_seed_mapping")]
    pub seed: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_steps_mapping")]
    pub steps: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_input_image_mapping")]
    pub input_image: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_mask_image_mapping")]
    pub mask_image: ComfyUiNodeInputMapping,
    #[serde(default)]
    pub output_node_ids: Vec<String>,
}

pub fn default_comfyui_prompt_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("text")
}

pub fn default_comfyui_negative_prompt_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("text")
}

pub fn default_comfyui_model_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("ckpt_name")
}

pub fn default_comfyui_width_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("width")
}

pub fn default_comfyui_height_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("height")
}

pub fn default_comfyui_seed_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("seed")
}

pub fn default_comfyui_steps_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("steps")
}

pub fn default_comfyui_input_image_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("image")
}

pub fn default_comfyui_mask_image_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("image")
}

impl Default for ComfyUiWorkflowMapping {
    fn default() -> Self {
        Self {
            prompt: default_comfyui_prompt_mapping(),
            negative_prompt: default_comfyui_negative_prompt_mapping(),
            model: default_comfyui_model_mapping(),
            width: default_comfyui_width_mapping(),
            height: default_comfyui_height_mapping(),
            seed: default_comfyui_seed_mapping(),
            steps: default_comfyui_steps_mapping(),
            input_image: default_comfyui_input_image_mapping(),
            mask_image: default_comfyui_mask_image_mapping(),
            output_node_ids: Vec::new(),
        }
    }
}

pub fn default_image_generation_timeout_seconds() -> u32 {
    600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageGenerationProviderConfig {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub provider_type: ImageGenerationProviderKind,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub deprecated: bool,
    pub base_url: String,
    #[serde(default)]
    pub api_keys: Vec<String>,
    #[serde(default)]
    pub codex_api_provider_id: Option<String>,
    #[serde(default)]
    pub key_cursor: u32,
    #[serde(default = "default_image_generation_timeout_seconds")]
    pub timeout_seconds: u32,
    #[serde(default)]
    pub watermark: bool,
    #[serde(default)]
    pub models: Vec<ImageGenerationModelConfig>,
    #[serde(default)]
    pub comfyui_workflow_json: String,
    #[serde(default)]
    pub comfyui_mapping: ComfyUiWorkflowMapping,
}

impl Default for ImageGenerationProviderConfig {
    fn default() -> Self {
        Self {
            id: "image-provider-openai".to_string(),
            name: "OpenAI Images".to_string(),
            provider_type: ImageGenerationProviderKind::Openai,
            enabled: true,
            deprecated: false,
            base_url: "https://api.openai.com/v1".to_string(),
            api_keys: Vec::new(),
            codex_api_provider_id: None,
            key_cursor: 0,
            timeout_seconds: default_image_generation_timeout_seconds(),
            watermark: false,
            models: vec![ImageGenerationModelConfig::default()],
            comfyui_workflow_json: String::new(),
            comfyui_mapping: ComfyUiWorkflowMapping::default(),
        }
    }
}

pub fn default_image_generation_providers() -> Vec<ImageGenerationProviderConfig> {
    Vec::new()
}
