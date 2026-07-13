#[derive(Debug, Clone, PartialEq, Eq)]
struct MessageStoreVerificationReport {
    message_count: usize,
    last_message_id: String,
    compaction_count: usize,
    index: MessageStoreIndexFile,
    last_valid_offset: u64,
}

fn verify_jsonl_snapshot_content(
    content: &str,
    expected_message_count: usize,
    expected_last_message_id: &str,
) -> Result<MessageStoreVerificationReport, String> {
    let mut offset = 0_u64;
    let mut items = Vec::<MessageStoreIndexItem>::new();
    let mut compaction_count = 0_usize;
    let mut last_message_id = String::new();

    for raw_line in content.split_inclusive('\n') {
        let byte_len = raw_line.as_bytes().len() as u64;
        let line = raw_line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            offset += byte_len;
            continue;
        }
        let message = decode_jsonl_snapshot_message(line)
            .map_err(|err| format!("JSONL 校验失败，offset={offset}: {err}"))?;
        let item = message_store_index_item_for_message(&message, offset, byte_len);
        if item.compaction_kind.is_some() {
            compaction_count += 1;
        }
        last_message_id = item.message_id.clone();
        items.push(item);
        offset += byte_len;
    }

    if !content.is_empty() && !content.ends_with('\n') {
        return Err("JSONL 校验失败: 文件末尾存在未换行的半行".to_string());
    }
    if expected_message_count != usize::MAX && items.len() != expected_message_count {
        return Err(format!(
            "JSONL 校验失败: 消息数量不一致，expected={}, actual={}",
            expected_message_count,
            items.len()
        ));
    }
    let expected_last_message_id = expected_last_message_id.trim();
    if !expected_last_message_id.is_empty() && last_message_id != expected_last_message_id {
        return Err(format!(
            "JSONL 校验失败: 最后一条消息不一致，expected={}, actual={}",
            expected_last_message_id,
            last_message_id
        ));
    }

    Ok(MessageStoreVerificationReport {
        message_count: items.len(),
        last_message_id,
        compaction_count,
        index: MessageStoreIndexFile::new(MESSAGE_STORE_MANIFEST_VERSION, items),
        last_valid_offset: offset,
    })
}

fn verify_jsonl_snapshot_file(
    path: &PathBuf,
    expected_message_count: usize,
    expected_last_message_id: &str,
) -> Result<MessageStoreVerificationReport, String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("读取 JSONL 快照失败，path={}，error={err}", path.display()))?;
    verify_jsonl_snapshot_content(&content, expected_message_count, expected_last_message_id)
}

fn repair_jsonl_snapshot_file(
    path: &PathBuf,
    seen_message_ids: &mut std::collections::HashSet<String>,
) -> Result<(MessageStoreVerificationReport, usize, usize), String> {
    let content = fs::read_to_string(path)
        .map_err(|err| format!("读取 JSONL 快照失败，path={}，error={err}", path.display()))?;
    let mut repaired = String::new();
    let mut discarded = 0usize;
    let mut duplicate = 0usize;
    for raw_line in content.split_inclusive('\n') {
        let line = raw_line.trim_end_matches(['\r', '\n']);
        if line.trim().is_empty() {
            continue;
        }
        match decode_jsonl_snapshot_message(line) {
            Ok(message) => {
                let message_id = message.id.trim().to_string();
                if message_id.is_empty() {
                    discarded += 1;
                } else if seen_message_ids.insert(message_id) {
                    repaired.push_str(&encode_jsonl_snapshot_message(&message)?);
                } else {
                    duplicate += 1;
                }
            }
            Err(_) => discarded += 1,
        }
    }
    if !content.is_empty() && !content.ends_with('\n') {
        let tail = content.rsplit_once('\n').map(|(_, tail)| tail).unwrap_or(&content);
        if !tail.trim().is_empty() && decode_jsonl_snapshot_message(tail).is_err() {
            discarded = discarded.max(1);
        }
    }
    if repaired != content {
        write_message_store_text_atomic(path, "repair.tmp", &repaired, "JSONL block 修复")?;
    }
    let report = verify_jsonl_snapshot_content(&repaired, usize::MAX, "")?;
    Ok((report, discarded, duplicate))
}

fn rebuild_jsonl_snapshot_index_from_file(path: &PathBuf) -> Result<MessageStoreIndexFile, String> {
    let report = verify_jsonl_snapshot_file(path, usize::MAX, "")?;
    Ok(report.index)
}
