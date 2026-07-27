fn comfyui_candidate_roots(base_url: &str) -> Vec<String> {
    let base = base_url.trim().trim_end_matches('/').to_string();
    let mut roots = if base.to_ascii_lowercase().ends_with("/api") {
        vec![base.clone(), base[..base.len().saturating_sub(4)].to_string()]
    } else {
        vec![base.clone(), format!("{base}/api")]
    };
    roots.retain(|value| !value.trim().is_empty());
    roots.dedup();
    roots
}

fn comfyui_workflow_from_json(raw: &str) -> Result<Value, String> {
    let parsed = serde_json::from_str::<Value>(raw)
        .map_err(|err| format!("ComfyUI workflow JSON 无效：{err}"))?;
    let workflow = match parsed.get("workflow") {
        Some(Value::Object(value)) => Value::Object(value.clone()),
        Some(Value::String(value)) => serde_json::from_str::<Value>(value)
            .map_err(|err| format!("ComfyUI workflow 字段不是有效 JSON：{err}"))?,
        _ => parsed,
    };
    if !workflow.is_object() {
        return Err("ComfyUI workflow 必须是 API Format 对象".to_string());
    }
    Ok(workflow)
}

fn inject_comfyui_mapping_value(
    workflow: &mut Value,
    mapping: &ComfyUiNodeInputMapping,
    value: Value,
    field_name: &str,
    required: bool,
) -> Result<(), String> {
    if mapping.node_ids.is_empty() {
        return if required {
            Err(format!("ComfyUI 未配置{field_name}节点 ID"))
        } else {
            Ok(())
        };
    }
    let input_key = mapping.input_key.trim();
    if input_key.is_empty() {
        return Err(format!("ComfyUI {field_name} input key 为空"));
    }
    for node_id in &mapping.node_ids {
        let node = workflow
            .get_mut(node_id)
            .ok_or_else(|| format!("ComfyUI workflow 中找不到{field_name}节点 {node_id}"))?;
        let inputs = node
            .get_mut("inputs")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("ComfyUI 节点 {node_id} 缺少 inputs 对象"))?;
        inputs.insert(input_key.to_string(), value.clone());
    }
    Ok(())
}

fn comfyui_dimensions_from_aspect_ratio(value: &str) -> Option<(u32, u32)> {
    let (ratio_width, ratio_height) = parse_aspect_ratio(value)?;
    let round_to_64 = |value: f64| -> u32 {
        (((value / 64.0).round() as u32).max(1) * 64).clamp(64, 4096)
    };
    if ratio_width >= ratio_height {
        Some((1024, round_to_64(1024.0 * f64::from(ratio_height) / f64::from(ratio_width))))
    } else {
        Some((round_to_64(1024.0 * f64::from(ratio_width) / f64::from(ratio_height)), 1024))
    }
}

fn comfyui_effective_dimensions(
    request: &ImageGenerationRequest,
    model: &ImageGenerationModelConfig,
) -> Option<(u32, u32)> {
    trimmed_image_generation_option(&request.size)
        .as_deref()
        .and_then(parse_pixel_size)
        .or_else(|| {
            trimmed_image_generation_option(&request.aspect_ratio)
                .as_deref()
                .and_then(comfyui_dimensions_from_aspect_ratio)
        })
        .or_else(|| {
            trimmed_image_generation_option(&model.default_size)
                .as_deref()
                .and_then(parse_pixel_size)
        })
        .or_else(|| {
            trimmed_image_generation_option(&model.default_aspect_ratio)
                .as_deref()
                .and_then(comfyui_dimensions_from_aspect_ratio)
        })
}

