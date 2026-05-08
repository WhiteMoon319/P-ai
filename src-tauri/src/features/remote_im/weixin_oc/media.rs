fn build_weixin_oc_http_client(timeout_ms: u64) -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|err| format!("创建个人微信 HTTP 客户端失败: {err}"))
}

fn weixin_oc_cdn_download_url(cdn_base_url: &str, encrypted_query_param: &str) -> String {
    format!(
        "{}/download?encrypted_query_param={}",
        cdn_base_url.trim_end_matches('/'),
        urlencoding::encode(encrypted_query_param.trim())
    )
}

fn weixin_oc_cdn_upload_url(cdn_base_url: &str, upload_param: &str, file_key: &str) -> String {
    format!(
        "{}/upload?encrypted_query_param={}&filekey={}",
        cdn_base_url.trim_end_matches('/'),
        urlencoding::encode(upload_param.trim()),
        urlencoding::encode(file_key.trim())
    )
}

fn weixin_oc_pkcs7_pad(data: &[u8]) -> Vec<u8> {
    let pad_len = 16 - (data.len() % 16);
    let pad_len = if pad_len == 0 { 16 } else { pad_len };
    let mut out = Vec::with_capacity(data.len() + pad_len);
    out.extend_from_slice(data);
    out.extend(std::iter::repeat_n(pad_len as u8, pad_len));
    out
}

fn weixin_oc_encrypt_media_ecb(raw: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    use aes::cipher::{generic_array::GenericArray, BlockEncrypt, KeyInit};

    if key.len() != 16 {
        return Err(format!("媒体 AES 密钥长度不正确: {}", key.len()));
    }
    let cipher = aes::Aes128::new_from_slice(key)
        .map_err(|err| format!("初始化媒体 AES 加密器失败: {err}"))?;
    let mut encrypted = weixin_oc_pkcs7_pad(raw);
    for chunk in encrypted.chunks_exact_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.encrypt_block(block);
    }
    Ok(encrypted)
}

fn weixin_oc_aes_padded_size(size: usize) -> usize {
    let remainder = size % 16;
    if remainder == 0 {
        size + 16
    } else {
        size + (16 - remainder)
    }
}

fn weixin_oc_encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

fn weixin_oc_pkcs7_unpad(data: &[u8]) -> Vec<u8> {
    let Some(&pad_len) = data.last() else {
        return Vec::new();
    };
    let pad_len = pad_len as usize;
    if pad_len == 0 || pad_len > 16 || pad_len > data.len() {
        return data.to_vec();
    }
    if data[data.len() - pad_len..]
        .iter()
        .all(|value| *value as usize == pad_len)
    {
        data[..data.len() - pad_len].to_vec()
    } else {
        data.to_vec()
    }
}

fn weixin_oc_decode_hex(input: &str) -> Result<Vec<u8>, String> {
    let normalized = input.trim();
    if normalized.is_empty() {
        return Err("十六进制密钥为空".to_string());
    }
    if normalized.len() % 2 != 0 {
        return Err("十六进制密钥长度不正确".to_string());
    }
    let mut out = Vec::with_capacity(normalized.len() / 2);
    let bytes = normalized.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        let hi = (bytes[idx] as char)
            .to_digit(16)
            .ok_or_else(|| "十六进制密钥包含非法字符".to_string())?;
        let lo = (bytes[idx + 1] as char)
            .to_digit(16)
            .ok_or_else(|| "十六进制密钥包含非法字符".to_string())?;
        out.push(((hi << 4) | lo) as u8);
        idx += 2;
    }
    Ok(out)
}

fn weixin_oc_parse_media_aes_key(aes_key_value: &str) -> Result<Vec<u8>, String> {
    let normalized = aes_key_value.trim();
    if normalized.is_empty() {
        return Err("媒体 AES 密钥为空".to_string());
    }
    let padded = format!(
        "{}{}",
        normalized,
        "=".repeat((4usize.wrapping_sub(normalized.len() % 4)) % 4)
    );
    let decoded = B64
        .decode(padded.as_bytes())
        .map_err(|err| format!("解析媒体 AES 密钥失败: {err}"))?;
    if decoded.len() == 16 {
        return Ok(decoded);
    }
    if decoded.len() == 32
        && decoded
            .iter()
            .all(|byte| (*byte as char).is_ascii_hexdigit())
    {
        let hex_text =
            std::str::from_utf8(&decoded).map_err(|err| format!("解析媒体 AES 十六进制失败: {err}"))?;
        return weixin_oc_decode_hex(hex_text);
    }
    Err("媒体 AES 密钥格式不支持".to_string())
}

