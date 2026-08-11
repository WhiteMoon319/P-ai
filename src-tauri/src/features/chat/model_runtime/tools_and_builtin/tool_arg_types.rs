#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct FetchToolArgs {
    pub(crate) url: String,
    #[serde(default)]
    pub(crate) max_length: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct BingSearchToolArgs {
    pub(crate) query: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemorySaveToolArgs {
    pub(crate) action: String,
    #[serde(default, rename = "sourceMemoryIds")]
    pub(crate) source_memory_ids: Vec<String>,
    pub(crate) memory: MemorySaveToolMemoryArgs,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MemorySaveToolMemoryArgs {
    #[serde(rename = "memoryType")]
    pub(crate) memory_type: String,
    pub(crate) judgment: String,
    #[serde(default)]
    pub(crate) reasoning: Option<String>,
    pub(crate) tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct RecallToolArgs {
    #[serde(default)]
    pub(crate) query: Option<String>,
    #[serde(default)]
    pub(crate) time: Option<String>,
    #[serde(default)]
    pub(crate) offset: Option<usize>,
    #[serde(default)]
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct TerminalExecToolArgs {
    #[serde(default)]
    pub(crate) action: Option<String>,
    #[serde(default)]
    pub(crate) command: Option<String>,
    #[serde(default)]
    pub(crate) timeout_ms: Option<u64>,
    #[serde(default)]
    pub(crate) commitment: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ConfigToolArgs {
    pub(crate) command: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReadMediaToolArgs {
    #[serde(alias = "absolute_path", alias = "absolutePath")]
    pub(crate) path: String,
    #[serde(default)]
    pub(crate) description: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct ContactSendFilesToolArgs {
    pub(crate) file_paths: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub(crate) struct GetSessionToolArgs {
    #[serde(default)]
    pub(crate) keyword: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct InformSessionToolArgs {
    pub(crate) session_id: String,
    pub(crate) content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DelegateToolArgs {
    pub(crate) department_id: String,
    #[serde(default)]
    pub(crate) target_agent_id: Option<String>,
    #[serde(default)]
    pub(crate) mode: Option<String>,
    #[serde(default)]
    pub(crate) why: Option<String>,
    #[serde(default)]
    pub(crate) goal: Option<String>,
    #[serde(default)]
    pub(crate) todo: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) background: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) question: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) focus: Option<String>,
}

pub(crate) fn delegate_arg_new_or_legacy(new_value: &Option<String>, legacy_value: &Option<String>) -> String {
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
pub(crate) enum DelegateMode {
    Background,
    Wait,
}

pub(crate) fn parse_delegate_mode(raw: Option<&str>) -> Result<DelegateMode, String> {
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

pub(crate) fn debug_text_snippet(text: &str, max_chars: usize) -> String {
    let compact = text.trim().replace('\r', "").replace('\n', "\\n");
    if compact.chars().count() <= max_chars {
        compact
    } else {
        let head = compact.chars().take(max_chars).collect::<String>();
        format!("{head}...")
    }
}

pub(crate) fn debug_exec_result_summary(value: &Value) -> String {
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
pub(crate) struct TaskToolArgsWire {
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) task_id: Option<String>,
    #[serde(default)]
    pub(crate) goal: Option<String>,
    #[serde(default)]
    pub(crate) todo: Option<String>,
    #[serde(default)]
    pub(crate) how: Option<String>,
    #[serde(default)]
    pub(crate) why: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) cause: Option<String>,
    #[serde(default)]
    pub(crate) flow: Option<String>,
    #[serde(default)]
    pub(crate) todos: Option<Vec<String>>,
    #[serde(default)]
    pub(crate) status_summary: Option<String>,
    #[serde(default)]
    pub(crate) stage_key: Option<String>,
    #[serde(default)]
    pub(crate) append_note: Option<String>,
    #[serde(default)]
    pub(crate) completion_state: Option<String>,
    #[serde(default)]
    pub(crate) completion_conclusion: Option<String>,
    #[serde(default)]
    pub(crate) trigger: Option<TaskTriggerInputLocal>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PlanToolArgs {
    pub(crate) action: String,
    pub(crate) path: String,
}