fn build_comfyui_workflow(
    provider: &ImageGenerationProviderConfig,
    model: &ImageGenerationModelConfig,
    request: &ImageGenerationRequest,
) -> Result<Value, String> {
    if provider.comfyui_workflow_json.trim().is_empty() {
        return Err("ComfyUI 尚未配置 API Format workflow JSON".to_string());
    }
    let mut workflow = comfyui_workflow_from_json(&provider.comfyui_workflow_json)?;
    inject_comfyui_mapping_value(
        &mut workflow,
        &provider.comfyui_mapping.prompt,
        Value::String(request.prompt.trim().to_string()),
        "正向提示词",
        true,
    )?;
    if let Some(negative_prompt) = request
        .negative_prompt
        .as_ref()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
    {
        inject_comfyui_mapping_value(
            &mut workflow,
            &provider.comfyui_mapping.negative_prompt,
            Value::String(negative_prompt.to_string()),
            "负向提示词",
            false,
        )?;
    }
    if !model.model.trim().is_empty() {
        inject_comfyui_mapping_value(
            &mut workflow,
            &provider.comfyui_mapping.model,
            Value::String(model.model.trim().to_string()),
            "模型",
            false,
        )?;
    }
    if let Some((width, height)) = comfyui_effective_dimensions(request, model) {
        inject_comfyui_mapping_value(
            &mut workflow,
            &provider.comfyui_mapping.width,
            Value::Number(serde_json::Number::from(width)),
            "宽度",
            false,
        )?;
        inject_comfyui_mapping_value(
            &mut workflow,
            &provider.comfyui_mapping.height,
            Value::Number(serde_json::Number::from(height)),
            "高度",
            false,
        )?;
    }
    if let Some(seed) = request.seed {
        inject_comfyui_mapping_value(
            &mut workflow,
            &provider.comfyui_mapping.seed,
            Value::Number(serde_json::Number::from(seed)),
            "随机种子",
            false,
        )?;
    }
    if let Some(steps) = request.steps {
        inject_comfyui_mapping_value(
            &mut workflow,
            &provider.comfyui_mapping.steps,
            Value::Number(serde_json::Number::from(steps)),
            "采样步数",
            false,
        )?;
    }
    Ok(workflow)
}

fn comfyui_request_with_auth(
    request: reqwest::RequestBuilder,
    api_key: &str,
) -> reqwest::RequestBuilder {
    if api_key.trim().is_empty() {
        request
    } else {
        request.bearer_auth(api_key.trim())
    }
}

async fn queue_comfyui_workflow(
    state: &AppState,
    provider: &ImageGenerationProviderConfig,
    api_key: &str,
    workflow: Value,
) -> Result<(String, String), String> {
    let payload = serde_json::json!({
        "prompt": workflow,
        "client_id": Uuid::new_v4().to_string()
    });
    let mut last_not_found = None::<String>;
    for root in comfyui_candidate_roots(&provider.base_url) {
        let endpoint = append_image_generation_endpoint(&root, "/prompt");
        let request = state
            .shared_http_client
            .post(&endpoint)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&payload)
            .timeout(std::time::Duration::from_secs(u64::from(provider.timeout_seconds)));
        let response = comfyui_request_with_auth(request, api_key)
            .send()
            .await
            .map_err(|err| format!("连接 ComfyUI 失败：{err}"))?;
        if matches!(response.status().as_u16(), 404 | 405) {
            let status = response.status();
            let bytes = read_limited_response_bytes(response, IMAGE_GENERATION_MAX_ERROR_BYTES).await?;
            last_not_found = Some(format!(
                "{}：HTTP {} {}",
                endpoint,
                status,
                truncate_image_generation_error_body(&bytes)
            ));
            continue;
        }
        let value = parse_image_generation_json_response(response, &provider.name).await?;
        let prompt_id = image_generation_value_string(&value, &["prompt_id", "promptId"])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "ComfyUI 排队响应缺少 prompt_id".to_string())?;
        return Ok((root, prompt_id.to_string()));
    }
    Err(format!(
        "ComfyUI 未找到 /prompt 或 /api/prompt 接口{}",
        last_not_found
            .map(|value| format!("：{value}"))
            .unwrap_or_default()
    ))
}

