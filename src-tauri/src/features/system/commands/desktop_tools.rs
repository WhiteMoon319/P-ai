fn native_notification_text_excerpt(raw: &str, max_chars: usize) -> String {
    let normalized = raw
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    for (idx, ch) in trimmed.chars().enumerate() {
        if idx >= max_chars {
            out.push_str("...");
            break;
        }
        out.push(ch);
    }
    out
}


#[cfg(target_os = "android")]
fn send_native_notification(
    _app: &NativeAppHandle,
    _title: &str,
    _body: &str,
    _play_sound: bool,
) -> Result<(), String> {
    // Android 通知由 live_update.rs 的原生通道实现（tauri_plugin_notification Android 端）。
    // 桌面操作提醒/回合完成通知在 Android 端由前端轮询事件队列呈现，此处为空操作占位。
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct NativeNotificationDemoResult {
    permission_before: String,
    permission_after: String,
    title: String,
    body: String,
    sent_at: String,
}


#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DemoTestNotificationInput {
    kind: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct DemoTestNotificationResult {
    kind: String,
    sent_at: String,
    title: String,
    body: String,
}

// 通知设置页的“通知测试”：kind = normal 发普通通知；
// kind = live_update 在 Android 上模拟 live update 完整生命周期
// （ongoing 步骤 1/2 → 2s 后刷新步骤 2/2 → 2s 后转终态），
// 桌面端没有岛语义，降级为普通通知。
#[cfg(target_os = "android")]
const DEMO_LIVE_UPDATE_NOTIFICATION_ID: i32 = 0x50414903;



#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct XcapToolInput {
    method: String,
    #[serde(default)]
    args: Value,
}

fn command_exists_in_path(name: &str) -> bool {
    let raw = name.trim();
    if raw.is_empty() {
        return false;
    }
    let path_value = match std::env::var_os("PATH") {
        Some(value) => value,
        None => return false,
    };
    let name_path = Path::new(raw);
    let mut candidates = Vec::<String>::new();
    if name_path.extension().is_some() {
        candidates.push(raw.to_string());
    } else {
        candidates.push(raw.to_string());
        #[cfg(target_os = "windows")]
        {
            if let Some(pathext) = std::env::var_os("PATHEXT") {
                for ext in pathext.to_string_lossy().split(';') {
                    let trimmed = ext.trim();
                    if !trimmed.is_empty() {
                        candidates.push(format!("{raw}{trimmed}"));
                    }
                }
            } else {
                candidates.push(format!("{raw}.exe"));
            }
        }
    }

    for dir in std::env::split_paths(&path_value) {
        for candidate in &candidates {
            if dir.join(candidate).is_file() {
                return true;
            }
        }
    }
    false
}

fn host_runtime_prerequisite_installed(kind: &str) -> Result<bool, String> {
    match kind.trim().to_ascii_lowercase().as_str() {
        "git" => {
            if command_exists_in_path("git") {
                return Ok(true);
            }
            #[cfg(target_os = "windows")]
            {
                return Ok([
                    r"C:\Program Files\Git\cmd\git.exe",
                    r"C:\Program Files\Git\bin\git.exe",
                    r"C:\Program Files (x86)\Git\cmd\git.exe",
                    r"C:\Program Files (x86)\Git\bin\git.exe",
                ]
                .iter()
                .any(|path| Path::new(path).is_file()));
            }
            #[cfg(not(target_os = "windows"))]
            {
                Ok(false)
            }
        }
        "node" => {
            if command_exists_in_path("node") {
                return Ok(true);
            }
            #[cfg(target_os = "windows")]
            {
                return Ok([
                    r"C:\Program Files\nodejs\node.exe",
                    r"C:\Program Files (x86)\nodejs\node.exe",
                ]
                .iter()
                .any(|path| Path::new(path).is_file()));
            }
            #[cfg(not(target_os = "windows"))]
            {
                Ok(false)
            }
        }
        "rg" | "ripgrep" => Ok(command_exists_in_path("rg")),
        other => Err(format!("不支持的运行时依赖：{other}")),
    }
}


