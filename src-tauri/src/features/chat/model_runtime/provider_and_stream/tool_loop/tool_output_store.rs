const TOOL_OUTPUT_MAX_LINES: usize = 2_000;
const TOOL_OUTPUT_MAX_BYTES: usize = 50 * 1024;
const TOOL_OUTPUT_RETENTION_SECS: u64 = 7 * 24 * 60 * 60;
static TOOL_OUTPUT_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn tool_output_line_count(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count().saturating_add(1)
}

fn tool_output_take_prefix(text: &str, max_bytes: usize) -> String {
    let mut end = 0usize;
    for (index, ch) in text.char_indices() {
        let next = index.saturating_add(ch.len_utf8());
        if next > max_bytes { break; }
        end = next;
    }
    text[..end].to_string()
}

fn tool_output_take_suffix(text: &str, max_bytes: usize) -> String {
    let mut start = text.len();
    let mut bytes = 0usize;
    for (index, ch) in text.char_indices().rev() {
        let size = ch.len_utf8();
        if bytes.saturating_add(size) > max_bytes { break; }
        start = index;
        bytes = bytes.saturating_add(size);
    }
    text[start..].to_string()
}

fn tool_output_preview(text: &str, marker: &str) -> String {
    let marker_bytes = marker.len();
    let text_budget = TOOL_OUTPUT_MAX_BYTES.saturating_sub(marker_bytes.saturating_add(4));
    let head_bytes = text_budget.div_ceil(2);
    let tail_bytes = text_budget / 2;
    let lines = text.lines().collect::<Vec<_>>();
    let (head_source, tail_source) = if lines.len() > TOOL_OUTPUT_MAX_LINES {
        let head_lines = TOOL_OUTPUT_MAX_LINES.div_ceil(2).saturating_sub(2);
        let tail_lines = (TOOL_OUTPUT_MAX_LINES / 2).saturating_sub(2);
        (
            lines[..head_lines].join("\n"),
            lines[lines.len().saturating_sub(tail_lines)..].join("\n"),
        )
    } else {
        (text.to_string(), text.to_string())
    };
    format!("{}\n\n{}\n\n{}", tool_output_take_prefix(&head_source, head_bytes), marker, tool_output_take_suffix(&tail_source, tail_bytes))
}

fn tool_output_directory_from_workspace(llm_workspace_path: &std::path::Path) -> std::path::PathBuf {
    llm_workspace_path.join("tool-output")
}

fn tool_output_directory(state: &AppState) -> std::path::PathBuf {
    tool_output_directory_from_workspace(&state.llm_workspace_path)
}

fn cleanup_expired_tool_outputs(directory: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(directory) else { return; };
    let now = std::time::SystemTime::now();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() { continue; }
        let expired = entry.metadata().ok().and_then(|meta| meta.modified().ok())
            .and_then(|modified| now.duration_since(modified).ok())
            .is_some_and(|age| age.as_secs() > TOOL_OUTPUT_RETENTION_SECS);
        if expired { let _ = std::fs::remove_file(path); }
    }
}

fn store_full_tool_output_at(directory: &std::path::Path, text: &str) -> Result<std::path::PathBuf, String> {
    std::fs::create_dir_all(&directory).map_err(|err| format!("创建工具输出目录失败: {err}"))?;
    cleanup_expired_tool_outputs(directory);
    let sequence = TOOL_OUTPUT_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let file = directory.join(format!("tool_{}_{}.txt", chrono::Utc::now().timestamp_millis(), sequence));
    std::fs::write(&file, text).map_err(|err| format!("保存完整工具输出失败: {err}"))?;
    Ok(file)
}

fn store_full_tool_output(state: &AppState, text: &str) -> Result<std::path::PathBuf, String> {
    store_full_tool_output_at(&tool_output_directory(state), text)
}