fn comfyui_history_record<'a>(value: &'a Value, prompt_id: &str) -> Option<&'a Value> {
    value.get(prompt_id).or_else(|| {
        value
            .get("prompt_id")
            .and_then(Value::as_str)
            .filter(|value| *value == prompt_id)
            .map(|_| value)
    })
}

fn comfyui_history_completed(record: &Value) -> bool {
    record
        .get("status")
        .and_then(|status| status.get("completed"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
        || record
            .get("status")
            .and_then(|status| image_generation_value_string(status, &["status_str", "statusStr"]))
            .is_some_and(|value| value.eq_ignore_ascii_case("success"))
}

fn comfyui_history_error(record: &Value) -> Option<String> {
    let status = record.get("status")?;
    let status_text = image_generation_value_string(status, &["status_str", "statusStr"])?;
    if status_text.eq_ignore_ascii_case("error") {
        Some(
            status
                .get("messages")
                .map(Value::to_string)
                .unwrap_or_else(|| "ComfyUI 执行失败".to_string()),
        )
    } else {
        None
    }
}

fn comfyui_view_url(root: &str, image: &Value) -> Result<String, String> {
    let filename = image
        .get("filename")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "ComfyUI 图片输出缺少 filename".to_string())?;
    let subfolder = image
        .get("subfolder")
        .and_then(Value::as_str)
        .unwrap_or("");
    let folder_type = image
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("output");
    let endpoint = append_image_generation_endpoint(root, "/view");
    let mut url = reqwest::Url::parse(&endpoint)
        .map_err(|err| format!("ComfyUI view URL 无效：{err}"))?;
    url.query_pairs_mut()
        .append_pair("filename", filename)
        .append_pair("subfolder", subfolder)
        .append_pair("type", folder_type);
    Ok(url.to_string())
}

fn extract_comfyui_history_images(
    record: &Value,
    root: &str,
    output_node_ids: &[String],
) -> Result<Vec<PendingGeneratedImage>, String> {
    let outputs = record
        .get("outputs")
        .and_then(Value::as_object)
        .ok_or_else(|| "ComfyUI history 缺少 outputs".to_string())?;
    let output_filter = output_node_ids
        .iter()
        .map(|value| value.as_str())
        .collect::<std::collections::HashSet<_>>();
    let mut images = Vec::<PendingGeneratedImage>::new();
    for (node_id, output) in outputs {
        if !output_filter.is_empty() && !output_filter.contains(node_id.as_str()) {
            continue;
        }
        let Some(node_images) = output.get("images").and_then(Value::as_array) else {
            continue;
        };
        for image in node_images {
            let url = comfyui_view_url(root, image)?;
            images.push(PendingGeneratedImage {
                source: PendingImageSource::RemoteUrl(url.clone()),
                mime_hint: None,
                remote_url: Some(url),
                revised_prompt: None,
            });
        }
    }
    Ok(images)
}

async fn wait_for_comfyui_images(
    state: &AppState,
    provider: &ImageGenerationProviderConfig,
    api_key: &str,
    root: &str,
    prompt_id: &str,
) -> Result<Vec<PendingGeneratedImage>, String> {
    let deadline = tokio::time::Instant::now()
        + std::time::Duration::from_secs(u64::from(provider.timeout_seconds));
    let endpoint = append_image_generation_endpoint(
        root,
        &format!("/history/{}", urlencoding::encode(prompt_id)),
    );
    loop {
        if tokio::time::Instant::now() >= deadline {
            return Err(format!("ComfyUI 生成超时（{} 秒）", provider.timeout_seconds));
        }
        let request = state
            .shared_http_client
            .get(&endpoint)
            .timeout(std::time::Duration::from_secs(15));
        let response = comfyui_request_with_auth(request, api_key)
            .send()
            .await
            .map_err(|err| format!("查询 ComfyUI history 失败：{err}"))?;
        let value = parse_image_generation_json_response(response, &provider.name).await?;
        if let Some(record) = comfyui_history_record(&value, prompt_id) {
            if let Some(error) = comfyui_history_error(record) {
                return Err(format!("ComfyUI 执行失败：{error}"));
            }
            let images = extract_comfyui_history_images(
                record,
                root,
                &provider.comfyui_mapping.output_node_ids,
            )?;
            if !images.is_empty() {
                return Ok(images);
            }
            if comfyui_history_completed(record) {
                return Err("ComfyUI 已完成，但没有找到图片输出".to_string());
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(750)).await;
    }
}

async fn generate_comfyui_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    let workflow = build_comfyui_workflow(&resolved.provider, &resolved.model, request)?;
    queue_and_wait_comfyui_workflow(state, resolved, api_key, workflow).await
}

async fn queue_and_wait_comfyui_workflow(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    api_key: &str,
    workflow: Value,
) -> Result<ProviderImageGenerationOutput, String> {
    let (root, prompt_id) = queue_comfyui_workflow(
        state,
        &resolved.provider,
        api_key,
        workflow,
    )
    .await?;
    let images = wait_for_comfyui_images(
        state,
        &resolved.provider,
        api_key,
        &root,
        &prompt_id,
    )
    .await?;
    Ok(ProviderImageGenerationOutput { images, text: None })
}

// ==================== ComfyUI 图像编辑（上传输入图 + 节点注入） ====================

async fn upload_comfyui_input_image(
    state: &AppState,
    provider: &ImageGenerationProviderConfig,
    api_key: &str,
    input: &ImageEditInputImage,
    file_stem: &str,
) -> Result<String, String> {
    let mut last_not_found = None::<String>;
    for root in comfyui_candidate_roots(&provider.base_url) {
        let endpoint = append_image_generation_endpoint(&root, "/upload/image");
        let form = reqwest::multipart::Form::new()
            .part("image", image_edit_multipart_part(input, file_stem)?)
            .text("overwrite", "true");
        let request = state
            .shared_http_client
            .post(&endpoint)
            .multipart(form)
            .timeout(std::time::Duration::from_secs(u64::from(provider.timeout_seconds)));
        let response = comfyui_request_with_auth(request, api_key)
            .send()
            .await
            .map_err(|err| format!("上传图片到 ComfyUI 失败：{err}"))?;
        if matches!(response.status().as_u16(), 404 | 405) {
            let status = response.status();
            let bytes = read_limited_response_bytes(response, IMAGE_GENERATION_MAX_ERROR_BYTES).await?;
            last_not_found = Some(format!(
                "{}：HTTP {} {}",
                endpoint,
                status,
                truncate_image_generation_error_body(&bytes)
            ));
            continue;
        }
        let value = parse_image_generation_json_response(response, &provider.name).await?;
        let name = image_generation_value_string(&value, &["name"])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "ComfyUI 上传响应缺少 name".to_string())?;
        let subfolder = value
            .get("subfolder")
            .and_then(Value::as_str)
            .map(str::trim)
            .unwrap_or("");
        return Ok(if subfolder.is_empty() {
            name.to_string()
        } else {
            format!("{subfolder}/{name}")
        });
    }
    Err(format!(
        "ComfyUI 未找到 /upload/image 或 /api/upload/image 接口{}",
        last_not_found
            .map(|value| format!("：{value}"))
            .unwrap_or_default()
    ))
}