fn weixin_oc_decrypt_media_ecb(encrypted: &[u8], key: &[u8]) -> Result<Vec<u8>, String> {
    use aes::cipher::{generic_array::GenericArray, BlockDecrypt, KeyInit};

    if key.len() != 16 {
        return Err(format!("媒体 AES 密钥长度不正确: {}", key.len()));
    }
    if encrypted.is_empty() {
        return Ok(Vec::new());
    }
    if encrypted.len() % 16 != 0 {
        return Err(format!("媒体密文长度不是 16 的倍数: {}", encrypted.len()));
    }
    let cipher = aes::Aes128::new_from_slice(key)
        .map_err(|err| format!("初始化媒体 AES 解密器失败: {err}"))?;
    let mut decrypted = encrypted.to_vec();
    for chunk in decrypted.chunks_exact_mut(16) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.decrypt_block(block);
    }
    Ok(weixin_oc_pkcs7_unpad(&decrypted))
}

async fn weixin_oc_download_image_bytes(
    client: &reqwest::Client,
    cdn_base_url: &str,
    encrypted_query_param: &str,
    aes_key_value: Option<&str>,
) -> Result<Vec<u8>, String> {
    let resp = client
        .get(weixin_oc_cdn_download_url(cdn_base_url, encrypted_query_param))
        .send()
        .await
        .map_err(|err| format!("下载个人微信图片失败: {err}"))?;
    let status = resp.status();
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("下载个人微信图片失败: status={} body={}", status, body));
    }
    let encrypted = resp
        .bytes()
        .await
        .map_err(|err| format!("读取个人微信图片响应失败: {err}"))?;
    if let Some(value) = aes_key_value.map(str::trim).filter(|value| !value.is_empty()) {
        let key = weixin_oc_parse_media_aes_key(value)?;
        return weixin_oc_decrypt_media_ecb(encrypted.as_ref(), &key);
    }
    Ok(encrypted.to_vec())
}

fn weixin_oc_normalize_image_mime(raw: &[u8]) -> String {
    match image::guess_format(raw) {
        Ok(image::ImageFormat::Png) => "image/png".to_string(),
        Ok(image::ImageFormat::Jpeg) => "image/jpeg".to_string(),
        Ok(image::ImageFormat::Gif) => "image/gif".to_string(),
        Ok(image::ImageFormat::WebP) => "image/webp".to_string(),
        _ => "image/jpeg".to_string(),
    }
}

fn weixin_oc_guess_attachment_mime(file_name: &str, fallback: &str) -> String {
    media_mime_from_path(std::path::Path::new(file_name))
        .unwrap_or(fallback)
        .to_string()
}

fn weixin_oc_build_attachment_meta(
    state: &AppState,
    file_name: &str,
    mime: &str,
    raw: &[u8],
) -> Result<(AttachmentMetaInput, String), String> {
    let saved = persist_raw_attachment_to_downloads(state, file_name, mime, raw)?;
    let relative_path = workspace_relative_path(state, &saved);
    let final_file_name = saved
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(file_name)
        .to_string();
    Ok((
        AttachmentMetaInput {
            file_name: final_file_name,
            relative_path: relative_path.clone(),
            mime: mime.to_string(),
        },
        relative_path,
    ))
}