fn bound_provider_tool_result(state: Option<&AppState>, result: ProviderToolResult) -> ProviderToolResult {
    let mut contextual = result.display_text.clone();
    for part in &result.parts {
        let extra = match part {
            ProviderToolResultPart::Text { text } => text,
            ProviderToolResultPart::Resource { text, .. } => text,
            ProviderToolResultPart::Image { .. } | ProviderToolResultPart::Audio { .. } => continue,
        };
        if !extra.is_empty() && !contextual.contains(extra) {
            if !contextual.is_empty() { contextual.push('\n'); }
            contextual.push_str(extra);
        }
    }
    let text = contextual.as_str();
    if tool_output_line_count(text) <= TOOL_OUTPUT_MAX_LINES && text.len() <= TOOL_OUTPUT_MAX_BYTES {
        return result;
    }
    let is_error = result.is_error;
    let mut media_parts = result.parts.into_iter().filter(|part| matches!(part, ProviderToolResultPart::Image { .. } | ProviderToolResultPart::Audio { .. })).collect::<Vec<_>>();
    let Some(state) = state else {
        let marker = "... 工具输出已截断；当前运行缺少状态，无法保存完整结果 ...";
        let preview = tool_output_preview(text, marker);
        media_parts.insert(0, ProviderToolResultPart::Text { text: preview.clone() });
        return ProviderToolResult { display_text: preview, parts: media_parts, is_error };
    };
    let marker = match store_full_tool_output(state, text) {
        Ok(path) => format!("... 工具输出已截断；完整内容保存于 {}。请使用搜索或按范围读取，禁止整文件读取 ...", terminal_path_for_user(&path)),
        Err(err) => format!("... 工具输出已截断；保存完整结果失败：{err} ..."),
    };
    let preview = tool_output_preview(text, &marker);
    media_parts.insert(0, ProviderToolResultPart::Text { text: preview.clone() });
    ProviderToolResult { display_text: preview, parts: media_parts, is_error }
}

#[cfg(test)]
mod tool_output_store_tests {
    use super::*;

    #[test]
    fn preview_should_keep_head_tail_and_utf8_boundaries() {
        let text = format!("开头\n{}\n结尾", "中".repeat(30_000));
        let preview = tool_output_preview(&text, "... truncated ...");
        assert!(preview.starts_with("开头"));
        assert!(preview.ends_with("结尾"));
        assert!(preview.len() <= TOOL_OUTPUT_MAX_BYTES);
    }

    #[test]
    fn preview_should_respect_line_limit() {
        let text = (0..3_000).map(|index| format!("line-{index}")).collect::<Vec<_>>().join("\n");
        let preview = tool_output_preview(&text, "... truncated ...");
        assert!(preview.lines().count() <= TOOL_OUTPUT_MAX_LINES);
        assert!(preview.contains("line-0"));
        assert!(preview.contains("line-2999"));
    }

    #[test]
    fn small_output_should_not_be_bounded() {
        let result = ProviderToolResult::text("small");
        assert_eq!(bound_provider_tool_result(None, result.clone()), result);
    }

    #[test]
    fn large_mcp_resource_should_be_bounded_and_keep_media() {
        let result = ProviderToolResult {
            display_text: "resource".to_string(),
            parts: vec![
                ProviderToolResultPart::Resource { mime: Some("text/plain".to_string()), uri: Some("mcp://large".to_string()), text: "x".repeat(TOOL_OUTPUT_MAX_BYTES + 1) },
                ProviderToolResultPart::Image { mime: "image/png".to_string(), data_base64: "abc".to_string() },
            ],
            is_error: false,
        };
        let bounded = bound_provider_tool_result(None, result);
        assert!(bounded.display_text.contains("工具输出已截断"));
        assert!(bounded.parts.iter().any(|part| matches!(part, ProviderToolResultPart::Image { .. })));
        assert!(!bounded.parts.iter().any(|part| matches!(part, ProviderToolResultPart::Resource { .. })));
    }

    #[test]
    fn full_output_should_be_written_to_managed_directory() {
        let root = std::env::temp_dir().join(format!("pai_tool_output_{}", chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()));
        let file = store_full_tool_output_at(&root, "complete output").expect("store output");
        assert_eq!(std::fs::read_to_string(&file).expect("read output"), "complete output");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn managed_directory_should_be_inside_llm_workspace() {
        let workspace = std::path::PathBuf::from("root").join("llm-workspace");
        assert_eq!(tool_output_directory_from_workspace(&workspace), workspace.join("tool-output"));
    }
}
