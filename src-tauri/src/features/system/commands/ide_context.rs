#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextReferenceInput {
    id: String,
    file_path: String,
    #[serde(default)]
    start_line: Option<u32>,
    #[serde(default)]
    end_line: Option<u32>,
    #[serde(default)]
    content: String,
    #[serde(default)]
    language_id: Option<String>,
    source: String,
    captured_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpsertIdeContextSnapshotInput {
    client_id: String,
    #[serde(default)]
    auth_token: Option<String>,
    #[serde(default)]
    editor: String,
    #[serde(default)]
    workspace_roots: Vec<String>,
    #[serde(default)]
    references: Vec<IdeContextReferenceInput>,
    #[serde(default)]
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextWorkspaceQueryInput {
    #[serde(default)]
    workspaces: Vec<IdeContextWorkspaceInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextWorkspaceInput {
    path: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextReferenceItemOutput {
    id: String,
    workspace_path: String,
    workspace_name: String,
    file_path: String,
    file_name: String,
    relative_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    start_line: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    end_line: Option<u32>,
    display_label: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    language_id: Option<String>,
    source: String,
    captured_at: String,
    text_block: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextWorkspaceGroupOutput {
    workspace_path: String,
    workspace_name: String,
    references: Vec<IdeContextReferenceItemOutput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextQueryResultOutput {
    groups: Vec<IdeContextWorkspaceGroupOutput>,
    updated_at: String,
}

const IDE_CONTEXT_BRIDGE_HOST: &str = "127.0.0.1";
const IDE_CONTEXT_BRIDGE_BIND_HOST: &str = "0.0.0.0";
#[cfg(test)]
const IDE_CONTEXT_BRIDGE_BASE_PORT: u16 = 8429;
const IDE_CONTEXT_BRIDGE_PATH: &str = "/ide-context";
const IDE_CONTEXT_CHAT_BRIDGE_PATH: &str = "/chat";
const IDE_CONTEXT_BRIDGE_DISCOVERY_FILE: &str = "p-ai-ide-context-bridge.json";
const IDE_CONTEXT_SNAPSHOT_TTL_SECS: i64 = 30;
const IDE_CONTEXT_AUTH_TOKEN_TTL_SECS: i64 = 90 * 24 * 60 * 60;
const IDE_CONTEXT_MAX_AUTH_TOKENS: usize = 128;
const WEB_ACCESS_SERVICE_ID: &str = "web_access";
static IDE_CONTEXT_BRIDGE_STARTED: AtomicBool = AtomicBool::new(false);
static IDE_CONTEXT_BRIDGE_SHUTDOWN: OnceLock<
    Arc<Mutex<Option<tokio_util::sync::CancellationToken>>>,
> = OnceLock::new();
static IDE_CONTEXT_BRIDGE_SERVER_TASK: OnceLock<
    Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>>,
> = OnceLock::new();
static IDE_CONTEXT_PORT_SERVICE_CORE: OnceLock<Arc<LocalPortServiceCore>> = OnceLock::new();
static IDE_CONTEXT_CHAT_CLIENTS: OnceLock<
    Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>>,
> = OnceLock::new();
static IDE_CONTEXT_CHAT_CLIENT_CONVERSATIONS: OnceLock<
    Arc<Mutex<std::collections::HashMap<String, String>>>,
> = OnceLock::new();
static WEB_ACCESS_CONNECTIONS: OnceLock<
    Arc<Mutex<std::collections::HashMap<String, WebAccessConnectionEntry>>>,
> = OnceLock::new();

#[derive(Debug, Clone)]
struct IdeContextRuntime {
    snapshots: Arc<Mutex<std::collections::HashMap<String, IdeContextSnapshot>>>,
    bridge_auth: Arc<Mutex<IdeContextBridgeAuthRuntime>>,
    current_port: Arc<Mutex<Option<u16>>>,
    web_access_cache: Arc<Mutex<IdeContextWebAccessCache>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebAccessConnectionSummary {
    id: String,
    path: String,
    peer_addr: String,
    local: bool,
    authenticated: bool,
    connected_at: String,
    client_id: String,
}

#[derive(Debug, Clone)]
struct WebAccessConnectionEntry {
    id: String,
    path: String,
    peer_addr: String,
    local: bool,
    authenticated: bool,
    connected_at: String,
    client_id: String,
}

#[derive(Debug, Default)]
struct IdeContextBridgeAuthRuntime {
    valid_tokens: std::collections::HashMap<String, OffsetDateTime>,
    remote_password: String,
}

#[derive(Debug, Default)]
struct IdeContextWebAccessCache {
    lan_hosts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GetWebAccessInfoInput {
    #[serde(default)]
    force_refresh: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct IdeContextPersistedBridgeToken {
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    token: String,
    #[serde(default)]
    #[serde(skip_serializing_if = "String::is_empty")]
    expires_at: String,
    #[serde(default)]
    tokens: Vec<IdeContextPersistedBridgeTokenEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextPersistedBridgeTokenEntry {
    token: String,
    expires_at: String,
}

impl IdeContextPersistedBridgeToken {
    fn normalized_entries(self) -> Vec<IdeContextPersistedBridgeTokenEntry> {
        if !self.tokens.is_empty() {
            return self.tokens;
        }
        if self.token.trim().is_empty() || self.expires_at.trim().is_empty() {
            return Vec::new();
        }
        vec![IdeContextPersistedBridgeTokenEntry {
            token: self.token,
            expires_at: self.expires_at,
        }]
    }
}

impl IdeContextRuntime {
    fn new() -> Self {
        let mut bridge_auth = IdeContextBridgeAuthRuntime::default();
        bridge_auth.remote_password = ide_context_generate_remote_password();
        Self {
            snapshots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            bridge_auth: Arc::new(Mutex::new(bridge_auth)),
            current_port: Arc::new(Mutex::new(None)),
            web_access_cache: Arc::new(Mutex::new(IdeContextWebAccessCache::default())),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextUpdatedEvent {
    client_id: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeContextBridgeDiscovery {
    url: String,
    bridge_url: String,
    chat_url: String,
    host: String,
    bind_host: String,
    port: u16,
    path: String,
    chat_path: String,
    pid: u32,
    updated_at: String,
    #[serde(default)]
    token: String,
    remote_password: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct WebAccessInfoOutput {
    running: bool,
    enabled: bool,
    configured_port: u16,
    port: u16,
    #[serde(default)]
    listen_addr: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    status_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    last_error: Option<String>,
    local_url: String,
    remote_urls: Vec<String>,
    remote_password: String,
    active_connections: Vec<WebAccessConnectionSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatJsonRpcRequest {
    #[serde(default)]
    jsonrpc: String,
    #[serde(default)]
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatJsonRpcError {
    code: i32,
    message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatAuthLoginInput {
    #[serde(default)]
    password: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatConversationInput {
    conversation_id: String,
    workspace_path: Option<String>,
    workspace_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatConversationBlockPageInput {
    conversation_id: String,
    #[serde(default)]
    block_id: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatCreateConversationInput {
    #[serde(default)]
    department_id: Option<String>,
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    shell_workspaces: Option<Vec<ShellWorkspaceConfig>>,
    #[serde(default)]
    shell_autonomous_mode: Option<bool>,
    #[serde(default)]
    workspace_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatSendInput {
    conversation_id: String,
    text: String,
    #[serde(default)]
    extra_text_blocks: Vec<String>,
    #[serde(default)]
    images: Vec<IdeChatImageInput>,
    #[serde(default)]
    attachments: Vec<AttachmentMetaInput>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatImageInput {
    mime: String,
    bytes_base64: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatQueueAttachmentInput {
    file_name: String,
    #[serde(default)]
    mime: String,
    bytes_base64: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatStopInput {
    conversation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatSelectModelInput {
    conversation_id: String,
    #[serde(default)]
    api_config_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatResolveTerminalApprovalInput {
    request_id: String,
    approved: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatTerminalApprovalRequestIdInput {
    request_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspacePermissionInput {
    conversation_id: String,
    access: String,
    #[serde(default)]
    workspace_path: Option<String>,
    #[serde(default)]
    workspace_name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatRewindInput {
    conversation_id: String,
    message_id: String,
    #[serde(default, rename = "agentId")]
    _agent_id: Option<String>,
    #[serde(default)]
    undo_apply_patch: bool,
}

fn ide_chat_avatar_data_url(state: &AppState, path: Option<&str>) -> String {
    let Some(path) = path.map(str::trim).filter(|value| !value.is_empty()) else {
        return String::new();
    };
    let Ok(avatars_dir) = avatar_storage_dir(state) else {
        return String::new();
    };
    let Ok(root) = fs::canonicalize(&avatars_dir) else {
        return String::new();
    };
    let Ok(target) = fs::canonicalize(path) else {
        return String::new();
    };
    if !target.starts_with(&root) {
        return String::new();
    }
    let Ok(metadata) = fs::metadata(&target) else {
        return String::new();
    };
    if !metadata.is_file() {
        return String::new();
    }
    let ext = target
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .unwrap_or_default();
    let mime = match ext.as_str() {
        "webp" => "image/webp",
        "png" => "image/png",
        _ => return String::new(),
    };
    let Ok(bytes) = fs::read(&target) else {
        return String::new();
    };
    format!("data:{mime};base64,{}", B64.encode(bytes))
}

fn ide_chat_persona_payload(state: &AppState, active_agent_id: Option<&str>) -> Result<Value, String> {
    let runtime = state_read_runtime_state_cached(state)?;
    let runtime_org = load_runtime_organization_snapshot(state)?;
    let agents = runtime_org.agents;
    let user_alias = runtime.user_alias.trim();
    let active_agent_id = active_agent_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| runtime.assistant_department_agent_id.trim());
    let mut persona_name_map = serde_json::Map::new();
    let mut persona_avatar_url_map = serde_json::Map::new();
    let mut assistant_name = String::new();
    let mut assistant_avatar_url = String::new();
    let mut user_avatar_url = String::new();
    for agent in &agents {
        let id = agent.id.trim();
        if id.is_empty() {
            continue;
        }
        let name = agent.name.trim();
        persona_name_map.insert(
            id.to_string(),
            serde_json::json!(if name.is_empty() { id } else { name }),
        );
        let avatar_url = ide_chat_avatar_data_url(state, agent.avatar_path.as_deref());
        if !avatar_url.is_empty() {
            persona_avatar_url_map.insert(id.to_string(), serde_json::json!(avatar_url.clone()));
        }
        if id == USER_PERSONA_ID || agent.is_built_in_user {
            if !avatar_url.is_empty() {
                user_avatar_url = avatar_url.clone();
            }
        }
        if id == active_agent_id {
            assistant_name = if name.is_empty() { id.to_string() } else { name.to_string() };
            assistant_avatar_url = avatar_url;
        }
    }
    if assistant_name.is_empty() {
        assistant_name = active_agent_id.to_string();
    }
    Ok(serde_json::json!({
        "userAlias": if user_alias.is_empty() { default_user_alias() } else { user_alias.to_string() },
        "userAvatarUrl": user_avatar_url,
        "assistantName": assistant_name,
        "assistantAvatarUrl": assistant_avatar_url,
        "personaNameMap": persona_name_map,
        "personaAvatarUrlMap": persona_avatar_url_map,
    }))
}

fn ide_chat_model_payload_for_conversation(state: &AppState, conversation: &Conversation) -> Result<Value, String> {
    let config = state_read_config_cached(state)?;
    let department_primary_id = config
        .departments
        .iter()
        .find(|department| department.id.trim() == conversation.department_id.trim())
        .map(department_primary_api_config_id)
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| config.assistant_department_api_config_id.trim().to_string());
    let resolved_department_primary_id = resolve_model_role_api_config_id(&config, &department_primary_id)
        .unwrap_or_else(|| department_primary_id.clone());
    let preferred_id = repair_conversation_preferred_model_for_snapshot(state, conversation)?;
    let conversation_call_primary_id = preferred_id
        .as_deref()
        .unwrap_or(resolved_department_primary_id.as_str())
        .to_string();
    let options = config
        .api_configs
        .iter()
        .filter(|api| is_text_chat_api(api))
        .map(|api| {
            serde_json::json!({
                "id": api.id,
                "name": api.name,
                "requestFormat": api.request_format,
                "model": api.model,
                "enableText": api.enable_text,
                "enableImage": api.enable_image,
            })
        })
        .collect::<Vec<_>>();
    Ok(serde_json::json!({
        "conversationCallPrimaryApiConfigId": conversation_call_primary_id,
        "preferredChatModelId": preferred_id,
        "toolReviewApiConfigId": config.tool_review_api_config_id,
        "chatModelOptions": options,
    }))
}

fn ide_chat_workspace_permission_payload(
    state: &AppState,
    conversation: &Conversation,
) -> Result<Value, String> {
    let workspaces = terminal_allowed_workspaces_for_conversation_canonical(state, Some(conversation))?;
    let main = workspaces
        .iter()
        .find(|workspace| workspace.level == SHELL_WORKSPACE_LEVEL_MAIN)
        .or_else(|| workspaces.iter().find(|workspace| workspace.level == SHELL_WORKSPACE_LEVEL_SYSTEM));
    let access = main
        .map(|workspace| workspace.access.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| SHELL_WORKSPACE_ACCESS_APPROVAL.to_string());
    Ok(serde_json::json!({
        "access": access,
        "workspaceName": main.map(|workspace| workspace.name.clone()).unwrap_or_default(),
        "rootPath": main.map(|workspace| workspace.path.to_string_lossy().to_string()).unwrap_or_default(),
    }))
}

fn ide_chat_conversation_from_meta_view(conversation_meta: &ConversationMetaView) -> Conversation {
    Conversation {
        id: conversation_meta.id.clone(),
        title: conversation_meta.title.clone(),
        agent_id: conversation_meta.agent_id.clone(),
        department_id: conversation_meta.department_id.clone(),
        bound_conversation_id: None,
        parent_conversation_id: None,
        child_conversation_ids: Vec::new(),
        fork_message_cursor: None,
        unread_count: conversation_meta.unread_count,
        conversation_kind: conversation_meta.conversation_kind.clone(),
        root_conversation_id: conversation_meta.root_conversation_id.clone(),
        delegate_id: conversation_meta.delegate_id.clone(),
        created_at: conversation_meta.created_at.clone(),
        updated_at: conversation_meta.updated_at.clone(),
        last_user_at: None,
        last_assistant_at: None,
        status: conversation_meta.status.clone(),
        summary: conversation_meta.summary.clone(),
        user_profile_snapshot: String::new(),
        shell_workspace_path: conversation_meta.shell_workspace_path.clone(),
        shell_workspaces: conversation_meta.shell_workspaces.clone(),
        shell_autonomous_mode: conversation_meta.shell_autonomous_mode,
        archived_at: conversation_meta.archived_at.clone(),
        messages: Vec::new(),
        fast_request_turns: conversation_meta.fast_request_turns.clone(),
        current_todos: conversation_meta.current_todos.clone(),
        memory_recall_table: Vec::new(),
        plan_mode_enabled: false,
        preferred_api_config_id: conversation_meta.preferred_api_config_id.clone(),
        auto_push_remote_contact_id: conversation_meta.auto_push_remote_contact_id.clone(),
        cumulative_usage: conversation_meta.cumulative_usage.clone(),
        active_goal: conversation_meta.active_goal.clone(),
    }
}

fn ide_chat_workspace_permission(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let conversation_meta =
        conversation_service_v2().get_conversation_meta(state, input.conversation_id.trim())?;
    let conversation = ide_chat_conversation_from_meta_view(&conversation_meta);
    ide_chat_workspace_permission_payload(state, &conversation)
}

fn ide_chat_select_workspace_permission(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspacePermissionInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let access = match input.access.trim() {
        SHELL_WORKSPACE_ACCESS_READ_ONLY => SHELL_WORKSPACE_ACCESS_READ_ONLY.to_string(),
        SHELL_WORKSPACE_ACCESS_APPROVAL => SHELL_WORKSPACE_ACCESS_APPROVAL.to_string(),
        SHELL_WORKSPACE_ACCESS_FULL_ACCESS => SHELL_WORKSPACE_ACCESS_FULL_ACCESS.to_string(),
        _ => return Err("Unsupported workspace access".to_string()),
    };
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let mut workspaces = conversation_meta.shell_workspaces.clone();
    let mut changed = false;
    for workspace in workspaces.iter_mut() {
        if normalize_shell_workspace_level_text(&workspace.level) == SHELL_WORKSPACE_LEVEL_MAIN {
            workspace.access = access.clone();
            changed = true;
        }
    }
    if !changed {
        let workspace_path = input.workspace_path.as_deref().map(str::trim).unwrap_or_default();
        if workspace_path.is_empty() {
            return Err("当前会话没有主工作目录，无法设置权限。".to_string());
        }
        let fallback_name = workspace_path
            .replace('\\', "/")
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("VS Code")
            .to_string();
        let name = input
            .workspace_name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(fallback_name.as_str())
            .to_string();
        workspaces.push(ShellWorkspaceConfig {
            id: "vscode-sidebar-main-workspace".to_string(),
            name,
            path: workspace_path.to_string(),
            level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(),
            access: access.clone(),
            built_in: false,
        });
    }
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &workspaces);
    let updated = apply_conversation_chat_workspace_changes(
        state,
        conversation_id,
        Some(None),
        Some(normalized_workspaces),
        None,
    )?;
    ide_chat_workspace_permission_payload(state, &updated)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceListInput {
    conversation_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceDirectoryListInput {
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatFileReaderReadInput {
    path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatFileReaderReadBlockInput {
    path: String,
    start_line: usize,
    line_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatReadPlanFileInput {
    conversation_id: String,
    path: String,
}

fn ide_chat_workspace_list(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceListInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let conversation_meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let conversation = ide_chat_conversation_from_meta_view(&conversation_meta);
    let workspaces = terminal_allowed_workspaces_for_conversation_canonical(state, Some(&conversation))?;
    let main = workspaces
        .iter()
        .find(|ws| ws.level == SHELL_WORKSPACE_LEVEL_MAIN)
        .or_else(|| workspaces.iter().find(|ws| ws.level == SHELL_WORKSPACE_LEVEL_SYSTEM));
    let root_path = main
        .map(|ws| terminal_path_for_user(&ws.path))
        .unwrap_or_default();
    let workspace_name = main
        .map(|ws| ws.name.clone())
        .unwrap_or_default();
    let autonomous_mode = conversation_meta.shell_autonomous_mode;
    let workspace_values: Vec<Value> = workspaces
        .iter()
        .map(|ws| {
            serde_json::json!({
                "id": ws.id,
                "name": ws.name,
                "level": ws.level,
                "access": ws.access,
                "builtIn": ws.built_in,
                "path": terminal_path_for_user(&ws.path),
            })
        })
        .collect();
    Ok(serde_json::json!({
        "workspaces": workspace_values,
        "rootPath": root_path,
        "workspaceName": workspace_name,
        "autonomousMode": autonomous_mode,
    }))
}

fn ide_chat_workspace_directory_list(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceDirectoryListInput>(params)?;
    let payload = list_file_reader_directory(input.path)?;
    let directories: Vec<Value> = payload
        .entries
        .into_iter()
        .filter(|entry| entry.is_directory)
        .map(|entry| {
            serde_json::json!({
                "path": entry.path,
                "name": entry.name,
            })
        })
        .collect();
    Ok(serde_json::json!({
        "path": payload.path,
        "name": payload.name,
        "directories": directories,
    }))
}

fn ide_chat_file_reader_directory_list(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceDirectoryListInput>(params)?;
    serde_json::to_value(list_file_reader_directory(input.path)?)
        .map_err(|err| format!("serialize file reader directory failed: {err}"))
}

fn ide_chat_file_reader_read(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatFileReaderReadInput>(params)?;
    serde_json::to_value(read_file_reader_file_inner(input.path, None)?)
        .map_err(|err| format!("serialize file reader payload failed: {err}"))
}

fn ide_chat_file_reader_read_block(params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatFileReaderReadBlockInput>(params)?;
    serde_json::to_value(read_file_reader_file_block(
        input.path,
        input.start_line,
        input.line_count,
    )?)
    .map_err(|err| format!("serialize file reader block failed: {err}"))
}

fn ide_chat_delegate_statuses(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<ListConversationDelegateStatusesInput>(params)?;
    serde_json::to_value(list_conversation_delegate_statuses_inner(input, state)?)
        .map_err(|err| format!("Serialize delegate statuses failed: {err}"))
}

fn ide_chat_delegate_abort(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<AbortDelegateConversationInput>(params)?;
    serde_json::to_value(abort_delegate_conversation_inner(input, state)?)
        .map_err(|err| format!("Serialize delegate abort result failed: {err}"))
}

fn ide_chat_delegate_block_page(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<GetConversationBlockPageInput>(params)?;
    serde_json::to_value(get_delegate_conversation_block_page_inner(input, state)?)
        .map_err(|err| format!("Serialize delegate block page failed: {err}"))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceLayoutSaveInput {
    conversation_id: String,
    #[serde(default)]
    workspaces: Vec<ShellWorkspaceConfig>,
    #[serde(default)]
    autonomous_mode: Option<bool>,
}

fn ide_chat_workspace_layout_save(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspaceLayoutSaveInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() {
        return Err("conversationId is required".to_string());
    }
    let normalized_workspaces = normalize_conversation_shell_workspaces(state, &input.workspaces);
    let updated = apply_conversation_chat_workspace_changes(
        state,
        conversation_id,
        Some(None),
        Some(normalized_workspaces),
        input.autonomous_mode,
    )?;
    ide_chat_workspace_permission_payload(state, &updated)
}

fn ide_chat_create_conversation_options(state: &AppState) -> Result<Value, String> {
    let runtime_org = load_runtime_organization_snapshot(state)?;
    let config = runtime_org.config;
    let agents = runtime_org.agents;
    let options = config
        .departments
        .iter()
        .flat_map(|department| {
            let department_id = department.id.trim();
            if department_id.is_empty() {
                return Vec::new();
            }
            let Some(api_config_id) = department_primary_chat_api_config_id(&config, department) else {
                return Vec::new();
            };
            let Some(api_config) = config
                .api_configs
                .iter()
                .find(|api| api.id.trim() == api_config_id && is_text_chat_api(api)) else {
                    return Vec::new();
                };
            let department_name = if department.name.trim().is_empty() {
                department_id
            } else {
                department.name.trim()
            };
            department
                .agent_ids
                .iter()
                .map(|value| value.trim())
                .filter(|agent_id| !agent_id.is_empty())
                .filter_map(|agent_id| {
                    let agent = agents
                        .iter()
                        .find(|agent| agent.id.trim() == agent_id && !agent.is_built_in_user)?;
                    let agent_name = if agent.name.trim().is_empty() {
                        agent_id
                    } else {
                        agent.name.trim()
                    };
                    Some(serde_json::json!({
                        "id": format!("{department_id}::{agent_id}"),
                        "departmentId": department_id,
                        "agentId": agent_id,
                        "departmentName": department_name,
                        "agentName": agent_name,
                        "label": format!("{department_name} / {agent_name}"),
                        "name": department_name,
                        "ownerAgentId": agent_id,
                        "ownerName": agent_name,
                        "providerName": if api_config.name.trim().is_empty() { api_config.id.trim() } else { api_config.name.trim() },
                        "modelName": api_config.model.trim(),
                        "apiConfigId": api_config_id,
                        "childDepartmentIds": &department.child_department_ids,
                    }))
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let default_agent_id = assistant_department_agent_id(&config).unwrap_or_else(default_assistant_department_agent_id);
    Ok(serde_json::json!({
        "departments": options,
        "defaultDepartmentId": ASSISTANT_DEPARTMENT_ID,
        "defaultAgentId": default_agent_id,
    }))
}

include!("ide_context/bridge_access.rs");

fn ide_context_chat_clients() -> Arc<Mutex<std::collections::HashMap<String, tokio::sync::mpsc::UnboundedSender<serde_json::Value>>>> {
    IDE_CONTEXT_CHAT_CLIENTS
        .get_or_init(|| Arc::new(Mutex::new(std::collections::HashMap::new())))
        .clone()
}

fn ide_context_chat_client_conversations() -> Arc<Mutex<std::collections::HashMap<String, String>>> {
    IDE_CONTEXT_CHAT_CLIENT_CONVERSATIONS
        .get_or_init(|| Arc::new(Mutex::new(std::collections::HashMap::new())))
        .clone()
}

fn web_access_connections() -> Arc<Mutex<std::collections::HashMap<String, WebAccessConnectionEntry>>> {
    WEB_ACCESS_CONNECTIONS
        .get_or_init(|| Arc::new(Mutex::new(std::collections::HashMap::new())))
        .clone()
}

fn ide_context_web_access_enabled(state: &AppState) -> bool {
    match state_read_config_cached(state) {
        Ok(config) => config.web_access_enabled,
        Err(err) => {
            runtime_log_error(format!("[网络访问] 读取配置失败，按关闭处理: {}", err));
            false
        }
    }
}

fn ide_context_bridge_shutdown_notification(reason: &str) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "bridge.shutdown",
        "params": {
            "reason": reason,
        },
    })
}

fn ide_context_notify_chat_clients_shutdown(reason: &str) {
    let notification = ide_context_bridge_shutdown_notification(reason);
    if let Ok(clients) = ide_context_chat_clients().lock() {
        for sender in clients.values() {
            let _ = sender.send(notification.clone());
        }
    }
}

fn web_access_register_connection(
    path: &str,
    peer_addr: &std::net::SocketAddr,
    local: bool,
    authenticated: bool,
    client_id: &str,
) -> String {
    let id = Uuid::new_v4().to_string();
    let entry = WebAccessConnectionEntry {
        id: id.clone(),
        path: path.trim().to_string(),
        peer_addr: peer_addr.to_string(),
        local,
        authenticated,
        connected_at: now_iso(),
        client_id: client_id.trim().to_string(),
    };
    if let Ok(mut connections) = web_access_connections().lock() {
        connections.insert(id.clone(), entry);
    }
    id
}

fn web_access_update_connection_auth(connection_id: &str, authenticated: bool, client_id: Option<&str>) {
    if let Ok(mut connections) = web_access_connections().lock() {
        if let Some(entry) = connections.get_mut(connection_id) {
            entry.authenticated = authenticated;
            if let Some(client_id) = client_id.map(str::trim).filter(|value| !value.is_empty()) {
                entry.client_id = client_id.to_string();
            }
        }
    }
}

fn web_access_remove_connection(connection_id: &str) {
    if let Ok(mut connections) = web_access_connections().lock() {
        connections.remove(connection_id);
    }
}

fn web_access_connection_summaries() -> Vec<WebAccessConnectionSummary> {
    let mut items = if let Ok(connections) = web_access_connections().lock() {
        connections
            .values()
            .cloned()
            .map(|entry| WebAccessConnectionSummary {
                id: entry.id,
                path: entry.path,
                peer_addr: entry.peer_addr,
                local: entry.local,
                authenticated: entry.authenticated,
                connected_at: entry.connected_at,
                client_id: entry.client_id,
            })
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    items.sort_by(|left, right| right.connected_at.cmp(&left.connected_at));
    items
}

fn ide_chat_broadcast_notification(method: &str, params: serde_json::Value) {
    let clients = ide_context_chat_clients();
    let message = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    let mut stale_ids = Vec::<String>::new();
    if let Ok(clients_guard) = clients.lock() {
        for (client_id, sender) in clients_guard.iter() {
            if sender.send(message.clone()).is_err() {
                stale_ids.push(client_id.clone());
            }
        }
    }
    if !stale_ids.is_empty() {
        if let Ok(mut clients_guard) = clients.lock() {
            for client_id in stale_ids {
                clients_guard.remove(&client_id);
            }
        }
    }
}

fn ide_chat_sidebar_client_id_from_label(label: &str) -> Option<String> {
    let label = label.trim();
    if let Some(value) = label.strip_prefix("vscode-sidebar:") {
        let client_id = value.trim();
        if !client_id.is_empty() {
            return Some(client_id.to_string());
        }
    }
    if let Some(value) = label.strip_prefix("ide-chat-sidebar-") {
        let client_id = value.trim();
        if !client_id.is_empty() {
            return Some(client_id.to_string());
        }
    }
    None
}

fn ide_chat_emit_notification_to_sidebar_conversation(
    conversation_id: &str,
    method: &str,
    params: serde_json::Value,
) -> usize {
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return 0;
    }
    let client_ids = ide_context_chat_client_conversations()
        .lock()
        .ok()
        .map(|conversations| {
            conversations
                .iter()
                .filter_map(|(client_id, mapped_conversation_id)| {
                    if mapped_conversation_id.trim() == cid {
                        Some(client_id.clone())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if client_ids.is_empty() {
        return 0;
    }
    let clients = ide_context_chat_clients();
    let message = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
    });
    let mut delivered = 0usize;
    let mut stale_ids = Vec::<String>::new();
    if let Ok(clients_guard) = clients.lock() {
        for client_id in &client_ids {
            match clients_guard.get(client_id) {
                Some(sender) if sender.send(message.clone()).is_ok() => {
                    delivered += 1;
                }
                Some(_) => stale_ids.push(client_id.clone()),
                None => stale_ids.push(client_id.clone()),
            }
        }
    }
    if !stale_ids.is_empty() {
        if let Ok(mut clients_guard) = clients.lock() {
            for client_id in &stale_ids {
                clients_guard.remove(client_id);
            }
        }
        if let Ok(mut conversations) = ide_context_chat_client_conversations().lock() {
            for client_id in stale_ids {
                conversations.remove(&client_id);
            }
        }
    }
    delivered
}

fn ide_context_prune_expired_bridge_tokens(auth: &mut IdeContextBridgeAuthRuntime, now: OffsetDateTime) {
    auth.valid_tokens.retain(|_, expires_at| *expires_at > now);
    if auth.valid_tokens.len() <= IDE_CONTEXT_MAX_AUTH_TOKENS {
        return;
    }
    let mut tokens = auth
        .valid_tokens
        .iter()
        .map(|(token, expires_at)| (token.clone(), *expires_at))
        .collect::<Vec<_>>();
    tokens.sort_by(|left, right| right.1.cmp(&left.1));
    auth.valid_tokens = tokens
        .into_iter()
        .take(IDE_CONTEXT_MAX_AUTH_TOKENS)
        .collect();
}

fn ide_context_bridge_token_store_path(state: &AppState) -> PathBuf {
    app_root_from_data_path(&state.data_path)
        .join("web-access")
        .join("bridge-auth-token.json")
}

fn ide_context_clear_persisted_bridge_token(state: &AppState) -> Result<(), String> {
    let path = ide_context_bridge_token_store_path(state);
    if !path.exists() {
        return Ok(());
    }
    fs::remove_file(&path)
        .map_err(|err| format!("删除 Web 访问令牌失败，path={}，error={err}", path.display()))
}

fn ide_context_persist_bridge_tokens(
    state: &AppState,
    tokens: &std::collections::HashMap<String, OffsetDateTime>,
) -> Result<(), String> {
    let path = ide_context_bridge_token_store_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Web 访问令牌目录失败，path={}，error={err}", parent.display()))?;
    }
    let mut entries = tokens
        .iter()
        .filter_map(|(token, expires_at)| {
            let normalized_token = token.trim();
            if normalized_token.is_empty() {
                return None;
            }
            Some(IdeContextPersistedBridgeTokenEntry {
                token: normalized_token.to_string(),
                expires_at: expires_at.format(&Rfc3339).ok()?,
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| right.expires_at.cmp(&left.expires_at));
    let payload = IdeContextPersistedBridgeToken {
        tokens: entries,
        ..Default::default()
    };
    let text = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("序列化 Web 访问令牌失败: {err}"))?;
    fs::write(&path, text)
        .map_err(|err| format!("写入 Web 访问令牌失败，path={}，error={err}", path.display()))
}

fn ide_context_try_restore_persisted_bridge_token(
    state: &AppState,
    runtime: &IdeContextRuntime,
) -> Result<(), String> {
    let path = ide_context_bridge_token_store_path(state);
    if !path.exists() {
        return Ok(());
    }
    let text = fs::read_to_string(&path)
        .map_err(|err| format!("读取 Web 访问令牌失败，path={}，error={err}", path.display()))?;
    let payload: IdeContextPersistedBridgeToken = serde_json::from_str(&text)
        .map_err(|err| format!("解析 Web 访问令牌失败，path={}，error={err}", path.display()))?;
    let now = now_utc();
    let mut restored = std::collections::HashMap::new();
    for entry in payload.normalized_entries() {
        let token = entry.token.trim().to_string();
        if token.is_empty() {
            continue;
        }
        let Some(expires_at) = parse_iso(&entry.expires_at) else {
            continue;
        };
        if expires_at > now {
            restored.insert(token, expires_at);
        }
    }
    if restored.is_empty() {
        let _ = ide_context_clear_persisted_bridge_token(state);
        return Ok(());
    }
    let mut auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
    auth.valid_tokens.clear();
    auth.valid_tokens.extend(restored);
    ide_context_prune_expired_bridge_tokens(&mut auth, now);
    Ok(())
}

fn ide_context_store_bridge_token(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
    token: &str,
    expires_at: OffsetDateTime,
) -> Result<(), String> {
    let normalized_token = token.trim().to_string();
    if normalized_token.is_empty() {
        return Err("Web 访问令牌为空，无法保存".to_string());
    }
    {
        let mut auth = runtime
            .bridge_auth
            .lock()
            .map_err(|_| "Failed to lock ide context bridge auth".to_string())?;
        auth.valid_tokens.insert(normalized_token.clone(), expires_at);
        ide_context_prune_expired_bridge_tokens(&mut auth, now_utc());
        if let Some(state) = state {
            ide_context_persist_bridge_tokens(state, &auth.valid_tokens)?;
        }
    }
    Ok(())
}

fn ide_context_issue_bridge_token_with_state(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
) -> Result<String, String> {
    let token = ide_context_generate_bridge_token();
    let now = now_utc();
    let expires_at = now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS);
    ide_context_store_bridge_token(runtime, state, &token, expires_at)?;
    Ok(token)
}

fn ide_context_consume_bridge_token_with_state(
    runtime: &IdeContextRuntime,
    state: Option<&AppState>,
    provided: &str,
) -> Result<String, (String, Option<String>)> {
    let provided = provided.trim();
    if provided.is_empty() {
        return Err(("authToken is required".to_string(), None));
    }
    if let Some(state) = state {
        let should_restore = runtime
            .bridge_auth
            .lock()
            .map(|auth| auth.valid_tokens.is_empty())
            .unwrap_or(false);
        if should_restore {
            if let Err(err) = ide_context_try_restore_persisted_bridge_token(state, runtime) {
                runtime_log_error(format!("[IDE 上下文桥] 恢复持久化 Web 访问令牌失败: {}", err));
            }
        }
    }
    let mut auth = runtime
        .bridge_auth
        .lock()
        .map_err(|_| ("Failed to lock ide context bridge auth".to_string(), None))?;
    let now = now_utc();
    ide_context_prune_expired_bridge_tokens(&mut auth, now);
    if auth.valid_tokens.is_empty() {
        drop(auth);
        if let Some(state) = state {
            let _ = ide_context_clear_persisted_bridge_token(state);
        }
        let refreshed_token = ide_context_issue_bridge_token_with_state(runtime, state)
            .map_err(|err| (err, None))?;
        return Err((
            "IDE context bridge token expired, discovery refreshed".to_string(),
            Some(refreshed_token),
        ));
    }
    if !auth.valid_tokens.contains_key(provided) {
        return Err(("invalid authToken".to_string(), None));
    }
    let expires_at = now + time::Duration::seconds(IDE_CONTEXT_AUTH_TOKEN_TTL_SECS);
    drop(auth);
    ide_context_store_bridge_token(runtime, state, provided, expires_at)
        .map_err(|err| (err, None))?;
    Ok(provided.to_string())
}

fn ide_context_normalize_time_or_now(field_name: &str, raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return now_iso();
    }
    match normalize_rfc3339_to_utc_storage(field_name, trimmed) {
        Ok(value) => value,
        Err(err) => {
            runtime_log_info(format!(
                "[IDE 上下文桥] 时间字段非法，回退当前时间: field={}, value={}, error={}",
                field_name, trimmed, err
            ));
            now_iso()
        }
    }
}

fn ide_context_timestamp_compare_desc(left: &str, right: &str) -> std::cmp::Ordering {
    match (parse_iso(left), parse_iso(right)) {
        (Some(left_time), Some(right_time)) => right_time.cmp(&left_time),
        (Some(_), None) => std::cmp::Ordering::Less,
        (None, Some(_)) => std::cmp::Ordering::Greater,
        (None, None) => right.cmp(left),
    }
}

fn ide_context_timestamp_is_newer(candidate: &str, current: &str) -> bool {
    if current.trim().is_empty() {
        return !candidate.trim().is_empty();
    }
    ide_context_timestamp_compare_desc(candidate, current) == std::cmp::Ordering::Less
}

fn ide_context_reference_dedup_key(item: &IdeContextReferenceItemOutput) -> String {
    let file_key = ide_context_compare_key(&item.file_path);
    let source_key = item.source.trim();
    if file_key.is_empty() && source_key.is_empty() {
        item.id.clone()
    } else if file_key.is_empty() {
        format!("{}|{}", item.id, source_key)
    } else if source_key.is_empty() {
        file_key
    } else {
        format!("{}|{}", file_key, source_key)
    }
}

fn ide_context_reference_source_priority(source: &str) -> u8 {
    match source.trim() {
        "selection" => 3,
        "visible_range" => 2,
        "active_file" => 1,
        _ => 0,
    }
}

fn ide_context_should_replace_reference(
    candidate: &IdeContextReferenceItemOutput,
    existing: &IdeContextReferenceItemOutput,
) -> bool {
    if ide_context_timestamp_is_newer(&candidate.captured_at, &existing.captured_at) {
        return true;
    }
    if ide_context_timestamp_is_newer(&existing.captured_at, &candidate.captured_at) {
        return false;
    }

    let candidate_priority = ide_context_reference_source_priority(&candidate.source);
    let existing_priority = ide_context_reference_source_priority(&existing.source);
    if candidate_priority != existing_priority {
        return candidate_priority > existing_priority;
    }

    let candidate_content_len = candidate.content.trim().chars().count();
    let existing_content_len = existing.content.trim().chars().count();
    if candidate_content_len != existing_content_len {
        return candidate_content_len > existing_content_len;
    }

    candidate.display_label < existing.display_label
}

fn ide_context_snapshot_is_expired(snapshot: &IdeContextSnapshot, now: &OffsetDateTime) -> bool {
    match parse_iso(&snapshot.updated_at) {
        Some(updated_at) => updated_at < (*now - time::Duration::seconds(IDE_CONTEXT_SNAPSHOT_TTL_SECS)),
        None => true,
    }
}

fn ide_context_prune_expired_snapshots(
    snapshots: &mut std::collections::HashMap<String, IdeContextSnapshot>,
) {
    let now = now_utc();
    snapshots.retain(|client_id, snapshot| {
        if ide_context_snapshot_is_expired(snapshot, &now) {
            runtime_log_debug(format!(
                "[IDE 上下文桥] 快照过期已清理: client_id={}, updated_at={}",
                client_id, snapshot.updated_at
            ));
            false
        } else {
            true
        }
    });
}

fn emit_ide_context_updated(state: &AppState, client_id: &str, updated_at: &str) {
    let app_handle = match state.app_handle.lock() {
        Ok(slot) => slot.clone(),
        Err(_) => None,
    };
    if let Some(app_handle) = app_handle {
        let _ = app_handle.emit(
            "ide-context-updated",
            IdeContextUpdatedEvent {
                client_id: client_id.to_string(),
                updated_at: updated_at.to_string(),
            },
        );
    }
    ide_chat_broadcast_notification(
        "ideContext.updated",
        serde_json::json!({
            "clientId": client_id,
            "updatedAt": updated_at,
        }),
    );
}

fn ide_context_compare_key(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_terminal_path_input_for_current_platform(trimmed);
    let path = std::path::PathBuf::from(if normalized.is_empty() { trimmed } else { &normalized });
    shell_workspace_display_path(&path)
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_ascii_lowercase()
}

fn ide_context_display_path(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let normalized = normalize_terminal_path_input_for_current_platform(trimmed);
    let path = std::path::PathBuf::from(if normalized.is_empty() { trimmed } else { &normalized });
    let resolved = path.canonicalize().unwrap_or(path);
    shell_workspace_display_path(&resolved).replace('\\', "/")
}

fn ide_context_workspace_name(input: &IdeContextWorkspaceInput) -> String {
    let explicit = input.name.as_deref().map(str::trim).unwrap_or("");
    if !explicit.is_empty() {
        return explicit.to_string();
    }
    let display_path = ide_context_display_path(&input.path);
    std::path::Path::new(&display_path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or(display_path)
}

fn ide_context_path_is_within_workspace(file_path: &str, workspace_path: &str) -> bool {
    let file_key = ide_context_compare_key(file_path);
    let workspace_key = ide_context_compare_key(workspace_path);
    if file_key.is_empty() || workspace_key.is_empty() {
        return false;
    }
    file_key == workspace_key || file_key.starts_with(&(workspace_key + "/"))
}

fn ide_context_relative_display_path(file_path: &str, workspace_path: &str) -> String {
    let file_display = ide_context_display_path(file_path);
    let workspace_display = ide_context_display_path(workspace_path);
    let file_key = ide_context_compare_key(&file_display);
    let workspace_key = ide_context_compare_key(&workspace_display);
    if file_key == workspace_key {
        return std::path::Path::new(&file_display)
            .file_name()
            .and_then(|value| value.to_str())
            .map(ToOwned::to_owned)
            .unwrap_or(file_display);
    }
    let prefix = format!("{}/", workspace_key);
    if let Some(relative_key) = file_key.strip_prefix(&prefix) {
        let relative = relative_key.replace('/', std::path::MAIN_SEPARATOR_STR);
        return relative.replace('\\', "/");
    }
    file_display
}

fn ide_context_line_suffix(start_line: Option<u32>, end_line: Option<u32>) -> String {
    match (start_line, end_line) {
        (Some(start), Some(end)) if end > start => format!(":{start}-{end}"),
        (Some(start), _) => format!(":{start}"),
        _ => String::new(),
    }
}

fn ide_context_text_block(file_path: &str, reference: &IdeContextReference) -> String {
    if reference.source.trim() == "active_file" {
        return ["[IDE 上下文引用]".to_string(), format!("文件: {file_path}")].join("\n");
    }
    let mut lines = vec!["[IDE 上下文引用]".to_string(), format!("文件: {file_path}")];
    if reference.start_line.is_some() || reference.end_line.is_some() {
        let line_text = match (reference.start_line, reference.end_line) {
            (Some(start), Some(end)) if end > start => format!("{start}-{end}"),
            (Some(start), _) => start.to_string(),
            (_, Some(end)) => end.to_string(),
            _ => String::new(),
        };
        if !line_text.is_empty() {
            lines.push(format!("行号: {line_text}"));
        }
    }
    if let Some(language_id) = reference
        .language_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        lines.push(format!("语言: {language_id}"));
    }
    let source = reference.source.trim();
    if !source.is_empty() {
        lines.push(format!("来源: {source}"));
    }
    let captured_at = reference.captured_at.trim();
    if !captured_at.is_empty() {
        lines.push(format!("采集时间: {captured_at}"));
    }
    lines.push("内容:".to_string());
    lines.push(reference.content.clone());
    lines.join("\n")
}

fn ide_context_bridge_url_for_host(host: &str, port: u16) -> String {
    format!("ws://{}:{}{}", host, port, IDE_CONTEXT_BRIDGE_PATH)
}

fn ide_context_chat_bridge_url_for_host(host: &str, port: u16) -> String {
    format!("ws://{}:{}{}", host, port, IDE_CONTEXT_CHAT_BRIDGE_PATH)
}

fn ide_context_bridge_url(port: u16) -> String {
    ide_context_bridge_url_for_host(IDE_CONTEXT_BRIDGE_HOST, port)
}

fn ide_context_chat_bridge_url(port: u16) -> String {
    ide_context_chat_bridge_url_for_host(IDE_CONTEXT_BRIDGE_HOST, port)
}

fn ide_context_sidebar_url_for_host(host: &str, port: u16) -> String {
    format!("http://{}:{}/sidebar", host, port)
}

fn ide_context_bridge_discovery_path() -> std::path::PathBuf {
    std::env::temp_dir().join(IDE_CONTEXT_BRIDGE_DISCOVERY_FILE)
}

fn ide_context_bridge_shutdown_slot() -> Arc<Mutex<Option<tokio_util::sync::CancellationToken>>> {
    IDE_CONTEXT_BRIDGE_SHUTDOWN
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

fn ide_context_bridge_create_shutdown_token() -> tokio_util::sync::CancellationToken {
    let token = tokio_util::sync::CancellationToken::new();
    if let Ok(mut slot) = ide_context_bridge_shutdown_slot().lock() {
        *slot = Some(token.clone());
    }
    token
}

fn ide_context_bridge_server_task_slot() -> Arc<Mutex<Option<tauri::async_runtime::JoinHandle<()>>>> {
    IDE_CONTEXT_BRIDGE_SERVER_TASK
        .get_or_init(|| Arc::new(Mutex::new(None)))
        .clone()
}

fn ide_context_bridge_set_server_task(handle: tauri::async_runtime::JoinHandle<()>) {
    if let Ok(mut slot) = ide_context_bridge_server_task_slot().lock() {
        *slot = Some(handle);
    }
}

fn ide_context_port_service_core() -> Arc<LocalPortServiceCore> {
    IDE_CONTEXT_PORT_SERVICE_CORE
        .get_or_init(|| Arc::new(LocalPortServiceCore::new()))
        .clone()
}

fn ide_context_bridge_server_task_is_running() -> bool {
    IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst)
}

fn ide_context_bridge_take_server_task() -> Option<tauri::async_runtime::JoinHandle<()>> {
    ide_context_bridge_server_task_slot()
        .lock()
        .ok()
        .and_then(|mut slot| slot.take())
}

fn publish_ide_context_bridge_discovery(port: u16, remote_password: &str) -> Result<(), String> {
    let url = ide_context_bridge_url(port);
    let chat_url = ide_context_chat_bridge_url(port);
    let payload = IdeContextBridgeDiscovery {
        url: url.clone(),
        bridge_url: url,
        chat_url,
        host: IDE_CONTEXT_BRIDGE_HOST.to_string(),
        bind_host: IDE_CONTEXT_BRIDGE_BIND_HOST.to_string(),
        port,
        path: IDE_CONTEXT_BRIDGE_PATH.to_string(),
        chat_path: IDE_CONTEXT_CHAT_BRIDGE_PATH.to_string(),
        pid: std::process::id(),
        updated_at: now_iso(),
        token: String::new(),
        remote_password: remote_password.to_string(),
    };
    let path = ide_context_bridge_discovery_path();
    let text = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Serialize IDE context bridge discovery failed: {err}"))?;
    fs::write(&path, text).map_err(|err| {
        format!(
            "Write IDE context bridge discovery failed ({}): {err}",
            path.display()
        )
    })?;
    Ok(())
}

fn clear_ide_context_bridge_discovery() {
    let path = ide_context_bridge_discovery_path();
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

async fn prepare_ide_context_bridge_server_start(
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
    port_service: &Arc<LocalPortServiceCore>,
) -> Option<(tokio::net::TcpListener, u16, String)> {
    let config = match state_read_config_cached(state) {
        Ok(config) => config,
        Err(err) => {
            runtime_log_error(format!(
                "[网络访问] 读取配置失败，使用默认端口: {}",
                err
            ));
            AppConfig::default()
        }
    };
    if !config.web_access_enabled {
        IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
        ide_context_set_current_port(ide_context_runtime, None);
        clear_ide_context_bridge_discovery();
        port_service
            .set_status_text(WEB_ACCESS_SERVICE_ID, Some("disabled".to_string()))
            .await;
        port_service
            .set_listen_addr(WEB_ACCESS_SERVICE_ID, None)
            .await;
        runtime_log_warn(format!("[网络访问] 跳过启动：网络访问已关闭"));
        return None;
    }
    port_service
        .set_status_text(WEB_ACCESS_SERVICE_ID, Some("binding".to_string()))
        .await;
    port_service.set_last_error(WEB_ACCESS_SERVICE_ID, None).await;
    let preferred_port = normalize_web_access_port(config.web_access_port);
    let (listener, port) = match bind_ide_context_bridge_listener(preferred_port).await {
        Ok(result) => result,
        Err(err) => {
            IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
            ide_context_set_current_port(ide_context_runtime, None);
            clear_ide_context_bridge_discovery();
            port_service
                .set_status_text(WEB_ACCESS_SERVICE_ID, Some("bind_failed".to_string()))
                .await;
            port_service
                .set_last_error(WEB_ACCESS_SERVICE_ID, Some(err.clone()))
                .await;
            runtime_log_error(format!("[网络访问] 监听失败: {}", err));
            return None;
        }
    };
    ide_context_set_current_port(ide_context_runtime, Some(port));
    let bridge_url = ide_context_bridge_url(port);
    port_service
        .set_listen_addr(WEB_ACCESS_SERVICE_ID, Some(format!("{}:{}", IDE_CONTEXT_BRIDGE_BIND_HOST, port)))
        .await;
    let remote_password = match ide_context_effective_remote_password(state, ide_context_runtime) {
        Ok(password) => password,
        Err(err) => {
            IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
            ide_context_set_current_port(ide_context_runtime, None);
            clear_ide_context_bridge_discovery();
            port_service
                .set_listen_addr(WEB_ACCESS_SERVICE_ID, None)
                .await;
            port_service
                .set_status_text(WEB_ACCESS_SERVICE_ID, Some("error".to_string()))
                .await;
            port_service
                .set_last_error(WEB_ACCESS_SERVICE_ID, Some(err.clone()))
                .await;
            runtime_log_error(format!("[网络访问] 初始化远程访问密码失败，error={}", err));
            return None;
        }
    };
    if let Err(err) = publish_ide_context_bridge_discovery(port, &remote_password) {
        IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
        ide_context_set_current_port(ide_context_runtime, None);
        clear_ide_context_bridge_discovery();
        port_service
            .set_listen_addr(WEB_ACCESS_SERVICE_ID, None)
            .await;
        port_service
            .set_status_text(WEB_ACCESS_SERVICE_ID, Some("error".to_string()))
            .await;
        port_service
            .set_last_error(WEB_ACCESS_SERVICE_ID, Some(err.clone()))
            .await;
        runtime_log_error(format!("[网络访问] 写入发现文件失败，error={}", err));
        return None;
    }
    port_service
        .set_status_text(WEB_ACCESS_SERVICE_ID, Some("listening".to_string()))
        .await;
    port_service.set_last_error(WEB_ACCESS_SERVICE_ID, None).await;
    port_service
        .add_log(
            WEB_ACCESS_SERVICE_ID,
            "info",
            &format!("服务启动，监听 {}", bridge_url),
        )
        .await;
    runtime_log_info(format!("[网络访问] 已监听 {}", bridge_url));
    Some((listener, port, bridge_url))
}

fn spawn_ide_context_bridge_server_task(
    app: AppHandle,
    state: AppState,
    ide_context_runtime: IdeContextRuntime,
    port_service: Arc<LocalPortServiceCore>,
    shutdown_token: tokio_util::sync::CancellationToken,
    listener: tokio::net::TcpListener,
    port: u16,
    bridge_url: String,
) {
    let server_task = tauri::async_runtime::spawn(async move {
        loop {
            let (stream, peer_addr) = tokio::select! {
                _ = shutdown_token.cancelled() => {
                    clear_ide_context_bridge_discovery();
                    IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
                    ide_context_set_current_port(&ide_context_runtime, None);
                    port_service
                        .set_status_text(WEB_ACCESS_SERVICE_ID, Some("stopped".to_string()))
                        .await;
                    port_service
                        .set_listen_addr(WEB_ACCESS_SERVICE_ID, None)
                        .await;
                    if let Ok(mut slot) = ide_context_bridge_shutdown_slot().lock() {
                        slot.take();
                    }
                    runtime_log_info(format!("[网络访问] 收到停机信号，停止监听 {}", bridge_url));
                    break;
                }
                result = listener.accept() => match result {
                    Ok(result) => result,
                    Err(err) => {
                        runtime_log_error(format!("[网络访问] 接收连接失败: {}", err));
                        continue;
                    }
                },
            };
            let state_clone = state.clone();
            let app_clone = app.clone();
            let ide_context_runtime_clone = ide_context_runtime.clone();
            tauri::async_runtime::spawn(async move {
                ide_context_ws_handle_connection(
                    stream,
                    peer_addr,
                    port,
                    app_clone,
                    state_clone,
                    ide_context_runtime_clone,
                )
                .await;
            });
        }
    });
    ide_context_bridge_set_server_task(server_task);
}

async fn bind_ide_context_bridge_listener(
    preferred_port: u16,
) -> Result<(tokio::net::TcpListener, u16), String> {
    let port = normalize_web_access_port(preferred_port);
    let addr = format!("{}:{}", IDE_CONTEXT_BRIDGE_BIND_HOST, port);
    match tokio::net::TcpListener::bind(&addr).await {
        Ok(listener) => Ok((listener, port)),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::AddrInUse {
                runtime_log_info(format!("[网络访问] 固定端口已占用，无法启动: {}", addr));
                Err(format!("固定端口 {} 已被占用，请释放后重试", port))
            } else {
                runtime_log_error(format!("[网络访问] 固定端口监听失败 {}: {}", addr, err));
                Err(format!("固定端口 {} 监听失败: {}", port, err))
            }
        }
    }
}

async fn ide_context_stream_is_websocket(stream: &tokio::net::TcpStream) -> bool {
    let mut buffer = [0_u8; 1024];
    match tokio::time::timeout(std::time::Duration::from_millis(500), stream.peek(&mut buffer)).await
    {
        Ok(Ok(count)) if count > 0 => {
            String::from_utf8_lossy(&buffer[..count])
                .to_ascii_lowercase()
                .contains("upgrade: websocket")
        }
        _ => false,
    }
}

fn ide_context_http_status_text(status: u16) -> &'static str {
    match status {
        200 => "OK",
        404 => "Not Found",
        405 => "Method Not Allowed",
        426 => "Upgrade Required",
        _ => "Internal Server Error",
    }
}

fn ide_context_http_header_value<'a>(headers: &'a str, name: &str) -> Option<&'a str> {
    let prefix = format!("{}:", name.to_ascii_lowercase());
    headers.lines().skip(1).find_map(|line| {
        let trimmed = line.trim();
        if trimmed.to_ascii_lowercase().starts_with(&prefix) {
            trimmed.split_once(':').map(|(_, value)| value.trim())
        } else {
            None
        }
    })
}

fn ide_context_http_path_from_request(headers: &str) -> (&str, &str) {
    let first_line = headers.lines().next().unwrap_or_default();
    let mut parts = first_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let uri = parts.next().unwrap_or("/");
    let path = uri.split('?').next().unwrap_or("/");
    (method, path)
}

fn ide_context_web_asset_path(path: &str) -> Option<String> {
    let path = path.trim();
    match path {
        "/" | "/sidebar" | "/sidebar.html" => Some("sidebar.html".to_string()),
        "/settings" | "/settings.html" => Some("settings.html".to_string()),
        _ if path.starts_with("/assets/") && !path.contains("..") => {
            Some(path.trim_start_matches('/').to_string())
        }
        _ => None,
    }
}

fn ide_context_web_icon_bytes(path: &str) -> Option<&'static [u8]> {
    match path.trim() {
        "/favicon.ico" | "/favicon.png" => {
            Some(include_bytes!("../../../../icons/32x32.png").as_slice())
        }
        _ => None,
    }
}

fn ide_context_web_html_with_bridge(asset_bytes: &[u8], host: &str) -> Vec<u8> {
    let chat_url = format!("ws://{}{}", host, IDE_CONTEXT_CHAT_BRIDGE_PATH);
    let injected = serde_json::json!({
        "chatUrl": chat_url,
        "workspaceRoots": [],
    });
    let script = format!(
        "<script>window.__PAI_SIDEBAR_BRIDGE__ = {}; window.__PAI_SETTINGS_BRIDGE__ = window.__PAI_SIDEBAR_BRIDGE__;</script>",
        injected
    );
    let icon_links = r#"<link rel="icon" type="image/png" href="/favicon.png">
  <link rel="shortcut icon" type="image/png" href="/favicon.png">"#;
    let injection = format!("{}\n  {}", icon_links, script);
    let html = String::from_utf8_lossy(asset_bytes);
    if html.contains("</head>") {
        html.replacen("</head>", &format!("  {}\n  </head>", injection), 1)
            .into_bytes()
    } else {
        format!("{}\n{}", injection, html).into_bytes()
    }
}

async fn ide_context_http_write_response(
    stream: &mut tokio::net::TcpStream,
    status: u16,
    content_type: &str,
    body: Vec<u8>,
) {
    use tokio::io::AsyncWriteExt;

    let headers = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
        status,
        ide_context_http_status_text(status),
        content_type,
        body.len()
    );
    let _ = stream.write_all(headers.as_bytes()).await;
    let _ = stream.write_all(&body).await;
    let _ = stream.shutdown().await;
}

async fn ide_context_http_handle_connection(
    mut stream: tokio::net::TcpStream,
    app: AppHandle,
) {
    use tokio::io::AsyncReadExt;

    let mut buffer = vec![0_u8; 8192];
    let count = match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        stream.read(&mut buffer),
    )
    .await
    {
        Ok(Ok(count)) => count,
        _ => 0,
    };
    let headers = String::from_utf8_lossy(&buffer[..count]);
    let (method, path) = ide_context_http_path_from_request(&headers);
    if method != "GET" {
        ide_context_http_write_response(
            &mut stream,
            405,
            "text/plain; charset=utf-8",
            b"Method Not Allowed".to_vec(),
        )
        .await;
        return;
    }
    if path == IDE_CONTEXT_BRIDGE_PATH || path == IDE_CONTEXT_CHAT_BRIDGE_PATH {
        ide_context_http_write_response(
            &mut stream,
            426,
            "text/plain; charset=utf-8",
            b"WebSocket upgrade required".to_vec(),
        )
        .await;
        return;
    }
    if let Some(icon) = ide_context_web_icon_bytes(path) {
        ide_context_http_write_response(
            &mut stream,
            200,
            "image/png",
            icon.to_vec(),
        )
        .await;
        return;
    }
    let Some(asset_path) = ide_context_web_asset_path(path) else {
        ide_context_http_write_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"Not Found".to_vec(),
        )
        .await;
        return;
    };
    let Some(asset) = app.asset_resolver().get(asset_path.clone()) else {
        ide_context_http_write_response(
            &mut stream,
            404,
            "text/plain; charset=utf-8",
            b"Asset Not Found".to_vec(),
        )
        .await;
        return;
    };
    let host = ide_context_http_header_value(&headers, "host")
        .filter(|value| !value.is_empty())
        .unwrap_or("127.0.0.1");
    let body = if asset_path == "sidebar.html" || asset_path == "settings.html" {
        ide_context_web_html_with_bridge(asset.bytes(), host)
    } else {
        asset.bytes().to_vec()
    };
    ide_context_http_write_response(
        &mut stream,
        200,
        asset.mime_type(),
        body,
    )
    .await;
}

fn upsert_ide_context_snapshot_internal(
    input: UpsertIdeContextSnapshotInput,
    runtime: &IdeContextRuntime,
) -> Result<(String, String), String> {
    let client_id = input.client_id.trim().to_string();
    if client_id.is_empty() {
        return Err("clientId is required".to_string());
    }
    let updated_at = input
        .updated_at
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| ide_context_normalize_time_or_now("updatedAt", value))
        .unwrap_or_else(now_iso);
    let snapshot = IdeContextSnapshot {
        client_id: client_id.clone(),
        editor: {
            let editor = input.editor.trim();
            if editor.is_empty() {
                "vscode".to_string()
            } else {
                editor.to_string()
            }
        },
        workspace_roots: input
            .workspace_roots
            .into_iter()
            .map(|path| ide_context_display_path(&path))
            .filter(|path| !path.trim().is_empty())
            .collect(),
        references: input
            .references
            .into_iter()
            .filter_map(|reference| {
                let id = reference.id.trim().to_string();
                let file_path = ide_context_display_path(&reference.file_path);
                let content = reference.content.trim().to_string();
                let source = reference.source.trim().to_string();
                let allow_empty_content = source == "active_file";
                if id.is_empty() || file_path.is_empty() || (!allow_empty_content && content.is_empty()) {
                    return None;
                }
                Some(IdeContextReference {
                    id,
                    file_path,
                    start_line: reference.start_line,
                    end_line: reference.end_line,
                    content,
                    language_id: reference
                        .language_id
                        .map(|value| value.trim().to_string())
                        .filter(|value| !value.is_empty()),
                    source,
                    captured_at: ide_context_normalize_time_or_now(
                        "references[].capturedAt",
                        &reference.captured_at,
                    ),
                })
            })
            .collect(),
        updated_at: updated_at.clone(),
    };
    let mut snapshots = runtime
        .snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;
    snapshots.insert(client_id.clone(), snapshot);
    Ok((client_id, updated_at))
}

#[tauri::command]
fn upsert_ide_context_snapshot(
    input: UpsertIdeContextSnapshotInput,
    state: State<'_, AppState>,
    ide_context_runtime: State<'_, IdeContextRuntime>,
) -> Result<(), String> {
    let (client_id, updated_at) =
        upsert_ide_context_snapshot_internal(input, ide_context_runtime.inner())?;
    emit_ide_context_updated(&state, &client_id, &updated_at);
    Ok(())
}

#[tauri::command]
fn query_ide_context_references(
    input: IdeContextWorkspaceQueryInput,
    ide_context_runtime: State<'_, IdeContextRuntime>,
) -> Result<IdeContextQueryResultOutput, String> {
    query_ide_context_references_internal(input, ide_context_runtime.inner())
}

#[tauri::command]
async fn get_web_access_info(
    app: AppHandle,
    state: State<'_, AppState>,
    ide_context_runtime: State<'_, IdeContextRuntime>,
    input: Option<GetWebAccessInfoInput>,
) -> Result<WebAccessInfoOutput, String> {
    get_web_access_info_inner(
        &app,
        &state,
        &ide_context_runtime,
        input.unwrap_or_default().force_refresh,
    )
    .await
}

async fn get_web_access_info_inner(
    app: &AppHandle,
    state: &AppState,
    ide_context_runtime: &IdeContextRuntime,
    force_refresh: bool,
) -> Result<WebAccessInfoOutput, String> {
    let status_snapshot = ide_context_port_service_core()
        .status_snapshot(WEB_ACCESS_SERVICE_ID)
        .await;
    let config = state_read_config_cached(&state)?;
    let configured_port = normalize_web_access_port(config.web_access_port);
    if !config.web_access_enabled {
        return Ok(WebAccessInfoOutput {
            running: false,
            enabled: false,
            configured_port,
            port: configured_port,
            listen_addr: status_snapshot.listen_addr,
            status_text: status_snapshot.status_text,
            last_error: status_snapshot.last_error,
            local_url: String::new(),
            remote_urls: Vec::new(),
            remote_password: String::new(),
            active_connections: Vec::new(),
        });
    }
    if !IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst)
        && !ide_context_bridge_server_task_is_running()
    {
        start_web_access_server(
            app.clone(),
            state.clone(),
            ide_context_runtime.clone(),
        )
        .await;
    }
    let status_snapshot = ide_context_port_service_core()
        .status_snapshot(WEB_ACCESS_SERVICE_ID)
        .await;
    let running = IDE_CONTEXT_BRIDGE_STARTED.load(Ordering::SeqCst);
    let actual_port = ide_context_current_port(ide_context_runtime);
    let port = actual_port.unwrap_or(configured_port);
    let (local_url, remote_urls) = match actual_port {
        Some(actual_port) => (
            ide_context_sidebar_url_for_host(IDE_CONTEXT_BRIDGE_HOST, actual_port),
            ide_context_get_cached_lan_hosts(ide_context_runtime, force_refresh)?
                .into_iter()
                .map(|host| ide_context_sidebar_url_for_host(&host, actual_port))
                .collect::<Vec<_>>(),
        ),
        None => (String::new(), Vec::new()),
    };
    Ok(WebAccessInfoOutput {
        running,
        enabled: true,
        configured_port,
        port,
        listen_addr: status_snapshot.listen_addr,
        status_text: status_snapshot.status_text,
        last_error: status_snapshot.last_error,
        local_url,
        remote_urls,
        remote_password: ide_context_effective_remote_password(state, ide_context_runtime)?,
        active_connections: web_access_connection_summaries(),
    })
}

fn ide_context_get_cached_lan_hosts(
    ide_context_runtime: &IdeContextRuntime,
    force_refresh: bool,
) -> Result<Vec<String>, String> {
    let mut cache = ide_context_runtime
        .web_access_cache
        .lock()
        .map_err(|_| "Failed to lock web access cache".to_string())?;
    if !force_refresh {
        if let Some(lan_hosts) = cache.lan_hosts.clone() {
            return Ok(lan_hosts);
        }
    }
    let lan_hosts = ide_context_lan_hosts();
    cache.lan_hosts = Some(lan_hosts.clone());
    Ok(lan_hosts)
}

fn query_ide_context_references_internal(
    input: IdeContextWorkspaceQueryInput,
    ide_context_runtime: &IdeContextRuntime,
) -> Result<IdeContextQueryResultOutput, String> {
    let workspaces: Vec<IdeContextWorkspaceInput> = input
        .workspaces
        .into_iter()
        .filter(|workspace| !workspace.path.trim().is_empty())
        .collect();
    if workspaces.is_empty() {
        return Ok(IdeContextQueryResultOutput {
            groups: Vec::new(),
            updated_at: String::new(),
        });
    }

    let mut snapshots = ide_context_runtime
        .snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;
    ide_context_prune_expired_snapshots(&mut snapshots);

    let mut groups = workspaces
        .iter()
        .map(|workspace| IdeContextWorkspaceGroupOutput {
            workspace_path: ide_context_display_path(&workspace.path),
            workspace_name: ide_context_workspace_name(workspace),
            references: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut latest_updated_at = String::new();

    for snapshot in snapshots.values() {
        if ide_context_timestamp_is_newer(&snapshot.updated_at, &latest_updated_at) {
            latest_updated_at = snapshot.updated_at.clone();
        }
        for reference in &snapshot.references {
            for group in &mut groups {
                if !ide_context_path_is_within_workspace(&reference.file_path, &group.workspace_path) {
                    continue;
                }
                let file_path = ide_context_display_path(&reference.file_path);
                let file_name = std::path::Path::new(&file_path)
                    .file_name()
                    .and_then(|value| value.to_str())
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| file_path.clone());
                let relative_path = ide_context_relative_display_path(&file_path, &group.workspace_path);
                let display_label = format!(
                    "{}{}",
                    file_name,
                    ide_context_line_suffix(reference.start_line, reference.end_line)
                );
                let text_block = ide_context_text_block(&file_path, reference);
                group.references.push(IdeContextReferenceItemOutput {
                    id: format!("{}:{}:{}", snapshot.client_id, reference.id, reference.captured_at),
                    workspace_path: group.workspace_path.clone(),
                    workspace_name: group.workspace_name.clone(),
                    file_path,
                    file_name,
                    relative_path,
                    start_line: reference.start_line,
                    end_line: reference.end_line,
                    display_label,
                    content: reference.content.clone(),
                    language_id: reference.language_id.clone(),
                    source: reference.source.clone(),
                    captured_at: reference.captured_at.clone(),
                    text_block,
                });
                break;
            }
        }
    }

    for group in &mut groups {
        let mut latest_by_file = std::collections::HashMap::<String, IdeContextReferenceItemOutput>::new();
        for item in group.references.drain(..) {
            let key = ide_context_reference_dedup_key(&item);
            let should_replace = latest_by_file
                .get(&key)
                .map(|existing| ide_context_should_replace_reference(&item, existing))
                .unwrap_or(true);
            if should_replace {
                latest_by_file.insert(key, item);
            }
        }
        group.references = latest_by_file.into_values().collect();
        group.references.sort_by(|left, right| {
            ide_context_timestamp_compare_desc(&left.captured_at, &right.captured_at)
                .then_with(|| left.display_label.cmp(&right.display_label))
        });
    }
    groups.retain(|group| !group.references.is_empty());

    Ok(IdeContextQueryResultOutput {
        groups,
        updated_at: latest_updated_at,
    })
}

include!("ide_context/jsonrpc_methods.rs");

include!("ide_context/jsonrpc_dispatch.rs");

include!("ide_context/bridge_server.rs");

#[cfg(test)]
mod ide_context_tests {
    use super::*;
    static IDE_CONTEXT_TEST_MUTEX: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();

    fn ide_context_test_lock() -> std::sync::MutexGuard<'static, ()> {
        IDE_CONTEXT_TEST_MUTEX
            .get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("lock ide context test mutex")
    }

    fn ide_context_test_state() -> AppState {
        let root = std::env::temp_dir().join(format!("eca-ide-context-test-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("create temp test root");
        std::fs::create_dir_all(root.join("llm-workspace")).expect("create temp llm workspace");
        AppState {
            app_handle: Arc::new(Mutex::new(None)),
            config_path: root.join("app_config.toml"),
            data_path: root.join("app_data.json"),
            llm_workspace_path: root.join("llm-workspace"),
            shared_http_client: reqwest::Client::new(),
            terminal_shell: detect_default_terminal_shell(),
            terminal_shell_candidates: detect_terminal_shell_candidates(),
            conversation_lock: Arc::new(ConversationDomainLock::new()),
            memory_lock: Arc::new(Mutex::new(())),
            cached_config: Arc::new(Mutex::new(None)),
            cached_config_mtime: Arc::new(Mutex::new(None)),
            cached_agents: Arc::new(Mutex::new(None)),
            cached_agents_mtime: Arc::new(Mutex::new(None)),
            cached_runtime_state: Arc::new(Mutex::new(None)),
            cached_runtime_state_mtime: Arc::new(Mutex::new(None)),
            cached_chat_index: Arc::new(Mutex::new(None)),
            cached_conversation_metadata: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_mtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_app_data: Arc::new(Mutex::new(None)),
            cached_app_data_signature: Arc::new(Mutex::new(None)),
            cached_app_data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_pending: Arc::new(Mutex::new(None)),
            app_data_persist_notify: Arc::new(tokio::sync::Notify::new()),
            app_data_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            conversation_persist_pending: Arc::new(Mutex::new(None)),
            conversation_persist_notify: Arc::new(tokio::sync::Notify::new()),
            conversation_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            conversation_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cached_conversation_dirty_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            cached_deleted_conversation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            app_data_persist_write_lock: Arc::new(Mutex::new(())),
            last_panic_snapshot: Arc::new(Mutex::new(None)),
            inflight_chat_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_tool_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_completed_tool_history: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_session_roots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_live_sessions: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            terminal_pending_approvals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            llm_round_logs: Arc::new(Mutex::new(RecentLlmRoundLogs::default())),
            conversation_runtime_slots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_processing_claims: Arc::new(Mutex::new(std::collections::HashSet::new())),
            goal_continue_suppressed_conversation_ids: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            pending_chat_result_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_chat_delta_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            accepted_submit_trace_ids: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            active_chat_view_bindings: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_list_activity_marks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dequeue_lock: Arc::new(Mutex::new(())),
            task_scheduler_notify: Arc::new(tokio::sync::Notify::new()),
            delegate_runtime_threads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_recent_threads: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            provider_streaming_disabled_keys: Arc::new(Mutex::new(std::collections::HashMap::new())),
            provider_system_message_user_fallback_keys: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            provider_request_gates: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
            remote_im_contact_runtime_states: Arc::new(Mutex::new(std::collections::HashMap::new())),
            remote_im_reply_delegate_runtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            remote_im_reply_delegate_semaphore: Arc::new(tokio::sync::Semaphore::new(8)),
            remote_im_channel_state_write_locks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            hidden_skill_snapshot_cache: Arc::new(Mutex::new(String::new())),
            preferred_release_source: Arc::new(Mutex::new("github".to_string())),
            migration_preview_dirs: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_active_ids: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            backend_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    #[test]
    fn ide_context_remote_password_accepts_human_input_format() {
        let runtime = IdeContextRuntime::new();
        let password = ide_context_remote_password(&runtime).expect("remote password");
        let compact_lowercase = password.replace('-', "").to_ascii_lowercase();

        assert!(ide_context_verify_remote_password(&runtime, None, &password).expect("verify password"));
        assert!(
            ide_context_verify_remote_password(&runtime, None, &compact_lowercase)
                .expect("verify compact password")
        );
        assert!(!ide_context_verify_remote_password(&runtime, None, "").expect("reject empty"));
        assert!(!ide_context_verify_remote_password(&runtime, None, "wrong-password").expect("reject wrong"));
    }

    #[test]
    fn ide_context_peer_is_local_only_allows_loopback() {
        let ipv4_local: std::net::SocketAddr = "127.0.0.1:8429".parse().expect("ipv4 local");
        let ipv6_local: std::net::SocketAddr = "[::1]:8429".parse().expect("ipv6 local");
        let remote: std::net::SocketAddr = "192.168.1.10:8429".parse().expect("remote");

        assert!(ide_context_peer_is_local(&ipv4_local));
        assert!(ide_context_peer_is_local(&ipv6_local));
        assert!(!ide_context_peer_is_local(&remote));
    }

    fn ide_context_ws_test_request(origin: Option<&str>) -> Request {
        ide_context_ws_test_request_with_host(origin, "127.0.0.1:8429")
    }

    fn ide_context_ws_test_request_with_host(origin: Option<&str>, host: &str) -> Request {
        let mut builder = Request::builder()
            .uri(IDE_CONTEXT_CHAT_BRIDGE_PATH)
            .header("host", host);
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        builder.body(()).expect("build websocket request")
    }

    #[test]
    fn ide_context_ws_origin_allows_owned_pages_and_vscode_webview() {
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(None),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("vscode-webview://abc123")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("http://127.0.0.1:8429")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(ide_context_ws_origin_allowed(
            &ide_context_ws_test_request_with_host(Some("http://192.168.1.20:8429"), "192.168.1.20:8429"),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
    }

    #[test]
    fn ide_context_ws_origin_rejects_external_pages() {
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("https://example.com")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("http://example.com:8429")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("http://127.0.0.1:43130")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request_with_host(Some("http://192.168.1.50:8429"), "127.0.0.1:8429"),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
        assert!(!ide_context_ws_origin_allowed(
            &ide_context_ws_test_request(Some("null")),
            IDE_CONTEXT_BRIDGE_BASE_PORT
        ));
    }

    #[test]
    fn ide_context_lan_host_filter_rejects_reserved_and_accepts_private_lan() {
        let mihomo: std::net::Ipv4Addr = "198.18.0.1".parse().expect("mihomo ip");
        let hyperv: std::net::Ipv4Addr = "192.168.240.1".parse().expect("hyperv ip");
        let ethernet: std::net::Ipv4Addr = "192.168.5.23".parse().expect("ethernet ip");
        let cgnat: std::net::Ipv4Addr = "100.64.1.2".parse().expect("cgnat ip");

        assert!(!ide_context_ipv4_is_remote_link_candidate(mihomo));
        assert!(!ide_context_ipv4_is_remote_link_candidate(cgnat));
        assert!(ide_context_ipv4_is_remote_link_candidate(hyperv));
        assert!(ide_context_ipv4_is_remote_link_candidate(ethernet));
    }

    #[test]
    fn ide_context_lan_host_rank_prefers_real_gateway_adapter() {
        let ethernet = IdeContextLanHostCandidate {
            ip: "192.168.5.23".parse().expect("ethernet ip"),
            adapter_name: "以太网".to_string(),
            adapter_description: "Realtek PCIe GbE Family Controller".to_string(),
            has_gateway: true,
            active: true,
        };
        let hyperv = IdeContextLanHostCandidate {
            ip: "192.168.240.1".parse().expect("hyperv ip"),
            adapter_name: "vEthernet (Default Switch)".to_string(),
            adapter_description: "Hyper-V Virtual Ethernet Adapter".to_string(),
            has_gateway: false,
            active: true,
        };

        assert!(ide_context_lan_host_rank(&ethernet) < ide_context_lan_host_rank(&hyperv));
    }

    #[tokio::test]
    async fn ide_context_bind_listener_should_fail_when_fixed_port_is_occupied() {
        let occupied_port = IDE_CONTEXT_BRIDGE_BASE_PORT;
        let occupied_listener = match std::net::TcpListener::bind((IDE_CONTEXT_BRIDGE_BIND_HOST, occupied_port)) {
            Ok(listener) => listener,
            Err(_) => return,
        };

        let error = bind_ide_context_bridge_listener(occupied_port)
            .await
            .expect_err("bind should fail on occupied fixed port");

        drop(occupied_listener);
        assert!(error.contains("已被占用"));
    }

    #[tokio::test]
    async fn local_port_service_restart_serialized_should_run_exclusively_per_service() {
        let core = std::sync::Arc::new(LocalPortServiceCore::new());
        let active = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let peak = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let first_core = core.clone();
        let first_active = active.clone();
        let first_peak = peak.clone();
        let first = tokio::spawn(async move {
            first_core
                .restart_serialized("web_access", || async move {
                    let current = first_active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    first_peak.fetch_max(current, std::sync::atomic::Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    first_active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                })
                .await
        });
        let second_core = core.clone();
        let second_active = active.clone();
        let second_peak = peak.clone();
        let second = tokio::spawn(async move {
            second_core
                .restart_serialized("web_access", || async move {
                    let current = second_active.fetch_add(1, std::sync::atomic::Ordering::SeqCst) + 1;
                    second_peak.fetch_max(current, std::sync::atomic::Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
                    second_active.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    Ok(())
                })
                .await
        });

        first.await.expect("first task join").expect("first task ok");
        second.await.expect("second task join").expect("second task ok");
        assert_eq!(peak.load(std::sync::atomic::Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn shutdown_web_access_server_should_update_runtime_snapshot() {
        let _test_guard = ide_context_test_lock();
        let port_service = ide_context_port_service_core();
        port_service.clear_runtime_state(WEB_ACCESS_SERVICE_ID).await;
        port_service
            .set_listen_addr(WEB_ACCESS_SERVICE_ID, Some("0.0.0.0:8429".to_string()))
            .await;
        port_service
            .set_status_text(WEB_ACCESS_SERVICE_ID, Some("listening".to_string()))
            .await;
        port_service.set_last_error(WEB_ACCESS_SERVICE_ID, None).await;
        IDE_CONTEXT_BRIDGE_STARTED.store(true, Ordering::SeqCst);

        let shutdown_token = ide_context_bridge_create_shutdown_token();
        let task = tauri::async_runtime::spawn(async move {
            shutdown_token.cancelled().await;
        });
        ide_context_bridge_set_server_task(task);

        shutdown_web_access_server().await;

        let snapshot = port_service.status_snapshot(WEB_ACCESS_SERVICE_ID).await;
        let logs = port_service.get_logs(WEB_ACCESS_SERVICE_ID).await;
        assert_eq!(snapshot.status_text.as_deref(), Some("stopped"));
        assert!(snapshot.listen_addr.is_empty());
        assert!(snapshot.last_error.is_none());
        assert!(
            logs.iter().any(|entry| entry.message.contains("服务已停止")),
            "shutdown should append stop log"
        );

        port_service.clear_runtime_state(WEB_ACCESS_SERVICE_ID).await;
        IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);
    }

    #[tokio::test]
    async fn shutdown_web_access_server_should_clear_stale_error_when_already_stopped() {
        let _test_guard = ide_context_test_lock();
        let port_service = ide_context_port_service_core();
        port_service
            .set_listen_addr(WEB_ACCESS_SERVICE_ID, Some("0.0.0.0:8429".to_string()))
            .await;
        port_service
            .set_status_text(WEB_ACCESS_SERVICE_ID, Some("error".to_string()))
            .await;
        port_service
            .set_last_error(WEB_ACCESS_SERVICE_ID, Some("stale failure".to_string()))
            .await;
        IDE_CONTEXT_BRIDGE_STARTED.store(false, Ordering::SeqCst);

        shutdown_web_access_server().await;

        let snapshot = port_service.status_snapshot(WEB_ACCESS_SERVICE_ID).await;
        assert_eq!(snapshot.status_text.as_deref(), Some("stopped"));
        assert!(snapshot.listen_addr.is_empty());
        assert!(snapshot.last_error.is_none());

        port_service.clear_runtime_state(WEB_ACCESS_SERVICE_ID).await;
    }

    #[test]
    fn ide_context_bridge_tokens_allow_concurrent_consumers_until_expiry() {
        let runtime = IdeContextRuntime::new();
        let token = ide_context_issue_bridge_token_with_state(&runtime, None).expect("issue token");

        let next_token = ide_context_consume_bridge_token_with_state(&runtime, None, &token)
            .expect("first consume");
        assert_eq!(next_token, token);

        let second_next = ide_context_consume_bridge_token_with_state(&runtime, None, &token)
            .expect("second consume with same token");
        assert_eq!(second_next, token);
    }

    #[test]
    fn ide_context_bridge_tokens_keep_multiple_login_tokens() {
        let runtime = IdeContextRuntime::new();
        let first = ide_context_issue_bridge_token_with_state(&runtime, None).expect("issue first token");
        let second = ide_context_issue_bridge_token_with_state(&runtime, None).expect("issue second token");

        assert_ne!(first, second);
        assert_eq!(
            ide_context_consume_bridge_token_with_state(&runtime, None, &first)
                .expect("first token remains valid"),
            first
        );
        assert_eq!(
            ide_context_consume_bridge_token_with_state(&runtime, None, &second)
                .expect("second token remains valid"),
            second
        );
    }

    #[test]
    fn ide_context_bridge_tokens_reject_unknown_token() {
        let runtime = IdeContextRuntime::new();
        let _ = ide_context_issue_bridge_token_with_state(&runtime, None).expect("issue token");
        let err = ide_context_consume_bridge_token_with_state(&runtime, None, "bad-token")
            .expect_err("invalid token");
        assert!(err.0.contains("invalid authToken"));
    }

    #[test]
    fn ide_context_bridge_tokens_reissue_when_cache_expired() {
        let runtime = IdeContextRuntime::new();
        {
            let mut auth = runtime.bridge_auth.lock().expect("lock auth");
            auth.valid_tokens.insert(
                "expired-token".to_string(),
                time::OffsetDateTime::now_utc() - time::Duration::seconds(1),
            );
        }

        let err = ide_context_consume_bridge_token_with_state(&runtime, None, "expired-token")
            .expect_err("expired token should refresh discovery");
        assert!(err.0.contains("expired"));
        let refreshed = err.1.expect("should issue refreshed token");
        let auth = runtime.bridge_auth.lock().expect("lock auth");
        assert!(auth.valid_tokens.contains_key(&refreshed));
    }

    #[test]
    fn ide_chat_send_message_should_return_formal_assistant_message_id_immediately() {
        let state = ide_context_test_state();
        let created = conversation_service_v2()
            .create_conversation(
                &state,
                &CreateUnarchivedConversationInput {
                    api_config_id: None,
                    agent_id: Some(DEFAULT_AGENT_ID.to_string()),
                    department_id: Some(ASSISTANT_DEPARTMENT_ID.to_string()),
                    title: Some("IDE发送即时assistant气泡".to_string()),
                    copy_source_conversation_id: None,
                    shell_workspaces: None,
                    shell_autonomous_mode: None,
                },
            )
            .expect("create conversation");

        let result = ide_chat_send_message(
            &state,
            serde_json::json!({
                "conversationId": created.conversation_id,
                "text": "你好",
            }),
        )
        .expect("send message");

        let assistant_message_id = result
            .get("assistantMessageId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let user_message_id = result
            .get("userMessageId")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let accepted = result
            .get("accepted")
            .and_then(Value::as_bool);
        let ingress = result
            .get("ingress")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();

        assert!(
            !user_message_id.is_empty(),
            "chat.send 应立即返回正式 userMessageId"
        );
        assert!(
            !assistant_message_id.is_empty(),
            "chat.send 应立即返回正式 assistantMessageId"
        );
        assert_eq!(accepted, Some(true));
        assert!(!ingress.is_empty(), "chat.send 应返回 ingress");
    }
}