#[cfg(target_os = "android")]
fn resolve_chat_tool_session_id(
    state: &AppState,
    api_config_id: &str,
    agent_id: &str,
    conversation_id: Option<&str>,
) -> Result<String, String> {
    let api_id = api_config_id.trim();
    let agent = agent_id.trim();
    if let Some(conversation_id) = conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let (department_id, conversation_agent_id) = match conversation_service_v2()
            .get_conversation_meta(state, conversation_id)
        {
            Ok(meta) => (meta.department_id, meta.agent_id),
            Err(primary_error) => match delegate_runtime_thread_conversation_get_any(
                state,
                conversation_id,
            )? {
                Some(conversation) => (conversation.department_id, conversation.agent_id),
                None => return Err(primary_error),
            },
        };
        let department_id = department_id.trim().to_string();
        let session_scope = if department_id.is_empty() {
            conversation_agent_id.trim().to_string()
        } else {
            department_id
        };
        if session_scope.is_empty() {
            return Err(format!("指定会话缺少部门或 Agent 标识：{conversation_id}"));
        }
        return Ok(normalize_terminal_tool_session_id(&inflight_chat_key(
            &session_scope,
            Some(conversation_id),
        )));
    }
    if api_id.is_empty() {
        return Err("apiConfigId is required.".to_string());
    }
    if agent.is_empty() {
        return Err("agentId is required.".to_string());
    }

    let runtime_snapshot = load_runtime_organization_snapshot(state)?;
    let config = runtime_snapshot.config;
    let resolved_api_id = resolve_model_role_api_config_id(&config, api_id)
        .ok_or_else(|| format!("Model role '{api_id}' is not configured."))?;
    if !config.api_configs.iter().any(|v| v.id == resolved_api_id) {
        return Err(format!("Selected API config '{api_id}' not found."));
    }
    let agents = runtime_snapshot.agents;
    if !agents.iter().any(|v| v.id == agent && !v.is_built_in_user) {
        return Err(format!("Selected agent '{agent}' not found."));
    }

    let department_id = conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .and_then(|conversation_id| {
            conversation_service_v2()
                .get_conversation_meta(state, conversation_id)
                .ok()
        })
        .and_then(|conversation_meta| {
            let department_id = conversation_meta.department_id.trim();
            (!department_id.is_empty()).then(|| department_id.to_string())
        })
        .or_else(|| department_for_agent_id(&config, agent).map(|department| department.id.clone()))
        .unwrap_or_else(|| agent.to_string());
    let session_id = inflight_chat_key(&department_id, conversation_id);
    Ok(normalize_terminal_tool_session_id(&session_id))
}

fn resolve_chat_workspace_conversation_id(
    state: &AppState,
    agent_id: &str,
    conversation_id: Option<&str>,
) -> Result<String, String> {
    if let Some(conversation_id) = conversation_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return Ok(conversation_id.to_string());
    }
    conversation_service_v2()
        .resolve_latest_foreground_conversation_id(state, agent_id)
        .and_then(|value| {
            value.ok_or_else(|| "当前没有可用的活跃会话，需要提供 conversationId。".to_string())
        })
}

