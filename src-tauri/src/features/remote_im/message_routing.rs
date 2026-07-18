fn remote_im_contact_display_name(contact: &RemoteImContact) -> String {
    let remark = contact.remark_name.trim();
    if !remark.is_empty() {
        return remark.to_string();
    }
    let remote_name = contact.remote_contact_name.trim();
    if !remote_name.is_empty() {
        return remote_name.to_string();
    }
    contact.remote_contact_id.trim().to_string()
}

#[derive(Debug, Clone)]
struct RemoteImOutboundContentDigest {
    text_preview: String,
    text_count: usize,
    image_count: usize,
    file_count: usize,
    other_count: usize,
}

fn remote_im_presence_state_label(state: RemoteImPresenceState) -> &'static str {
    match state {
        RemoteImPresenceState::Away => "离场",
        RemoteImPresenceState::Present => "在场",
    }
}

fn remote_im_work_state_label(state: RemoteImWorkState) -> &'static str {
    match state {
        RemoteImWorkState::Idle => "空闲",
        RemoteImWorkState::Busy => "忙碌",
    }
}

fn remote_im_yes_no(value: bool) -> &'static str {
    if value { "是" } else { "否" }
}

fn remote_im_preview_text(text: &str, max_chars: usize) -> String {
    let normalized = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.is_empty() {
        return "（无文本）".to_string();
    }
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    format!(
        "{}...",
        normalized.chars().take(max_chars).collect::<String>()
    )
}

fn remote_im_contact_log_label(contact: &RemoteImContact) -> String {
    format!(
        "{}[{}:{}]",
        remote_im_contact_display_name(contact),
        contact.remote_contact_type.trim(),
        contact.remote_contact_id.trim()
    )
}

fn remote_im_contact_log_marker(contact: &RemoteImContact) -> String {
    format!(
        "[{}:{}]",
        contact.remote_contact_type.trim(),
        contact.remote_contact_id.trim()
    )
}

fn remote_im_contact_downloads_segment(value: &str, fallback: &str) -> String {
    let sanitized = sanitize_download_file_name(value);
    let trimmed = sanitized.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn remote_im_contact_downloads_subdir_parts(
    channel_id: &str,
    contact_type: &str,
    contact_id: &str,
) -> String {
    format!(
        "contacts/{}/{}/{}/downloads",
        remote_im_contact_downloads_segment(channel_id, "unknown-channel"),
        remote_im_contact_downloads_segment(contact_type, "unknown-type"),
        remote_im_contact_downloads_segment(contact_id, "unknown-contact")
    )
}

fn remote_im_contact_downloads_subdir(contact: &RemoteImContact) -> String {
    remote_im_contact_downloads_subdir_parts(
        &contact.channel_id,
        &contact.remote_contact_type,
        &contact.remote_contact_id,
    )
}

fn remote_im_contact_downloads_relative_dir(contact: &RemoteImContact) -> String {
    format!("downloads/{}", remote_im_contact_downloads_subdir(contact))
}

fn remote_im_activation_source_log_label(source: &RemoteImActivationSource) -> String {
    let display_name = source.remote_contact_name.trim();
    let name = if display_name.is_empty() {
        source.remote_contact_id.trim()
    } else {
        display_name
    };
    format!(
        "{}[{}:{}]",
        name,
        source.remote_contact_type.trim(),
        source.remote_contact_id.trim()
    )
}

fn remote_im_outbound_content_digest(content: &[Value]) -> RemoteImOutboundContentDigest {
    let mut text_count = 0usize;
    let mut image_count = 0usize;
    let mut file_count = 0usize;
    let mut other_count = 0usize;
    let mut text_fragments = Vec::<String>::new();
    let mut asset_names = Vec::<String>::new();
    for item in content {
        match item.get("type").and_then(Value::as_str).unwrap_or("") {
            "text" => {
                text_count += 1;
                let text = item
                    .get("text")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                    .unwrap_or("");
                if !text.is_empty() {
                    text_fragments.push(text.to_string());
                }
            }
            "image" => {
                image_count += 1;
                if let Some(name) = item
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    asset_names.push(name.to_string());
                }
            }
            "file" => {
                file_count += 1;
                if let Some(name) = item
                    .get("name")
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    asset_names.push(name.to_string());
                }
            }
            _ => {
                other_count += 1;
            }
        }
    }
    let preview_source = if !text_fragments.is_empty() {
        text_fragments.join(" / ")
    } else if !asset_names.is_empty() {
        asset_names.join(", ")
    } else if image_count + file_count + other_count > 0 {
        format!("附件 {} 个", image_count + file_count + other_count)
    } else {
        String::new()
    };
    RemoteImOutboundContentDigest {
        text_preview: remote_im_preview_text(&preview_source, 100),
        text_count,
        image_count,
        file_count,
        other_count,
    }
}

#[cfg(not(test))]
fn remote_im_append_channel_log(channel_id: &str, level: &str, message: String) {
    let channel_id = channel_id.trim().to_string();
    let level = level.trim().to_string();
    let message = message.trim().to_string();
    if channel_id.is_empty() || level.is_empty() || message.is_empty() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        onebot_v11_ws_manager()
            .add_log(&channel_id, &level, &message)
            .await;
    });
}

#[cfg(test)]
fn remote_im_append_channel_log(channel_id: &str, level: &str, message: String) {
    let _ = (channel_id, level, message);
}

