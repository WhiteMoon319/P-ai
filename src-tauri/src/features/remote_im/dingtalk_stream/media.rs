async fn dingtalk_download_file_by_code(
    channel: &RemoteImChannelConfig,
    download_code: &str,
    robot_code: &str,
) -> Result<(Vec<u8>, String), String> {
    let access_token = dingtalk_access_token(channel).await?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .map_err(|err| format!("build dingtalk download client failed: {err}"))?;
    let response = client
        .post(DINGTALK_DOWNLOAD_API)
        .header("x-acs-dingtalk-access-token", access_token)
        .json(&serde_json::json!({
            "downloadCode": download_code,
            "robotCode": robot_code
        }))
        .send()
        .await
        .map_err(|err| format!("dingtalk download-url request failed: {err}"))?;
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .map_err(|err| format!("parse dingtalk download-url response failed: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "dingtalk download-url rejected http {}: {}",
            status.as_u16(),
            body
        ));
    }
    let download_url = body
        .get("downloadUrl")
        .and_then(Value::as_str)
        .or_else(|| body.get("data").and_then(|v| v.get("downloadUrl")).and_then(Value::as_str))
        .map(str::trim)
        .unwrap_or("");
    if download_url.is_empty() {
        return Err(format!("dingtalk download-url missing: {body}"));
    }
    let file_resp = client
        .get(download_url)
        .send()
        .await
        .map_err(|err| format!("dingtalk download file failed: {err}"))?;
    let file_status = file_resp.status();
    if !file_status.is_success() {
        return Err(format!(
            "dingtalk download file rejected http {}",
            file_status.as_u16()
        ));
    }
    if let Some(content_len) = file_resp.content_length() {
        if content_len > DINGTALK_MAX_DOWNLOAD_SIZE_BYTES {
            return Err(format!(
                "dingtalk download file too large: {} bytes > {} bytes",
                content_len,
                DINGTALK_MAX_DOWNLOAD_SIZE_BYTES
            ));
        }
    }
    let mime = file_resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(str::trim)
        .unwrap_or("application/octet-stream")
        .to_string();
    let mut stream = file_resp.bytes_stream();
    let mut total = 0u64;
    let mut bytes = Vec::<u8>::new();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|err| format!("read dingtalk downloaded file failed: {err}"))?;
        total = total.saturating_add(chunk.len() as u64);
        if total > DINGTALK_MAX_DOWNLOAD_SIZE_BYTES {
            return Err(format!(
                "dingtalk download file too large while streaming: {} bytes > {} bytes",
                total,
                DINGTALK_MAX_DOWNLOAD_SIZE_BYTES
            ));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok((bytes, mime))
}

fn mime_from_name_fallback(file_name: &str) -> String {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".mp3") {
        "audio/mpeg".to_string()
    } else if lower.ends_with(".wav") {
        "audio/wav".to_string()
    } else if lower.ends_with(".ogg") {
        "audio/ogg".to_string()
    } else if lower.ends_with(".amr") {
        "audio/amr".to_string()
    } else if lower.ends_with(".mp4") {
        "video/mp4".to_string()
    } else if lower.ends_with(".pdf") {
        "application/pdf".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

fn normalize_dingtalk_image_mime(raw: &[u8], mime: &str) -> String {
    if let Some(detected) = image_mime_from_bytes(raw) {
        return detected.to_string();
    }
    let trimmed = mime.trim();
    if trimmed.starts_with("image/") {
        return trimmed.to_string();
    }
    if trimmed.is_empty() {
        "image/png".to_string()
    } else {
        trimmed.to_string()
    }
}