fn apply_conversation_chat_workspace_changes(
    state: &AppState,
    conversation_id: &str,
    shell_workspace_path: Option<Option<String>>,
    shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
    shell_autonomous_mode: Option<bool>,
    shell_work_mode: Option<String>,
) -> Result<Conversation, String> {
    if delegate_runtime_thread_conversation_get(state, conversation_id)?.is_some() {
        let next_path = shell_workspace_path.clone();
        let next_workspaces = shell_workspaces.clone();
        delegate_runtime_thread_modify(state, conversation_id, move |thread| {
            let original_path = thread.conversation.shell_workspace_path.clone();
            let original_workspaces = thread.conversation.shell_workspaces.clone();
            let original_autonomous_mode = thread.conversation.shell_autonomous_mode;
            let original_work_mode = thread.conversation.shell_work_mode.clone();
            if let Some(value) = next_path.clone() {
                thread.conversation.shell_workspace_path = value;
            }
            if let Some(value) = next_workspaces.clone() {
                thread.conversation.shell_workspaces = value;
            }
            if let Some(value) = shell_autonomous_mode {
                thread.conversation.shell_autonomous_mode = value;
            }
            if let Some(value) = shell_work_mode.clone() {
                thread.conversation.shell_work_mode = normalize_shell_work_mode_text(&value);
            }
            if thread.conversation.shell_workspace_path.as_deref().map(str::trim).filter(|value| !value.is_empty()).is_some()
                && terminal_workspace_path_from_conversation(state, &thread.conversation).is_none()
            {
                thread.conversation.shell_workspace_path = None;
            }
            if thread.conversation.shell_workspace_path == original_path
                && thread.conversation.shell_workspaces == original_workspaces
                && thread.conversation.shell_autonomous_mode == original_autonomous_mode
                && thread.conversation.shell_work_mode == original_work_mode
            {
                return Ok(());
            }
            mark_prompt_cache_rebuild_for_system_environment_by_conversation(
                state,
                conversation_id,
            );
            Ok(())
        })?;
        return delegate_runtime_thread_conversation_get_any(state, conversation_id)?
            .ok_or_else(|| format!("指定会话不存在：{conversation_id}"));
    }

    let updated = conversation_service_v2().update_shell_workspace(
        state,
        conversation_id,
        shell_workspace_path,
        shell_workspaces,
        shell_autonomous_mode,
        shell_work_mode,
    )?;
    mark_prompt_cache_rebuild_for_system_environment_by_conversation(state, conversation_id);
    Ok(updated)
}

fn workspace_name_from_path(path: &Path) -> String {
    path.file_name()
        .and_then(|v| v.to_str())
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn resolve_workspace_display_name_for_conversation(
    state: &AppState,
    conversation: Option<&Conversation>,
    root: &Path,
) -> String {
    let root_key = normalize_terminal_path_for_compare(root);
    if let Ok(workspaces) = terminal_allowed_workspaces_for_conversation_canonical(state, conversation) {
        for ws in workspaces {
            if normalize_terminal_path_for_compare(&ws.path) == root_key {
                return ws.name;
            }
        }
    }
    workspace_name_from_path(root)
}

fn build_chat_shell_workspace_list(
    state: &AppState,
    conversation: Option<&Conversation>,
) -> Vec<ShellWorkspaceConfig> {
    terminal_allowed_workspaces_for_conversation_canonical(state, conversation)
        .unwrap_or_default()
        .into_iter()
        .map(|workspace| ShellWorkspaceConfig {
            id: workspace.id,
            name: workspace.name,
            path: workspace.path.to_string_lossy().to_string(),
            level: workspace.level,
            access: workspace.access,
            built_in: workspace.built_in,
        })
        .collect()
}

fn build_chat_shell_workspace_output(
    state: &AppState,
    session_id: String,
    conversation: Option<&Conversation>,
    root: PathBuf,
) -> ChatShellWorkspaceOutput {
    ChatShellWorkspaceOutput {
        session_id,
        workspace_name: resolve_workspace_display_name_for_conversation(state, conversation, &root),
        root_path: root.to_string_lossy().to_string(),
        workspaces: build_chat_shell_workspace_list(state, conversation),
        autonomous_mode: conversation.map(|value| value.shell_autonomous_mode).unwrap_or(false),
        shell_work_mode: conversation
            .map(|value| normalize_shell_work_mode_text(&value.shell_work_mode))
            .unwrap_or_else(default_shell_work_mode),
    }
}

fn shell_workspace_display_path(path: &Path) -> String {
    #[cfg(target_os = "windows")]
    {
        let raw = path.to_string_lossy();
        if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
            return format!(r"\\{rest}");
        }
        if let Some(rest) = raw.strip_prefix(r"\\?\") {
            return rest.to_string();
        }
        raw.to_string()
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string_lossy().to_string()
    }
}






fn get_chat_shell_workspace_inner(
    input: ChatShellWorkspaceInput,
    state: &AppState,
) -> Result<ChatShellWorkspaceOutput, String> {
    let session_id =
        resolve_chat_tool_session_id(
            state,
            &input.api_config_id,
            &input.agent_id,
            input.conversation_id.as_deref(),
        )?;
    let conversation = terminal_session_conversation(state, &session_id)?;
    let root = terminal_session_root_canonical(state, &session_id)?;
    Ok(build_chat_shell_workspace_output(
        state,
        session_id,
        conversation.as_ref(),
        root,
    ))
}


