#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ImageGenerationProviderKind {
    Comfyui,
    Codex,
    Openai,
    Xai,
    Seedream,
    Gemini,
}

pub(crate) const CODEX_IMAGE_MAIN_MODEL: &str = "gpt-5.6-luna";
pub(crate) const CODEX_IMAGE_TOOL_MODEL: &str = "gpt-image-2";

impl Default for ImageGenerationProviderKind {
    fn default() -> Self {
        Self::Openai
    }
}

impl ImageGenerationProviderKind {
    pub(crate) fn as_str(self) -> &'static str {
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
pub(crate) struct ImageGenerationModelConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) model: String,
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) deprecated: bool,
    #[serde(default)]
    pub(crate) default_size: Option<String>,
    #[serde(default)]
    pub(crate) default_aspect_ratio: Option<String>,
    #[serde(default)]
    pub(crate) default_quality: Option<String>,
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
pub(crate) struct ComfyUiNodeInputMapping {
    #[serde(default)]
    pub(crate) node_ids: Vec<String>,
    #[serde(default)]
    pub(crate) input_key: String,
}

impl ComfyUiNodeInputMapping {
    pub(crate) fn with_input_key(input_key: &str) -> Self {
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
pub(crate) struct ComfyUiWorkflowMapping {
    #[serde(default = "default_comfyui_prompt_mapping")]
    pub(crate) prompt: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_negative_prompt_mapping")]
    pub(crate) negative_prompt: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_model_mapping")]
    pub(crate) model: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_width_mapping")]
    pub(crate) width: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_height_mapping")]
    pub(crate) height: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_seed_mapping")]
    pub(crate) seed: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_steps_mapping")]
    pub(crate) steps: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_input_image_mapping")]
    pub(crate) input_image: ComfyUiNodeInputMapping,
    #[serde(default = "default_comfyui_mask_image_mapping")]
    pub(crate) mask_image: ComfyUiNodeInputMapping,
    #[serde(default)]
    pub(crate) output_node_ids: Vec<String>,
}

pub(crate) fn default_comfyui_prompt_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("text")
}

pub(crate) fn default_comfyui_negative_prompt_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("text")
}

pub(crate) fn default_comfyui_model_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("ckpt_name")
}

pub(crate) fn default_comfyui_width_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("width")
}

pub(crate) fn default_comfyui_height_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("height")
}

pub(crate) fn default_comfyui_seed_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("seed")
}

pub(crate) fn default_comfyui_steps_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("steps")
}

pub(crate) fn default_comfyui_input_image_mapping() -> ComfyUiNodeInputMapping {
    ComfyUiNodeInputMapping::with_input_key("image")
}

pub(crate) fn default_comfyui_mask_image_mapping() -> ComfyUiNodeInputMapping {
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

pub(crate) fn default_image_generation_timeout_seconds() -> u32 {
    600
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImageGenerationProviderConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) provider_type: ImageGenerationProviderKind,
    #[serde(default = "default_true")]
    pub(crate) enabled: bool,
    #[serde(default)]
    pub(crate) deprecated: bool,
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_keys: Vec<String>,
    #[serde(default)]
    pub(crate) codex_api_provider_id: Option<String>,
    #[serde(default)]
    pub(crate) key_cursor: u32,
    #[serde(default = "default_image_generation_timeout_seconds")]
    pub(crate) timeout_seconds: u32,
    #[serde(default)]
    pub(crate) watermark: bool,
    #[serde(default)]
    pub(crate) models: Vec<ImageGenerationModelConfig>,
    #[serde(default)]
    pub(crate) comfyui_workflow_json: String,
    #[serde(default)]
    pub(crate) comfyui_mapping: ComfyUiWorkflowMapping,
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

pub(crate) fn default_image_generation_providers() -> Vec<ImageGenerationProviderConfig> {
    Vec::new()
}
