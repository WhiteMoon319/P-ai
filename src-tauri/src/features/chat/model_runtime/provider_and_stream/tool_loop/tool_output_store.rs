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

fn tool_output_preview_with_limits(
    text: &str,
    marker: &str,
    max_lines: usize,
    max_bytes: usize,
) -> String {
    let marker_bytes = marker.len();
    let text_budget = max_bytes.saturating_sub(marker_bytes.saturating_add(4));
    let head_bytes = text_budget.div_ceil(2);
    let tail_bytes = text_budget / 2;
    let lines = text.lines().collect::<Vec<_>>();
    let (head_source, tail_source) = if lines.len() > max_lines {
        let head_lines = max_lines.div_ceil(2).saturating_sub(2);
        let tail_lines = (max_lines / 2).saturating_sub(2);
        (
            lines[..head_lines].join("\n"),
            lines[lines.len().saturating_sub(tail_lines)..].join("\n"),
        )
    } else {
        (text.to_string(), text.to_string())
    };
    format!("{}\n\n{}\n\n{}", tool_output_take_prefix(&head_source, head_bytes), marker, tool_output_take_suffix(&tail_source, tail_bytes))
}

fn tool_output_preview(text: &str, marker: &str) -> String {
    tool_output_preview_with_limits(text, marker, TOOL_OUTPUT_MAX_LINES, TOOL_OUTPUT_MAX_BYTES)
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

#[derive(Debug, Clone)]
struct ProviderToolProjection {
    text: String,
    metadata: ProviderToolMetadata,
}

fn project_provider_tool_result(
    state: Option<&AppState>,
    tool_name: &str,
    result: &ProviderToolResult,
) -> ProviderToolProjection {
    if tool_name == "exec" {
        let exit_code = result
            .metadata
            .exit_code
            .and_then(|value| i32::try_from(value).ok())
            .unwrap_or(-1);
        let duration = std::time::Duration::from_millis(
            result.metadata.wall_time_ms.unwrap_or_default(),
        );
        let text = format_exec_output_for_model(
            exit_code,
            duration,
            result.metadata.timed_out,
            &result.output,
            default_exec_model_truncation_policy(),
        );
        return ProviderToolProjection {
            text,
            metadata: result.metadata.clone(),
        };
    }

    let text = result.output.as_str();
    let oversized = tool_output_line_count(text) > TOOL_OUTPUT_MAX_LINES
        || text.len() > TOOL_OUTPUT_MAX_BYTES;
    let mut metadata = result.metadata.clone();
    let projected_output = if oversized {
        let full_path = state.and_then(|state| store_full_tool_output(state, text).ok());
        metadata.truncated = true;
        metadata.total_output_lines = Some(tool_output_line_count(text));
        if let Some(path) = full_path.as_ref() {
            metadata.output_paths.push(terminal_path_for_user(path));
        }
        let marker = match (tool_name, full_path.as_ref()) {
            ("exec", _) => "... output truncated ...".to_string(),
            (_, Some(path)) => format!(
                "... output truncated ...\n\nFull output saved to: {}\nUse search or ranged reads; do not read the whole file.",
                terminal_path_for_user(path)
            ),
            (_, None) => "... output truncated; full output could not be saved ...".to_string(),
        };
        if tool_name == "exec" {
            tool_output_preview_with_limits(
                text,
                &marker,
                TOOL_OUTPUT_MAX_LINES.saturating_sub(10),
                TOOL_OUTPUT_MAX_BYTES.saturating_sub(4 * 1024),
            )
        } else {
            tool_output_preview(text, &marker)
        }
    } else {
        text.to_string()
    };
    ProviderToolProjection {
        text: projected_output,
        metadata,
    }
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
        assert_eq!(project_provider_tool_result(None, "read", &result).text, "small");
    }

    #[test]
    fn large_mcp_resource_should_be_bounded_and_keep_media() {
        let result = ProviderToolResult {
            output: "x".repeat(TOOL_OUTPUT_MAX_BYTES + 1),
            metadata: ProviderToolMetadata::default(),
            parts: vec![
                ProviderToolResultPart::Resource { mime: Some("text/plain".to_string()), uri: Some("mcp://large".to_string()), text: "x".repeat(TOOL_OUTPUT_MAX_BYTES + 1) },
                ProviderToolResultPart::Image { mime: "image/png".to_string(), data_base64: "abc".to_string(), width: 1, height: 1 },
            ],
            is_error: false,
        };
        let projected = project_provider_tool_result(None, "mcp", &result);
        assert!(projected.text.contains("output truncated"));
        assert!(result.parts.iter().any(|part| matches!(part, ProviderToolResultPart::Image { .. })));
    }

    #[test]
    fn exec_projection_should_use_codex_wrapper_and_middle_truncation() {
        let result = ProviderToolResult {
            output: (0..10_000)
                .map(|index| format!("line-{index}"))
                .collect::<Vec<_>>()
                .join("\n"),
            metadata: ProviderToolMetadata {
                exit_code: Some(0),
                wall_time_ms: Some(420),
                ..ProviderToolMetadata::default()
            },
            parts: Vec::new(),
            is_error: false,
        };
        let projection = project_provider_tool_result(None, "exec", &result);
        assert!(projection.text.starts_with("Exit code: 0\nWall time: 0.4 seconds"));
        assert!(projection.text.contains("Total output lines: 10000"));
        assert!(projection.text.contains("tokens truncated"));
        assert!(projection.text.contains("line-0"));
        assert!(projection.text.contains("line-9999"));
        assert!(!projection.text.contains("... output truncated ..."));
    }

    #[test]
    fn exec_projection_should_prefix_timeout_content() {
        let result = ProviderToolResult {
            output: "partial".to_string(),
            metadata: ProviderToolMetadata {
                exit_code: Some(-1),
                wall_time_ms: Some(1_500),
                timed_out: true,
                ..ProviderToolMetadata::default()
            },
            parts: Vec::new(),
            is_error: true,
        };
        let projection = project_provider_tool_result(None, "exec", &result);

        assert!(projection.text.starts_with("Exit code: -1\nWall time: 1.5 seconds\nOutput:\n"));
        assert!(projection.text.contains("command timed out after 1500 milliseconds\npartial"));
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