// 每个 LoadImage 类节点只承载一张图片，按映射顺序一一注入上传后的文件名。
fn inject_comfyui_input_images(
    workflow: &mut Value,
    mapping: &ComfyUiNodeInputMapping,
    field_name: &str,
    uploaded_names: &[String],
) -> Result<(), String> {
    if mapping.node_ids.is_empty() {
        return Err(format!(
            "ComfyUI 未配置{field_name}节点映射，无法执行图像编辑"
        ));
    }
    if uploaded_names.len() > mapping.node_ids.len() {
        return Err(format!(
            "ComfyUI {field_name}节点只配置了 {} 个，无法承载 {} 张图片",
            mapping.node_ids.len(),
            uploaded_names.len()
        ));
    }
    let input_key = mapping.input_key.trim();
    if input_key.is_empty() {
        return Err(format!("ComfyUI {field_name} input key 为空"));
    }
    for (node_id, name) in mapping.node_ids.iter().zip(uploaded_names) {
        let node = workflow
            .get_mut(node_id)
            .ok_or_else(|| format!("ComfyUI workflow 中找不到{field_name}节点 {node_id}"))?;
        let inputs = node
            .get_mut("inputs")
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("ComfyUI 节点 {node_id} 缺少 inputs 对象"))?;
        inputs.insert(input_key.to_string(), Value::String(name.clone()));
    }
    Ok(())
}