fn weixin_oc_push_normalized_image_and_attachment(
    state: &AppState,
    file_name: &str,
    raw: &[u8],
    mime: &str,
    images: &mut Vec<BinaryPart>,
    attachments: &mut Vec<AttachmentMetaInput>,
) -> Option<String> {
    let normalized_image = match normalize_image_bytes_for_llm_request(raw, Some(mime)) {
        Ok(image) => Some(image),
        Err(err) => {
            eprintln!(
                "[远程IM][个人微信事件] 图片规范化失败，改按附件入队，file_name={}，mime={}，err={}",
                file_name, mime, err
            );
            return match weixin_oc_build_attachment_meta(state, file_name, mime, raw) {
                Ok((attachment, relative_path)) => {
                    attachments.push(attachment);
                    Some(build_attachment_notice_text("", &relative_path))
                }
                Err(save_err) => {
                    eprintln!(
                        "[远程IM][个人微信事件] 图片降级附件落盘失败，改仅保留文字提示，file_name={}，mime={}，err={}",
                        file_name, mime, save_err
                    );
                    Some(format!(
                        "[系统提示] 收到一张图片，但未能作为图片输入提供给模型，原因：{}。同时附件保存也失败：{}。",
                        err.trim(),
                        save_err.trim()
                    ))
                }
            };
        }
    };
    let attachment_raw = normalized_image
        .as_ref()
        .map(|image| image.bytes.as_slice())
        .unwrap_or(raw);
    let attachment_mime = normalized_image
        .as_ref()
        .map(|image| image.mime.as_str())
        .unwrap_or(mime)
        .to_string();
    let relative_path = match weixin_oc_build_attachment_meta(state, file_name, &attachment_mime, attachment_raw) {
        Ok((attachment, relative_path)) => {
            attachments.push(attachment);
            relative_path
        }
        Err(err) => {
            eprintln!(
                "[远程IM][个人微信事件] 图片附件落盘失败，继续保留模型图片输入，file_name={}，mime={}，err={}",
                file_name, attachment_mime, err
            );
            if let Some(image) = normalized_image {
                images.push(BinaryPart {
                    mime: image.mime,
                    bytes_base64: B64.encode(&image.bytes),
                    saved_path: None,
                });
            }
            return Some(format!(
                "[系统提示] 收到一张图片，已作为图片输入提供给模型，但附件保存失败：{}。",
                err.trim()
            ));
        }
    };
    if let Some(image) = normalized_image {
        images.push(BinaryPart {
            mime: attachment_mime,
            bytes_base64: B64.encode(&image.bytes),
            saved_path: Some(relative_path),
        });
    }
    None
}

async fn weixin_oc_collect_media(
    state: &AppState,
    client: &reqwest::Client,
    credentials: &WeixinOcCredentials,
    item_list: &[WeixinOcMessageItem],
) -> Result<WeixinOcCollectedMedia, String> {
    let mut images = Vec::<BinaryPart>::new();
    let mut audios = Vec::<BinaryPart>::new();
    let mut attachments = Vec::<AttachmentMetaInput>::new();
    let mut notices = Vec::<String>::new();
    let cdn_base_url = credentials.normalized_cdn_base_url();
    for item in item_list {
        let item_type = item.item_type.unwrap_or(0);
        let (media, file_name, fallback_mime, aes_key_override) = match item_type {
            2 => {
                let Some(image_item) = item.image_item.as_ref() else {
                    continue;
                };
                let Some(media) = image_item.media.as_ref() else {
                    continue;
                };
                (
                    media,
                    "image.jpg".to_string(),
                    "image/jpeg".to_string(),
                    image_item
                        .aeskey
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(|value| B64.encode(value)),
                )
            }
            3 => {
                let Some(voice_item) = item.voice_item.as_ref() else {
                    continue;
                };
                let Some(media) = voice_item.media.as_ref() else {
                    continue;
                };
                (
                    media,
                    "voice.silk".to_string(),
                    "audio/x-silk".to_string(),
                    None,
                )
            }
            4 => {
                let Some(file_item) = item.file_item.as_ref() else {
                    continue;
                };
                let Some(media) = file_item.media.as_ref() else {
                    continue;
                };
                let file_name = file_item
                    .file_name
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("file.bin")
                    .to_string();
                let mime = weixin_oc_guess_attachment_mime(&file_name, "application/octet-stream");
                (
                    media,
                    file_name.clone(),
                    mime,
                    None,
                )
            }
            5 => {
                let Some(video_item) = item.video_item.as_ref() else {
                    continue;
                };
                let Some(media) = video_item.media.as_ref() else {
                    continue;
                };
                (
                    media,
                    "video.mp4".to_string(),
                    "video/mp4".to_string(),
                    None,
                )
            }
            _ => continue,
        };
        let Some(encrypted_query_param) = media
            .encrypt_query_param
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let aes_key_value = aes_key_override.or_else(|| {
            media.aes_key
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        });
        let raw = weixin_oc_download_image_bytes(
            client,
            &cdn_base_url,
            encrypted_query_param,
            aes_key_value.as_deref(),
        )
        .await?;
        let mime = if item_type == 2 {
            weixin_oc_normalize_image_mime(&raw)
        } else {
            fallback_mime
        };
        match item_type {
            2 => {
                if let Some(notice) = weixin_oc_push_normalized_image_and_attachment(
                    state,
                    &file_name,
                    &raw,
                    &mime,
                    &mut images,
                    &mut attachments,
                ) {
                    notices.push(notice);
                }
            }
            3 => {
                let (attachment, relative_path) =
                    weixin_oc_build_attachment_meta(state, &file_name, &mime, &raw)?;
                attachments.push(attachment);
                audios.push(BinaryPart {
                    mime,
                    bytes_base64: B64.encode(&raw),
                    saved_path: Some(relative_path),
                });
            }
            4 | 5 => {
                let (attachment, _) = weixin_oc_build_attachment_meta(state, &file_name, &mime, &raw)?;
                attachments.push(attachment);
            }
            _ => {}
        }
    }
    Ok(WeixinOcCollectedMedia {
        images,
        audios,
        attachments,
        notices,
    })
}

