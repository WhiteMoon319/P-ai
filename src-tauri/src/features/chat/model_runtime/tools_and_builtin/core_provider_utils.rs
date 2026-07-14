fn debug_value_snippet(value: &Value, max_chars: usize) -> String {
    let raw = serde_json::to_string(value).unwrap_or_else(|_| "<invalid json>".to_string());
    if raw.chars().count() <= max_chars {
        raw
    } else {
        let head = raw.chars().take(max_chars).collect::<String>();
        format!("{head}...")
    }
}

fn send_tool_status_event(
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
    tool_name: &str,
    tool_status: &str,
    tool_args: Option<&str>,
    tool_call_id: Option<&str>,
    message: &str,
) {
    let send_result = on_delta.send(AssistantDeltaEvent {
        delta: String::new(),
        kind: Some("tool_status".to_string()),
        request_id: None,
        activation_id: None,
        phase_id: None,
        reason: None,
        tool_name: Some(tool_name.to_string()),
        tool_call_id: tool_call_id.map(|value| value.to_string()),
        tool_status: Some(tool_status.to_string()),
        tool_args: tool_args.map(|v| v.to_string()),
        message: Some(message.to_string()),
        stream_cache: None,
    });
    let unified_status = match tool_status.trim().to_ascii_lowercase().as_str() {
        "start" | "running" | "begin" => "开始",
        "done" | "success" | "ok" | "completed" => "完成",
        "skip" | "skipped" => "跳过",
        "error" | "failed" | "fail" => "失败",
        _ => "未知",
    };
    let delivery_desc = if send_result.is_ok() {
        "投递成功"
    } else {
        "投递失败"
    };
    runtime_log_debug(format!(
        "[工具调用] 名称={} 状态={} 消息={} 事件投递结果={}",
        tool_name, unified_status, message, delivery_desc
    ));
}

fn send_stream_rebind_required_event(
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
    request_id: Option<&str>,
    reason: &str,
) {
    let _ = on_delta.send(AssistantDeltaEvent {
        delta: String::new(),
        kind: Some("stream_rebind_required".to_string()),
        request_id: request_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        activation_id: None,
        phase_id: None,
        reason: Some(reason.trim().to_string()),
        tool_name: None,
        tool_call_id: None,
        tool_status: None,
        tool_args: None,
        message: None,
        stream_cache: None,
    });
}

fn tool_failure_result_text(tool_name: &str, err_text: &str) -> String {
    let tool_name = tool_name.trim();
    let err_text = err_text.trim();
    format!("Tool failed: {tool_name}\nError: {err_text}")
}

fn tool_enabled(
    selected_api: &ApiConfig,
    _agent: &AgentProfile,
    current_department: Option<&DepartmentConfig>,
    id: &str,
) -> bool {
    if tool_restricted_by_department(current_department, id).is_some() {
        return false;
    }
    if builtin_tool_is_fixed_system(id) {
        return selected_api.enable_tools;
    }
    if builtin_tool_is_local_conversation_fixed(id) || builtin_tool_is_contact_only_hidden(id) {
        return false;
    }
    if id == "screenshot" && !selected_api.enable_image {
        return false;
    }
    if !selected_api.enable_tools {
        return false;
    }
    if tool_forced_by_department(current_department, id) {
        return true;
    }
    if !department_permission_allows_any_name(
        current_department,
        DepartmentPermissionCategory::BuiltinTool,
        &[id],
    ) {
        return false;
    }
    if let Some(tool) = default_agent_tools().iter().find(|tool| tool.id == id) {
        return tool.enabled;
    }
    if matches!(id, "write" | "delete" | "update" | "move") {
        return true;
    }
    true
}

#[derive(Debug)]
struct ToolInvokeError(String);

impl std::fmt::Display for ToolInvokeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ToolInvokeError {}

impl From<String> for ToolInvokeError {
    fn from(value: String) -> Self {
        Self(value)
    }
}

fn clean_text(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_image_unsupported_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("unknown variant `image_url`")
        || lower.contains("expected `text`")
        || lower.contains("does not support image")
        || lower.contains("image input")
}

fn truncate_by_chars(input: &str, max_chars: usize) -> String {
    if max_chars == 0 {
        return String::new();
    }
    if input.chars().count() <= max_chars {
        return input.to_string();
    }
    let mut out = String::new();
    for (idx, ch) in input.chars().enumerate() {
        if idx >= max_chars {
            break;
        }
        out.push(ch);
    }
    out.push_str("...");
    out
}

#[cfg(test)]
mod core_provider_utils_tests {
    use super::*;

    #[test]
    fn tool_failure_result_text_is_plain_and_explicit() {
        let raw = tool_failure_result_text("contact_send_files", "远程IM渠道未开启文件发送");
        assert_eq!(raw, "Tool failed: contact_send_files\nError: 远程IM渠道未开启文件发送");
        assert!(serde_json::from_str::<Value>(&raw).is_err());
    }

    #[test]
    fn tool_enabled_should_use_system_defaults_instead_of_agent_specific_tool_flags() {
        let mut api = AppConfig::default()
            .api_configs
            .into_iter()
            .next()
            .expect("default api config");
        api.enable_tools = true;
        api.enable_image = true;

        let mut agent = default_agent();
        if let Some(tool) = agent.tools.iter_mut().find(|tool| tool.id == "fetch") {
            tool.enabled = false;
        }
        if let Some(tool) = agent.tools.iter_mut().find(|tool| tool.id == "plan") {
            tool.enabled = true;
        }

        assert!(tool_enabled(&api, &agent, None, "fetch"));
        assert!(!tool_enabled(&api, &agent, None, "plan"));
        assert!(!tool_enabled(&api, &agent, None, "contact_reply"));
    }
}