async fn edit_comfyui_image_once(
    state: &AppState,
    resolved: &ResolvedImageGenerationModel,
    request: &ImageGenerationRequest,
    inputs: &ImageEditInputs,
    api_key: &str,
) -> Result<ProviderImageGenerationOutput, String> {
    ensure_image_edit_input_limits(
        &resolved.provider.name,
        inputs,
        IMAGE_EDIT_MAX_INPUT_IMAGES,
        true,
    )?;
    let mapping = &resolved.provider.comfyui_mapping;
    // 无法映射时返回可读错误，不静默降级为文生图。
    if mapping.input_image.node_ids.is_empty() {
        return Err(
            "ComfyUI 未配置输入图片节点映射（input_image），无法执行图像编辑；请在生图设置中为该供应商补充 LoadImage 节点映射".to_string(),
        );
    }
    if inputs.mask.is_some() && mapping.mask_image.node_ids.is_empty() {
        return Err(
            "ComfyUI 未配置 mask 输入节点映射（mask_image），无法使用 mask；请补充映射或去掉 mask 参数".to_string(),
        );
    }
    let mut workflow = build_comfyui_workflow(&resolved.provider, &resolved.model, request)?;
    let batch = Uuid::new_v4().to_string();
    let mut uploaded_names = Vec::with_capacity(inputs.images.len());
    for (index, input) in inputs.images.iter().enumerate() {
        uploaded_names.push(
            upload_comfyui_input_image(
                state,
                &resolved.provider,
                api_key,
                input,
                &format!("pai-edit-{batch}-{index}"),
            )
            .await?,
        );
    }
    inject_comfyui_input_images(
        &mut workflow,
        &mapping.input_image,
        "输入图片",
        &uploaded_names,
    )?;
    if let Some(mask) = &inputs.mask {
        let mask_name = upload_comfyui_input_image(
            state,
            &resolved.provider,
            api_key,
            mask,
            &format!("pai-edit-{batch}-mask"),
        )
        .await?;
        inject_comfyui_input_images(
            &mut workflow,
            &mapping.mask_image,
            "mask 图片",
            &[mask_name],
        )?;
    }
    queue_and_wait_comfyui_workflow(state, resolved, api_key, workflow).await
}

#[cfg(test)]
mod image_generation_comfyui_tests {
    use super::*;

    fn comfy_provider() -> ImageGenerationProviderConfig {
        ImageGenerationProviderConfig {
            id: "comfy".to_string(),
            name: "ComfyUI".to_string(),
            provider_type: ImageGenerationProviderKind::Comfyui,
            enabled: true,
            deprecated: false,
            base_url: "http://127.0.0.1:8188".to_string(),
            api_keys: Vec::new(),
            codex_api_provider_id: None,
            key_cursor: 0,
            timeout_seconds: 600,
            watermark: false,
            models: Vec::new(),
            comfyui_workflow_json: serde_json::json!({
                "6": { "inputs": { "text": "old" }, "class_type": "CLIPTextEncode" },
                "7": { "inputs": { "text": "old" }, "class_type": "CLIPTextEncode" },
                "5": { "inputs": { "width": 512, "height": 512 }, "class_type": "EmptyLatentImage" }
            }).to_string(),
            comfyui_mapping: ComfyUiWorkflowMapping {
                prompt: ComfyUiNodeInputMapping { node_ids: vec!["6".to_string()], input_key: "text".to_string() },
                negative_prompt: ComfyUiNodeInputMapping { node_ids: vec!["7".to_string()], input_key: "text".to_string() },
                width: ComfyUiNodeInputMapping { node_ids: vec!["5".to_string()], input_key: "width".to_string() },
                height: ComfyUiNodeInputMapping { node_ids: vec!["5".to_string()], input_key: "height".to_string() },
                ..ComfyUiWorkflowMapping::default()
            },
        }
    }

