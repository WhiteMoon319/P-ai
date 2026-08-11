pub(crate) fn build_qwen_media_block(
    media_type: ReadMediaDetectedType,
    mime: &str,
    content_base64: &str,
) -> Result<serde_json::Value, String> {
    Ok(match media_type {
        ReadMediaDetectedType::Image => {
            let data_url = build_qwen_media_data_url(mime, content_base64)?;
            serde_json::json!({
                "type": "image_url",
                "image_url": { "url": data_url }
            })
        }
        ReadMediaDetectedType::Audio => {
            if content_base64.len() > QWEN_MEDIA_DATA_URL_LIMIT_BYTES {
                return Err(format!(
                    "QWEN 音频 Base64 超过 50MB，当前大小={} bytes，上限={} bytes。",
                    content_base64.len(),
                    QWEN_MEDIA_DATA_URL_LIMIT_BYTES
                ));
            }
            serde_json::json!({
                "type": "input_audio",
                "input_audio": {
                    "data": content_base64,
                    "format": openai_input_audio_format_from_mime_for_read_media(mime)
                }
            })
        }
        ReadMediaDetectedType::Video => {
            let data_url = build_qwen_media_data_url(mime, content_base64)?;
            serde_json::json!({
                "type": "video_url",
                "video_url": { "url": data_url }
            })
        }
    })
}

pub(crate) fn build_qwen_media_data_url(mime: &str, content_base64: &str) -> Result<String, String> {
    let data_url = format!("data:{mime};base64,{content_base64}");
    if data_url.len() > QWEN_MEDIA_DATA_URL_LIMIT_BYTES {
        return Err(format!(
            "QWEN 多模态 Data URL 超过 50MB，当前大小={} bytes，上限={} bytes。",
            data_url.len(),
            QWEN_MEDIA_DATA_URL_LIMIT_BYTES
        ));
    }
    Ok(data_url)
}

pub(crate) async fn describe_qwen_media_with_multimodal_api(
    state: &AppState,
    resolved_api: &ResolvedApiConfig,
    selected_api: &ApiConfig,
    media_type: ReadMediaDetectedType,
    mime: &str,
    content_base64: &str,
    description: &str,
) -> Result<String, String> {
    let request_api = resolve_request_api_config(resolved_api).await?;
    let api_key = consume_api_key_for_request(&request_api);
    let url = openai_family_chat_completions_url(&request_api.base_url);
    let user_text = build_read_media_user_text(media_type, description);
    let media_block = build_qwen_media_block(media_type, mime, content_base64)?;
    let max_tokens = request_api
        .max_output_tokens
        .unwrap_or(selected_api.max_output_tokens);
    let body = serde_json::json!({
        "model": selected_api.model,
        "messages": [
            {
                "role": "system",
                "content": "[SYSTEM PROMPT]\n你是 QWEN 多媒体理解助手。请优先完成用户要求，输出简洁、结构清楚的中文结果。"
            },
            {
                "role": "user",
                "content": [
                    media_block,
                    {
                        "type": "text",
                        "text": user_text
                    }
                ]
            }
        ],
        "max_tokens": max_tokens
    });
    let resolved_protocol = resolve_model_protocol(
        request_api.request_format,
        &request_api.base_url,
        &selected_api.model,
        genai::adapter::AdapterKind::OpenAI,
    );
    let request_builder = state
        .shared_http_client
        .post(&url)
        .header(reqwest::header::CONTENT_TYPE, "application/json");
    let request_builder = apply_provider_auth_scheme(
        request_builder,
        resolved_protocol.auth_scheme,
        api_key.trim(),
    )?;
    let response = apply_read_media_timeout(
        apply_extra_headers(request_builder, &request_api.extra_headers),
        media_type,
    )
    .json(&body)
    .send()
    .await
    .map_err(|err| read_media_request_error("请求 QWEN 多模态接口失败", &err))?;
    if !response.status().is_success() {
        let status = response.status();
        let raw = response.text().await.unwrap_or_default();
        let snippet = raw.chars().take(1000).collect::<String>();
        return Err(format!("QWEN 多模态请求失败：{} | {}", status, snippet));
    }
    let payload = response
        .json::<serde_json::Value>()
        .await
        .map_err(|err| read_media_response_error("解析 QWEN 多模态响应失败", &err))?;
    let text = extract_openai_family_message_text(&payload);
    if text.is_empty() {
        return Err(format!("QWEN 多模态响应为空：{payload}"));
    }
    Ok(text)
}
