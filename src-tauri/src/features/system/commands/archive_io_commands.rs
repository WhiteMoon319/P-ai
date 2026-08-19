use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportArchiveToFileInput {
    pub(crate) archive_id: String,
    pub(crate) format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExportArchiveFileResult {
    pub(crate) path: String,
    pub(crate) archive_id: String,
    pub(crate) format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveExportPayload {
    pub(crate) version: u32,
    pub(crate) exported_at: String,
    pub(crate) archive: ConversationArchive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportArchivesFromJsonInput {
    pub(crate) payload_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ImportArchivesResult {
    pub(crate) imported_count: usize,
    pub(crate) replaced_count: usize,
    pub(crate) skipped_count: usize,
    pub(crate) total_count: usize,
    pub(crate) selected_archive_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveImportBatchPayload {
    pub(crate) archives: Vec<ConversationArchive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveImportAppDataPayload {
    pub(crate) archived_conversations: Vec<ConversationArchive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ArchiveImportConversationsPayload {
    pub(crate) conversations: Vec<Conversation>,
}

pub(crate) fn parse_archives_for_import(raw: &str) -> Result<Vec<ConversationArchive>, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Archive payload is empty".to_string());
    }
    if let Ok(payload) = serde_json::from_str::<ArchiveExportPayload>(trimmed) {
        return Ok(vec![payload.archive]);
    }
    if let Ok(archive) = serde_json::from_str::<ConversationArchive>(trimmed) {
        return Ok(vec![archive]);
    }
    if let Ok(batch) = serde_json::from_str::<ArchiveImportBatchPayload>(trimmed) {
        if !batch.archives.is_empty() {
            return Ok(batch.archives);
        }
    }
    if let Ok(batch) = serde_json::from_str::<ArchiveImportAppDataPayload>(trimmed) {
        if !batch.archived_conversations.is_empty() {
            return Ok(batch.archived_conversations);
        }
    }
    if let Ok(batch) = serde_json::from_str::<ArchiveImportConversationsPayload>(trimmed) {
        let out = batch
            .conversations
            .into_iter()
            .filter(|c| !c.summary.trim().is_empty())
            .map(|c| conversation_to_archive(&c))
            .collect::<Vec<_>>();
        if !out.is_empty() {
            return Ok(out);
        }
    }
    if let Ok(list) = serde_json::from_str::<Vec<ConversationArchive>>(trimmed) {
        if !list.is_empty() {
            return Ok(list);
        }
    }
    Err("Invalid archive payload. Expected exported archive JSON.".to_string())
}

pub(crate) fn normalize_archive_for_import(archive: &mut ConversationArchive, data_path: &PathBuf) {
    if archive.archive_id.trim().is_empty() {
        archive.archive_id = Uuid::new_v4().to_string();
    }
    if archive.archived_at.trim().is_empty() {
        archive.archived_at = now_iso();
    }
    archive.reason = clean_text(archive.reason.trim());
    if archive.reason.is_empty() {
        archive.reason = "import_archive".to_string();
    }
    let conversation = &mut archive.source_conversation;
    if conversation.id.trim().is_empty() {
        conversation.id = Uuid::new_v4().to_string();
    }
    conversation.title = clean_text(conversation.title.trim());
    if conversation.title.is_empty() {
        conversation.title = format!("Imported {}", archive_time_label(&archive.archived_at));
    }
    if conversation.created_at.trim().is_empty() {
        conversation.created_at = archive.archived_at.clone();
    }
    if conversation.updated_at.trim().is_empty() {
        conversation.updated_at = conversation.created_at.clone();
    }
    conversation.status = "archived".to_string();
    conversation.fast_request_turns.clear();
    if conversation.last_user_at.as_ref().map(|v| v.trim().is_empty()).unwrap_or(false) {
        conversation.last_user_at = None;
    }
    if conversation
        .last_assistant_at
        .as_ref()
        .map(|v| v.trim().is_empty())
        .unwrap_or(false)
    {
        conversation.last_assistant_at = None;
    }
    for message in &mut conversation.messages {
        if message.id.trim().is_empty() {
            message.id = Uuid::new_v4().to_string();
        }
        if message.created_at.trim().is_empty() {
            message.created_at = conversation.updated_at.clone();
        }
        message.role = clean_text(message.role.trim());
        if message.role.is_empty() {
            message.role = "user".to_string();
        }
        for part in &mut message.parts {
            match part {
                MessagePart::Text { text, .. } => {
                    *text = clean_text(text.trim());
                }
                MessagePart::Image {
                    mime,
                    bytes_base64,
                    name,
                    ..
                } => {
                    *mime = clean_text(mime.trim());
                    if mime.is_empty() {
                        *mime = "image/webp".to_string();
                    }
                    *bytes_base64 = bytes_base64.trim().to_string();
                    *name = name
                        .as_ref()
                        .map(|v| clean_text(v.trim()))
                        .filter(|v| !v.is_empty());
                }
                MessagePart::Audio {
                    mime,
                    bytes_base64,
                    name,
                    ..
                } => {
                    *mime = clean_text(mime.trim());
                    if mime.is_empty() {
                        *mime = "audio/webm".to_string();
                    }
                    *bytes_base64 = bytes_base64.trim().to_string();
                    *name = name
                        .as_ref()
                        .map(|v| clean_text(v.trim()))
                        .filter(|v| !v.is_empty());
                }
                MessagePart::Attachment { path, mime, name } => {
                    *path = clean_text(path.trim());
                    *mime = clean_text(mime.trim());
                    if mime.is_empty() {
                        *mime = "application/octet-stream".to_string();
                    }
                    *name = clean_text(name.trim());
                    if name.is_empty() {
                        *name = std::path::Path::new(path)
                            .file_name()
                            .and_then(|value| value.to_str())
                            .unwrap_or("attachment")
                            .to_string();
                    }
                }
            }
        }
        canonicalize_message_parts_for_persistence(&mut message.parts, data_path);
        message.provider_meta =
            provider_meta_without_legacy_attachments(message.provider_meta.take());
        message
            .extra_text_blocks
            .iter_mut()
            .for_each(|text| *text = clean_text(text.trim()));
        message.extra_text_blocks.retain(|text| !text.is_empty());
    }
}

pub(crate) fn archive_message_plain_text(message: &ChatMessage) -> String {
    message
        .parts
        .iter()
        .filter_map(|part| match part {
            MessagePart::Text { text, .. } => Some(text.trim().to_string()),
            _ => None,
        })
        .filter(|text| !text.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn archive_message_image_count(message: &ChatMessage) -> usize {
    message
        .parts
        .iter()
        .filter(|part| {
            matches!(part, MessagePart::Image { .. })
                || matches!(part, MessagePart::Attachment { mime, .. } if matches!(message_attachment_kind(mime), "image" | "pdf"))
        })
        .count()
}

pub(crate) fn archive_message_audio_count(message: &ChatMessage) -> usize {
    message
        .parts
        .iter()
        .filter(|part| {
            matches!(part, MessagePart::Audio { .. })
                || matches!(part, MessagePart::Attachment { mime, .. } if message_attachment_kind(mime) == "audio")
        })
        .count()
}

pub(crate) fn tool_call_markdown_lines(message: &ChatMessage) -> Vec<String> {
    tool_history_markdown_lines_from_message(message)
}

pub(crate) fn archive_message_markdown_block(message: &ChatMessage) -> String {
    let role = match message.role.as_str() {
        "user" => "用户",
        "assistant" => "助手",
        "tool" => "工具",
        other => other,
    };
    let mut lines = Vec::<String>::new();
    lines.push(format!("### {}  {}", role, message.created_at));

    let text = archive_message_plain_text(message);
    if !text.is_empty() {
        lines.push(text);
    }

    let image_count = archive_message_image_count(message);
    if image_count > 0 {
        lines.push(format!("- 图片 x{image_count}"));
    }
    let audio_count = archive_message_audio_count(message);
    if audio_count > 0 {
        lines.push(format!("- 音频 x{audio_count}"));
    }

    for line in tool_call_markdown_lines(message) {
        lines.push(line);
    }

    if lines.len() == 1 {
        lines.push("- (空消息)".to_string());
    }
    lines.join("\n")
}

pub(crate) fn build_archive_markdown(archive: &ConversationArchive) -> String {
    let mut blocks = Vec::<String>::new();
    blocks.push("# 对话归档".to_string());
    blocks.push(format!("- 标题: {}", archive.source_conversation.title));
    blocks.push(format!("- 归档时间: {}", archive.archived_at));
    if !archive.source_conversation.summary.trim().is_empty() {
        blocks.push(String::new());
        blocks.push("## 摘要".to_string());
        blocks.push(archive.source_conversation.summary.trim().to_string());
    }
    blocks.push(String::new());
    blocks.push("## 消息时间线".to_string());
    for message in &archive.source_conversation.messages {
        let role = message.role.as_str();
        if role != "user" && role != "assistant" && role != "tool" {
            continue;
        }
        blocks.push(String::new());
        blocks.push(archive_message_markdown_block(message));
    }
    blocks.join("\n")
}



pub(crate) fn import_archives_from_json_inner(
    input: ImportArchivesFromJsonInput,
    state: &AppState,
) -> Result<ImportArchivesResult, String> {
    let mut incoming_archives = parse_archives_for_import(&input.payload_json)?;
    if incoming_archives.is_empty() {
        return Err("No archives found in payload.".to_string());
    }

    let result = conversation_service_v2().import_archives(state, &mut incoming_archives)?;
    Ok(ImportArchivesResult {
        imported_count: result.imported_count,
        replaced_count: result.replaced_count,
        skipped_count: result.skipped_count,
        total_count: result.total_count,
        selected_archive_id: result.selected_archive_id,
    })
}