    #[test]
    fn comfyui_workflow_should_inject_multiple_fields() {
        let provider = comfy_provider();
        let model = ImageGenerationModelConfig {
            id: "workflow".to_string(),
            name: "Workflow".to_string(),
            model: String::new(),
            enabled: true,
            deprecated: false,
            default_size: None,
            default_aspect_ratio: None,
            default_quality: None,
        };
        let request = ImageGenerationRequest {
            prompt: "new prompt".to_string(),
            negative_prompt: Some("bad".to_string()),
            size: Some("1024x768".to_string()),
            ..ImageGenerationRequest::default()
        };
        let workflow = build_comfyui_workflow(&provider, &model, &request).unwrap_or_default();
        assert_eq!(workflow.pointer("/6/inputs/text").and_then(Value::as_str), Some("new prompt"));
        assert_eq!(workflow.pointer("/7/inputs/text").and_then(Value::as_str), Some("bad"));
        assert_eq!(workflow.pointer("/5/inputs/width").and_then(Value::as_u64), Some(1024));
        assert_eq!(workflow.pointer("/5/inputs/height").and_then(Value::as_u64), Some(768));
    }

    #[test]
    fn comfyui_history_should_extract_only_selected_output_nodes() {
        let record = serde_json::json!({
            "outputs": {
                "9": { "images": [{ "filename": "a.png", "subfolder": "", "type": "output" }] },
                "10": { "images": [{ "filename": "b.png", "subfolder": "", "type": "output" }] }
            }
        });
        let images = extract_comfyui_history_images(
            &record,
            "http://127.0.0.1:8188",
            &["10".to_string()],
        )
        .unwrap_or_default();
        assert_eq!(images.len(), 1);
        assert!(images[0].remote_url.as_deref().unwrap_or_default().contains("b.png"));
    }

    #[test]
    fn comfyui_input_image_injection_should_map_names_and_reject_overflow() {
        let mut workflow = serde_json::json!({
            "11": { "inputs": { "image": "old-a.png" }, "class_type": "LoadImage" },
            "12": { "inputs": { "image": "old-b.png" }, "class_type": "LoadImage" }
        });
        let mapping = ComfyUiNodeInputMapping {
            node_ids: vec!["11".to_string(), "12".to_string()],
            input_key: "image".to_string(),
        };
        inject_comfyui_input_images(
            &mut workflow,
            &mapping,
            "输入图片",
            &["clip/new-a.png".to_string()],
        )
        .unwrap_or_default();
        assert_eq!(
            workflow.pointer("/11/inputs/image").and_then(Value::as_str),
            Some("clip/new-a.png")
        );
        // 未占用的节点保持原值，不被清空。
        assert_eq!(
            workflow.pointer("/12/inputs/image").and_then(Value::as_str),
            Some("old-b.png")
        );

        let overflow = inject_comfyui_input_images(
            &mut workflow,
            &mapping,
            "输入图片",
            &["a".to_string(), "b".to_string(), "c".to_string()],
        )
        .err()
        .unwrap_or_default();
        assert!(overflow.contains("无法承载 3 张"));

        let missing = inject_comfyui_input_images(
            &mut workflow,
            &ComfyUiNodeInputMapping { node_ids: Vec::new(), input_key: "image".to_string() },
            "输入图片",
            &["a".to_string()],
        )
        .err()
        .unwrap_or_default();
        assert!(missing.contains("未配置输入图片节点映射"));
    }
}