#[derive(Debug, Deserialize)]
struct WeixinOcGetUploadUrlResp {
    #[serde(default)]
    ret: i64,
    #[serde(default)]
    errcode: i64,
    #[serde(default)]
    errmsg: String,
    #[serde(default)]
    #[serde(alias = "uploadParam")]
    upload_param: String,
    #[serde(default)]
    #[serde(alias = "uploadFullUrl")]
    upload_full_url: String,
}

fn weixin_oc_media_aes_key_hex() -> String {
    weixin_oc_encode_hex(Uuid::new_v4().as_bytes())
}

fn weixin_oc_random_hex_id() -> String {
    Uuid::new_v4().simple().to_string()
}

async fn weixin_oc_request_upload_url(
    client: &reqwest::Client,
    credentials: &WeixinOcCredentials,
    to_user_id: &str,
    file_key: &str,
    raw: &[u8],
    upload_media_type: i64,
    aes_key_hex: &str,
) -> Result<WeixinOcGetUploadUrlResp, String> {
    let ciphertext_size = weixin_oc_aes_padded_size(raw.len());
    let body = serde_json::json!({
        "filekey": file_key,
        "media_type": upload_media_type,
        "to_user_id": to_user_id,
        "rawsize": raw.len(),
        "rawfilemd5": format!("{:x}", md5::compute(raw)),
        "filesize": ciphertext_size,
        "no_need_thumb": true,
        "aeskey": aes_key_hex,
        "base_info": {
            "channel_version": "easy_call_ai"
        }
    });
    let body_text = serde_json::to_string(&body)
        .map_err(|err| format!("序列化 getuploadurl 请求失败: {err}"))?;
    let headers = weixin_oc_request_headers(&body_text, Some(credentials.token.as_str()))?;
    let resp = client
        .post(format!(
            "{}/ilink/bot/getuploadurl",
            credentials.normalized_base_url().trim_end_matches('/')
        ))
        .headers(headers)
        .body(body_text)
        .send()
        .await
        .map_err(|err| format!("请求 getuploadurl 失败: {err}"))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .map_err(|err| format!("读取 getuploadurl 响应失败: {err}"))?;
    if !status.is_success() {
        return Err(format!(
            "请求 getuploadurl 失败: status={} body={}",
            status, text
        ));
    }
    let parsed = serde_json::from_str::<WeixinOcGetUploadUrlResp>(&text)
        .map_err(|err| format!("解析 getuploadurl 响应失败: {err}, body={text}"))?;
    if parsed.ret != 0 || parsed.errcode != 0 {
        return Err(format!(
            "请求 getuploadurl 失败: ret={} errcode={} errmsg={}",
            parsed.ret, parsed.errcode, parsed.errmsg
        ));
    }
    if parsed.upload_param.trim().is_empty() && parsed.upload_full_url.trim().is_empty() {
        return Err("请求 getuploadurl 失败: 返回中缺少 upload_param / upload_full_url".to_string());
    }
    runtime_log_info(format!(
        "[个人微信媒体发送] getuploadurl 完成: to_user_id={}, media_type={}, raw_size={}, upload_param_len={}, upload_full_url_present={}",
        to_user_id.trim(),
        upload_media_type,
        raw.len(),
        parsed.upload_param.len(),
        !parsed.upload_full_url.trim().is_empty()
    ));
    Ok(parsed)
}