fn update_chat_shell_workspace_layout_inner(
    input: SaveChatShellWorkspacesInput,
    state: &AppState,
) -> Result<ChatShellWorkspaceOutput, String> {
    let session_id =
        resolve_chat_tool_session_id(
            state,
            &input.api_config_id,
            &input.agent_id,
            input.conversation_id.as_deref(),
        )?;
    let conversation_id = resolve_chat_workspace_conversation_id(
        state,
        &input.agent_id,
        input.conversation_id.as_deref(),
    )?;
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &input.workspaces);
    let updated = apply_conversation_chat_workspace_changes(
        state,
        &conversation_id,
        Some(None),
        Some(normalized_workspaces),
        input.autonomous_mode,
        input.shell_work_mode,
    )?;
    {
        let mut roots = state
            .terminal_session_roots
            .lock()
            .map_err(|_| "Failed to lock terminal session roots".to_string())?;
        roots.remove(&session_id);
    }
    let root = terminal_session_root_canonical(state, &session_id)?;
    Ok(build_chat_shell_workspace_output(
        state,
        session_id,
        Some(&updated),
        root,
    ))
}





#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderFilePayload {
    path: String,
    name: String,
    extension: String,
    kind: String,
    content: String,
    force_plain: bool,
    virtualized: bool,
    total_lines: usize,
    block_line_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderFileBlockPayload {
    path: String,
    start_line: usize,
    end_line: usize,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderDirectoryEntry {
    path: String,
    name: String,
    is_directory: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderDirectoryPayload {
    path: String,
    name: String,
    entries: Vec<FileReaderDirectoryEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderWatchTargetInput {
    path: String,
    kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderWatchTargetsInput {
    session_id: String,
    targets: Vec<FileReaderWatchTargetInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReaderWatchEventPayload {
    session_id: String,
    path: String,
    kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FileReaderWatchFingerprint {
    exists: bool,
    is_dir: bool,
    len: u64,
    modified_ms: u128,
    directory_signature: String,
}

#[derive(Debug, Clone)]
struct FileReaderWatchTrackedTarget {
    path: String,
    kind: String,
    fingerprint: FileReaderWatchFingerprint,
}

static FILE_READER_WATCH_TARGETS: OnceLock<
    Mutex<std::collections::HashMap<String, Vec<FileReaderWatchTrackedTarget>>>,
> = OnceLock::new();
static FILE_READER_WATCH_THREAD_STARTED: OnceLock<()> = OnceLock::new();

const FILE_READER_PLAIN_TEXT_THRESHOLD: u64 = 2 * 1024 * 1024;
const FILE_READER_LINE_TRUNCATE_CHARS: usize = 10000;
const FILE_READER_BLOCK_LINE_COUNT: usize = 120;
const FILE_READER_READ_BURST_WINDOW_MS: u64 = 100;
const FILE_READER_WATCH_POLL_MS: u64 = 700;
const FILE_READER_WATCH_MAX_TARGETS: usize = 160;
const FILE_READER_WATCH_DIRECTORY_SIGNATURE_LIMIT: usize = 100;

#[derive(Debug, Clone)]
struct FileReaderReadTrace {
    at: std::time::Instant,
    window_label: String,
    path: String,
}

static FILE_READER_READ_TRACES: OnceLock<Mutex<Vec<FileReaderReadTrace>>> = OnceLock::new();

const NATIVE_NOTIFICATION_BODY_MAX_CHARS: usize = 180;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ResolveTerminalApprovalInput {
    request_id: String,
    approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ChatShellWorkspaceInput {
    #[serde(default)]
    api_config_id: String,
    #[serde(default)]
    agent_id: String,
    #[serde(default)]
    conversation_id: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SaveChatShellWorkspacesInput {
    #[serde(default)]
    api_config_id: String,
    #[serde(default)]
    agent_id: String,
    conversation_id: Option<String>,
    #[serde(default)]
    workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default)]
    autonomous_mode: Option<bool>,
    #[serde(default)]
    shell_work_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ChatShellWorkspaceOutput {
    session_id: String,
    workspace_name: String,
    root_path: String,
    workspaces: Vec<ShellWorkspaceConfig>,
    autonomous_mode: bool,
    shell_work_mode: String,
}
