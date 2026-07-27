#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ImageGenerateToolArgs {
    prompt: String,
    #[serde(default)]
    aspect_ratio: Option<String>,
    #[serde(default, alias = "size")]
    resolution: Option<String>,
}

#[derive(Debug, Clone)]
struct BuiltinImageGenerateTool {
    app_state: AppState,
}

impl RuntimeToolMetadata for BuiltinImageGenerateTool {
    fn provider_tool_definition(&self) -> ProviderToolDefinition {
        ProviderToolDefinition::new(
            "image_generate",
            "根据提示词生成一张图片，自动保存到 Assistant Space；返回的 message 中包含 Markdown 图片行，向用户展示图片时直接原样引用该行，不要改写路径。",
            serde_json::json!({
                "type": "object",
                "properties": {
                    "prompt": {
                        "type": "string",
                        "description": "详细、可直接交给生图模型的提示词。"
                    },
                    "aspect_ratio": {
                        "type": "string",
                        "description": "可选宽高比，例如 1:1、16:9、9:16。"
                    },
                    "resolution": {
                        "type": "string",
                        "description": "可选分辨率或尺寸，例如 1024x1024、1536x1024、2K。"
                    }
                },
                "required": ["prompt"],
                "additionalProperties": false
            }),
        )
    }
}

impl RuntimeValueTool for BuiltinImageGenerateTool {
    const NAME: &'static str = "image_generate";
    type Args = ImageGenerateToolArgs;
    type Error = ToolInvokeError;

    fn timeout_override(_args_json: &str) -> Option<std::time::Duration> {
        Some(std::time::Duration::from_secs(1_830))
    }

    fn call_typed(&self, args: Self::Args) -> RuntimeToolValueFuture<'_, Self::Error> {
        Box::pin(async move {
            let request = ImageGenerationRequest {
                prompt: args.prompt,
                model_id: None,
                negative_prompt: None,
                size: args.resolution,
                aspect_ratio: args.aspect_ratio,
                quality: None,
                n: 1,
                seed: None,
                steps: None,
            };
            let result = generate_images(&self.app_state, request)
                .await
                .map_err(ToolInvokeError::from)?;
            let markdown = result
                .images
                .iter()
                .map(|image| image.markdown.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            let images = serde_json::to_value(&result.images)
                .map_err(|err| ToolInvokeError::from(format!("序列化生图结果失败：{err}")))?;
            Ok(serde_json::json!({
                "ok": true,
                "message": format!(
                    "图片已生成并保存到 Assistant Space。最终回答必须原样包含以下 Markdown 图片行，不要改写路径，也不要只回复‘已生成’：\n\n{markdown}"
                ),
                "provider": result.provider_name,
                "providerType": result.provider_type,
                "modelId": result.model_id,
                "model": result.model,
                "images": images,
                "providerText": result.provider_text
            }))
        })
    }
}

#[cfg(test)]
mod image_generate_tool_tests {
    use super::*;

    #[test]
    fn image_generate_definition_should_require_prompt_and_default_model() {
        let state = AppState::new().ok();
        let Some(state) = state else {
            return;
        };
        let definition = BuiltinImageGenerateTool { app_state: state }.provider_tool_definition();
        assert_eq!(definition.name, "image_generate");
        assert_eq!(
            definition.parameters["required"].as_array().and_then(|items| items.first()).and_then(Value::as_str),
            Some("prompt")
        );
        assert!(definition.description.contains("Markdown 图片行"));
        assert!(!definition.description.contains("设置页"));
        let properties = definition.parameters["properties"].as_object().cloned().unwrap_or_default();
        assert!(properties.contains_key("prompt"));
        assert!(properties.contains_key("aspect_ratio"));
        assert!(properties.contains_key("resolution"));
        assert!(!properties.contains_key("model_id"));
        assert!(!properties.contains_key("negative_prompt"));
        assert!(!properties.contains_key("quality"));
    }
}