async fn weixin_oc_upload_to_cdn(
    client: &reqwest::Client,
    credentials: &WeixinOcCredentials,
    upload_param: &str,
    upload_full_url: &str,
    file_key: &str,
    aes_key_hex: &str,
    raw: &[u8],
) -> Result<String, String> {
    let key = weixin_oc_decode_hex(aes_key_hex)?;
    let encrypted = weixin_oc_encrypt_media_ecb(raw, &key)?;
    let upload_url = if !upload_full_url.trim().is_empty() {
        upload_full_url.trim().to_string()
    } else {
        weixin_oc_cdn_upload_url(
            credentials.normalized_cdn_base_url().as_str(),
            upload_param,
            file_key,
        )
    };
    let resp = client
        .post(upload_url)
        .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
        .body(encrypted)
        .send()
        .await
        .map_err(|err| format!("上传个人微信媒体到 CDN 失败: {err}"))?;
    let status = resp.status();
    let encrypted_query_param = resp
        .headers()
        .get("x-encrypted-param")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .unwrap_or("")
        .to_string();
    let body = resp.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(format!(
            "上传个人微信媒体到 CDN 失败: status={} body={}",
            status, body
        ));
    }
    if encrypted_query_param.is_empty() {
        return Err(format!(
            "上传个人微信媒体到 CDN 失败: 响应缺少 x-encrypted-param, body={body}"
        ));
    }
    runtime_log_info(format!(
        "[个人微信媒体发送] CDN 上传 完成: file_key={}, cipher_size={}, encrypted_query_param_len={}",
        file_key.trim(),
        weixin_oc_aes_padded_size(raw.len()),
        encrypted_query_param.len()
    ));
    Ok(encrypted_query_param)
}

async fn weixin_oc_prepare_outbound_media_item(
    client: &reqwest::Client,
    credentials: &WeixinOcCredentials,
    to_user_id: &str,
    upload_media_type: i64,
    item_type: i64,
    file_name: &str,
    raw: &[u8],
) -> Result<Value, String> {
    runtime_log_info(format!(
        "[个人微信媒体发送] 开始准备媒体: to_user_id={}, item_type={}, upload_media_type={}, file_name={}, raw_size={}",
        to_user_id.trim(),
        item_type,
        upload_media_type,
        file_name.trim(),
        raw.len()
    ));
    let file_key = weixin_oc_random_hex_id();
    let aes_key_hex = weixin_oc_media_aes_key_hex();
    let upload = weixin_oc_request_upload_url(
        client,
        credentials,
        to_user_id,
        &file_key,
        raw,
        upload_media_type,
        &aes_key_hex,
    )
    .await?;
    let encrypted_query_param = weixin_oc_upload_to_cdn(
        client,
        credentials,
        upload.upload_param.as_str(),
        upload.upload_full_url.as_str(),
        &file_key,
        &aes_key_hex,
        raw,
    )
    .await?;
    let media_payload = serde_json::json!({
        "encrypt_query_param": encrypted_query_param,
        "aes_key": B64.encode(aes_key_hex.as_bytes()),
        "encrypt_type": 1,
    });
    let ciphertext_size = weixin_oc_aes_padded_size(raw.len());
    Ok(match item_type {
        WEIXIN_OC_IMAGE_ITEM_TYPE => serde_json::json!({
            "type": WEIXIN_OC_IMAGE_ITEM_TYPE,
            "image_item": {
                "media": media_payload,
                "mid_size": ciphertext_size,
            }
        }),
        WEIXIN_OC_FILE_ITEM_TYPE => serde_json::json!({
            "type": WEIXIN_OC_FILE_ITEM_TYPE,
            "file_item": {
                "media": media_payload,
                "file_name": file_name,
                "len": raw.len().to_string(),
            }
        }),
        WEIXIN_OC_VIDEO_ITEM_TYPE => serde_json::json!({
            "type": WEIXIN_OC_VIDEO_ITEM_TYPE,
            "video_item": {
                "media": media_payload,
                "video_size": ciphertext_size,
            }
        }),
        _ => {
            return Err(format!("个人微信媒体类型不支持: item_type={item_type}"));
        }
    })
}

