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
const IDE_CONTEXT_BRIDGE_BASE_PORT: u16 = 43129;
const IDE_CONTEXT_BRIDGE_MAX_PORT: u16 = 43139;
const IDE_CONTEXT_BRIDGE_PATH: &str = "/ide-context";
const IDE_CONTEXT_BRIDGE_DISCOVERY_FILE: &str = "p-ai-ide-context-bridge.json";
static IDE_CONTEXT_BRIDGE_STARTED: AtomicBool = AtomicBool::new(false);

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
    host: String,
    port: u16,
    path: String,
    pid: u32,
    updated_at: String,
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

fn ide_context_bridge_url(port: u16) -> String {
    format!("ws://{}:{}{}", IDE_CONTEXT_BRIDGE_HOST, port, IDE_CONTEXT_BRIDGE_PATH)
}

fn ide_context_bridge_discovery_path() -> std::path::PathBuf {
    std::env::temp_dir().join(IDE_CONTEXT_BRIDGE_DISCOVERY_FILE)
}

fn publish_ide_context_bridge_discovery(port: u16) -> Result<(), String> {
    let url = ide_context_bridge_url(port);
    let payload = IdeContextBridgeDiscovery {
        url: url.clone(),
        bridge_url: url,
        host: IDE_CONTEXT_BRIDGE_HOST.to_string(),
        port,
        path: IDE_CONTEXT_BRIDGE_PATH.to_string(),
        pid: std::process::id(),
        updated_at: now_iso(),
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

async fn bind_ide_context_bridge_listener() -> Result<(tokio::net::TcpListener, u16), String> {
    let mut errors = Vec::new();
    for port in IDE_CONTEXT_BRIDGE_BASE_PORT..=IDE_CONTEXT_BRIDGE_MAX_PORT {
        let addr = format!("{}:{}", IDE_CONTEXT_BRIDGE_HOST, port);
        match tokio::net::TcpListener::bind(&addr).await {
            Ok(listener) => return Ok((listener, port)),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::AddrInUse {
                    eprintln!("[IDE 上下文桥] 端口占用，尝试顺延: {}", addr);
                } else {
                    eprintln!("[IDE 上下文桥] 监听失败，尝试下一个端口 {}: {}", addr, err);
                }
                errors.push(format!("{addr}: {err}"));
            }
        }
    }
    Err(format!(
        "No available IDE context bridge port in {}:{}-{} ({})",
        IDE_CONTEXT_BRIDGE_HOST,
        IDE_CONTEXT_BRIDGE_BASE_PORT,
        IDE_CONTEXT_BRIDGE_MAX_PORT,
        errors.join("; ")
    ))
}

fn upsert_ide_context_snapshot_internal(
    input: UpsertIdeContextSnapshotInput,
    state: &AppState,
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
        .map(ToOwned::to_owned)
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
                    captured_at: {
                        let captured_at = reference.captured_at.trim();
                        if captured_at.is_empty() {
                            now_iso()
                        } else {
                            captured_at.to_string()
                        }
                    },
                })
            })
            .collect(),
        updated_at: updated_at.clone(),
    };
    let mut snapshots = state
        .ide_context_snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;
    snapshots.insert(client_id.clone(), snapshot);
    Ok((client_id, updated_at))
}

#[tauri::command]
fn upsert_ide_context_snapshot(
    input: UpsertIdeContextSnapshotInput,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let (client_id, updated_at) = upsert_ide_context_snapshot_internal(input, &state)?;
    emit_ide_context_updated(&state, &client_id, &updated_at);
    Ok(())
}

#[tauri::command]
fn query_ide_context_references(
    input: IdeContextWorkspaceQueryInput,
    state: State<'_, AppState>,
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

    let snapshots = state
        .ide_context_snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;

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
        if latest_updated_at.is_empty() || snapshot.updated_at > latest_updated_at {
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
        group.references.sort_by(|left, right| {
            right
                .captured_at
                .cmp(&left.captured_at)
                .then_with(|| left.display_label.cmp(&right.display_label))
        });
        let mut seen = std::collections::HashSet::<String>::new();
        group.references.retain(|item| seen.insert(item.id.clone()));
    }
    groups.retain(|group| !group.references.is_empty());

    Ok(IdeContextQueryResultOutput {
        groups,
        updated_at: latest_updated_at,
    })
}

