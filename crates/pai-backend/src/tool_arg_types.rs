use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::task::TaskTriggerInputLocal;

/// 调试值片段（从 src-tauri core_provider_utils.rs 迁入）。
pub fn debug_value_snippet(value: &Value, max_chars: usize) -> String {
    let raw = serde_json::to_string(value).unwrap_or_else(|_| "<invalid json>".to_string());
    if raw.chars().count() <= max_chars {
        raw
    } else {
        let head = raw.chars().take(max_chars).collect::<String>();
        format!("{head}...")
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FetchToolArgs {
    pub url: String,
    #[serde(default)]
    pub max_length: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BingSearchToolArgs {
    pub query: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySaveToolArgs {
    pub action: String,
    #[serde(default, rename = "sourceMemoryIds")]
    pub source_memory_ids: Vec<String>,
    pub memory: MemorySaveToolMemoryArgs,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MemorySaveToolMemoryArgs {
    #[serde(rename = "memoryType")]
    pub memory_type: String,
    pub judgment: String,
    #[serde(default)]
    pub reasoning: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RecallToolArgs {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TerminalExecToolArgs {
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub commitment: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigToolArgs {
    pub command: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReadMediaToolArgs {
    #[serde(alias = "absolute_path", alias = "absolutePath")]
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContactSendFilesToolArgs {
    pub file_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct GetSessionToolArgs {
    #[serde(default)]
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InformSessionToolArgs {
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DelegateToolArgs {
    pub department_id: String,
    #[serde(default)]
    pub target_agent_id: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub todo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub focus: Option<String>,
}

pub fn delegate_arg_new_or_legacy(new_value: &Option<String>, legacy_value: &Option<String>) -> String {
    new_value
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| {
            legacy_value
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_default()
        .to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelegateMode {
    Background,
    Wait,
}

pub fn parse_delegate_mode(raw: Option<&str>) -> Result<DelegateMode, String> {
    match raw.map(str::trim).filter(|value| !value.is_empty()) {
        None => Ok(DelegateMode::Wait),
        Some("background") => Ok(DelegateMode::Background),
        Some("wait") => Ok(DelegateMode::Wait),
        Some(other) => Err(format!(
            "delegate.mode 必须是 `wait` 或 `background`，当前收到：{other}"
        )),
    }
}

#[cfg(test)]
mod tool_arg_types_tests {
    use super::*;

    #[test]
    fn parse_delegate_mode_should_default_to_wait() {
        assert_eq!(parse_delegate_mode(None).expect("default mode"), DelegateMode::Wait);
        assert_eq!(parse_delegate_mode(Some("")).expect("empty mode"), DelegateMode::Wait);
    }

    #[test]
    fn parse_delegate_mode_should_reject_legacy_values() {
        assert!(parse_delegate_mode(Some("sync")).is_err());
        assert!(parse_delegate_mode(Some("async")).is_err());
    }
}

pub fn debug_text_snippet(text: &str, max_chars: usize) -> String {
    let compact = text.trim().replace('\r', "").replace('\n', "\\n");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        let head = compact.chars().take(max_chars).collect::<String>();
        format!("{head}...")
    }
}

pub fn debug_exec_result_summary(value: &Value) -> String {
    let Some(obj) = value.as_object() else {
        return debug_value_snippet(value, 320);
    };
    let ok = obj.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let approved = obj.get("approved").and_then(Value::as_bool);
    let timed_out = obj.get("timedOut").and_then(Value::as_bool).unwrap_or(false);
    let exit_code = obj.get("exitCode").and_then(Value::as_i64).unwrap_or(-1);
    let duration_ms = obj.get("durationMs").and_then(Value::as_u64).unwrap_or(0);
    let blocked_reason = obj
        .get("blockedReason")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let command = obj.get("command").and_then(Value::as_str).unwrap_or_default();
    let stdout = obj.get("stdout").and_then(Value::as_str).unwrap_or_default();
    let stderr = obj.get("stderr").and_then(Value::as_str).unwrap_or_default();
    format!(
        "ok={}, approved={}, timedOut={}, exitCode={}, durationMs={}, blockedReason={}, command={}, stdout={}, stderr={}",
        ok,
        approved
            .map(|v| v.to_string())
            .unwrap_or_else(|| "n/a".to_string()),
        timed_out,
        exit_code,
        duration_ms,
        if blocked_reason.is_empty() { "none" } else { blocked_reason },
        debug_text_snippet(command, 160),
        debug_text_snippet(stdout, 220),
        debug_text_snippet(stderr, 220),
    )
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskToolArgsWire {
    pub action: String,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub goal: Option<String>,
    #[serde(default)]
    pub todo: Option<String>,
    #[serde(default)]
    pub how: Option<String>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub cause: Option<String>,
    #[serde(default)]
    pub flow: Option<String>,
    #[serde(default)]
    pub todos: Option<Vec<String>>,
    #[serde(default)]
    pub status_summary: Option<String>,
    #[serde(default)]
    pub stage_key: Option<String>,
    #[serde(default)]
    pub append_note: Option<String>,
    #[serde(default)]
    pub completion_state: Option<String>,
    #[serde(default)]
    pub completion_conclusion: Option<String>,
    #[serde(default)]
    pub trigger: Option<TaskTriggerInputLocal>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlanToolArgs {
    pub action: String,
    pub path: String,
}