#[cfg(not(test))]
async fn remote_im_append_channel_log_async(channel_id: &str, level: &str, message: String) {
    let channel_id = channel_id.trim().to_string();
    let level = level.trim().to_string();
    let message = message.trim().to_string();
    if channel_id.is_empty() || level.is_empty() || message.is_empty() {
        return;
    }
    onebot_v11_ws_manager()
        .add_log(&channel_id, &level, &message)
        .await;
}

#[cfg(test)]
async fn remote_im_append_channel_log_async(channel_id: &str, level: &str, message: String) {
    let _ = (channel_id, level, message);
}

fn remote_im_resolve_contact_log_query(
    state: &AppState,
    contact_id: &str,
) -> Result<(String, String), String> {
    let normalized_contact_id = contact_id.trim();
    if normalized_contact_id.is_empty() {
        return Err("contact_id 为必填项。".to_string());
    }
    let runtime = state_read_runtime_state_cached(state)?;
    let contact = runtime
        .remote_im_contacts
        .iter()
        .find(|item| item.id == normalized_contact_id)
        .ok_or_else(|| format!("未找到远程联系人：{normalized_contact_id}"))?;
    Ok((
        contact.channel_id.trim().to_string(),
        remote_im_contact_log_marker(contact),
    ))
}

fn remote_im_filter_channel_logs_for_contact(
    logs: Vec<ChannelLogEntry>,
    contact_marker: &str,
) -> Vec<ChannelLogEntry> {
    let normalized_marker = contact_marker.trim();
    if normalized_marker.is_empty() {
        return Vec::new();
    }
    logs.into_iter()
        .filter(|entry| entry.message.contains(normalized_marker))
        .collect()
}

fn remote_im_resolve_effective_route_mode(
    _config: &AppConfig,
    _contact: &RemoteImContact,
) -> String {
    "dedicated_contact_conversation".to_string()
}

fn remote_im_contact_conversation_title(contact: &RemoteImContact) -> String {
    format!("联系人 · {}", remote_im_contact_display_name(contact))
}

fn remote_im_contact_conversation_key_parts(
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
) -> String {
    format!(
        "remote_im_contact:{}:{}:{}",
        channel_id.trim(),
        remote_contact_type.trim().to_ascii_lowercase(),
        remote_contact_id.trim()
    )
}

fn remote_im_contact_conversation_key(contact: &RemoteImContact) -> String {
    remote_im_contact_conversation_key_parts(
        &contact.channel_id,
        &contact.remote_contact_type,
        &contact.remote_contact_id,
    )
}

fn remote_im_set_sender_origin_meta(
    input: &RemoteImEnqueueInput,
    conversation_id: &str,
    contact_record_id: &str,
) -> Value {
    serde_json::json!({
        "origin": {
            "kind": "remote_im",
            "channel_id": input.channel_id,
            "platform": input.platform,
            "im_name": input.im_name,
            "contact_type": input.remote_contact_type,
            "contact_id": input.remote_contact_id,
            "contact_name": input.remote_contact_name,
            "contact_record_id": contact_record_id,
            "sender_id": input.sender_id,
            "sender_name": input.sender_name,
            "sender_avatar_url": input.sender_avatar_url,
            "platform_message_id": input.platform_message_id,
            "conversation_id": conversation_id
        }
    })
}

fn remote_im_resolve_inbound_activate(
    channel: &RemoteImChannelConfig,
    message_flag: Option<bool>,
) -> bool {
    message_flag.unwrap_or(channel.activate_assistant)
}

fn origin_value_string<'a>(origin: &'a Value, key: &str) -> Option<&'a str> {
    origin
        .get(key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn message_origin_string<'a>(message: &'a ChatMessage, key: &str) -> Option<&'a str> {
    let origin = message.provider_meta.as_ref()?.get("origin")?;
    origin_value_string(origin, key)
}

fn message_has_remote_im_platform_message(
    message: &ChatMessage,
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
    platform_message_id: &str,
) -> bool {
    message_origin_string(message, "kind") == Some("remote_im")
        && message_origin_string(message, "channel_id") == Some(channel_id)
        && message_origin_string(message, "contact_type") == Some(remote_contact_type)
        && message_origin_string(message, "contact_id") == Some(remote_contact_id)
        && message_origin_string(message, "platform_message_id") == Some(platform_message_id)
}

fn ready_store_has_remote_im_platform_message(
    state: &AppState,
    conversation_id: &str,
    channel_id: &str,
    remote_contact_type: &str,
    remote_contact_id: &str,
    platform_message_id: &str,
) -> Result<bool, String> {
    let paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
    let Some(page) = message_store::read_ready_message_store_block_page(&paths, None)? else {
        return Ok(false);
    };
    for block in page.blocks.into_iter().rev() {
        let Some(block_page) =
            message_store::read_ready_message_store_block_page(&paths, Some(block.block_id))?
        else {
            continue;
        };
        if block_page.messages.iter().any(|message| {
            message_has_remote_im_platform_message(
                message,
                channel_id,
                remote_contact_type,
                remote_contact_id,
                platform_message_id,
            )
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

struct ValidatedEnqueueInput {
    text: String,
    images: Vec<BinaryPart>,
    audios: Vec<BinaryPart>,
    attachments: Vec<AttachmentMetaInput>,
    channel: RemoteImChannelConfig,
}