fn start_ide_context_bridge_server(state: AppState) {
    if IDE_CONTEXT_BRIDGE_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    tauri::async_runtime::spawn(async move {
        let (listener, port) = match bind_ide_context_bridge_listener().await {
            Ok(result) => result,
            Err(err) => {
                clear_ide_context_bridge_discovery();
                eprintln!("[IDE 上下文桥] 监听失败: {}", err);
                return;
            }
        };
        let bridge_url = ide_context_bridge_url(port);
        if let Err(err) = publish_ide_context_bridge_discovery(port) {
            eprintln!("[IDE 上下文桥] 写入发现文件失败: {}", err);
        }
        eprintln!("[IDE 上下文桥] 已监听 {}", bridge_url);
        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok(result) => result,
                Err(err) => {
                    eprintln!("[IDE 上下文桥] 接收连接失败: {}", err);
                    continue;
                }
            };
            let state_clone = state.clone();
            tauri::async_runtime::spawn(async move {
                ide_context_ws_handle_connection(stream, peer_addr, state_clone).await;
            });
        }
    });
}

async fn ide_context_ws_handle_connection(
    stream: tokio::net::TcpStream,
    peer_addr: std::net::SocketAddr,
    state: AppState,
) {
    let path_holder = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let path_holder_clone = path_holder.clone();
    let ws_stream = match accept_hdr_async(stream, move |request: &Request, response: Response| {
        if let Ok(mut slot) = path_holder_clone.lock() {
            *slot = request.uri().path().to_string();
        }
        Ok(response)
    })
    .await
    {
        Ok(ws_stream) => ws_stream,
        Err(err) => {
            eprintln!("[IDE 上下文桥] WebSocket 握手失败 {}: {}", peer_addr, err);
            return;
        }
    };
    let path = path_holder.lock().map(|value| value.clone()).unwrap_or_default();
    if path != IDE_CONTEXT_BRIDGE_PATH {
        eprintln!("[IDE 上下文桥] 非法路径 {} from {}", path, peer_addr);
        return;
    }
    eprintln!("[IDE 上下文桥] 客户端已连接: {}", peer_addr);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut connected_client_id = String::new();
    let _ = ws_sender
        .send(tokio_tungstenite::tungstenite::Message::Text(
            serde_json::json!({"type": "ready", "path": IDE_CONTEXT_BRIDGE_PATH}).to_string().into(),
        ))
        .await;
    while let Some(message) = ws_receiver.next().await {
        match message {
            Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                match serde_json::from_str::<UpsertIdeContextSnapshotInput>(&text) {
                    Ok(input) => match upsert_ide_context_snapshot_internal(input, &state) {
                        Ok((client_id, updated_at)) => {
                            connected_client_id = client_id.clone();
                            emit_ide_context_updated(&state, &client_id, &updated_at);
                            let _ = ws_sender
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::json!({"type": "ack", "ok": true}).to_string().into(),
                                ))
                                .await;
                        }
                        Err(err) => {
                            let _ = ws_sender
                                .send(tokio_tungstenite::tungstenite::Message::Text(
                                    serde_json::json!({"type": "ack", "ok": false, "error": err}).to_string().into(),
                                ))
                                .await;
                        }
                    },
                    Err(err) => {
                        let _ = ws_sender
                            .send(tokio_tungstenite::tungstenite::Message::Text(
                                serde_json::json!({"type": "ack", "ok": false, "error": format!("invalid json: {err}")}).to_string().into(),
                            ))
                            .await;
                    }
                }
            }
            Ok(tokio_tungstenite::tungstenite::Message::Ping(payload)) => {
                let _ = ws_sender.send(tokio_tungstenite::tungstenite::Message::Pong(payload)).await;
            }
            Ok(tokio_tungstenite::tungstenite::Message::Close(_)) => break,
            Ok(_) => {}
            Err(err) => {
                eprintln!("[IDE 上下文桥] 客户端消息错误 {}: {}", peer_addr, err);
                break;
            }
        }
    }
    if !connected_client_id.is_empty() {
        match state.ide_context_snapshots.lock() {
            Ok(mut snapshots) => {
                snapshots.remove(&connected_client_id);
            }
            Err(_) => {
                eprintln!("[IDE 上下文桥] 清理客户端缓存失败: {}", connected_client_id);
            }
        }
    }
    eprintln!("[IDE 上下文桥] 客户端已断开: {}", peer_addr);
}
