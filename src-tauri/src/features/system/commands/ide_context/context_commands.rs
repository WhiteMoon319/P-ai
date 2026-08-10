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

fn ide_chat_parse_workspace_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    match ide_chat_parse_params::<T>(params.clone()) {
        Ok(value) => Ok(value),
        Err(_) => ide_chat_parse_param_field::<T>(params, "input"),
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspacePermissionInput {
    conversation_id: String,
    access: String,
    workspace_path: Option<String>,
    workspace_name: Option<String>,
}

fn ide_chat_workspace_permission_payload(state: &AppState, conversation: &Conversation) -> Result<Value, String> {
    let workspaces = terminal_allowed_workspaces_for_conversation_canonical(state, Some(conversation))?;
    let main = workspaces.iter().find(|w| w.level == SHELL_WORKSPACE_LEVEL_MAIN)
        .or_else(|| workspaces.iter().find(|w| w.level == SHELL_WORKSPACE_LEVEL_SYSTEM));
    Ok(serde_json::json!({
        "access": main.map(|w| w.access.trim()).filter(|v| !v.is_empty()).unwrap_or(SHELL_WORKSPACE_ACCESS_APPROVAL),
        "workspaceName": main.map(|w| w.name.clone()).unwrap_or_default(),
        "rootPath": main.map(|w| w.path.to_string_lossy().to_string()).unwrap_or_default(),
    }))
}

fn ide_chat_workspace_permission(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatConversationInput>(params)?;
    let meta = conversation_service_v2().get_conversation_meta(state, input.conversation_id.trim())?;
    ide_chat_workspace_permission_payload(state, &ide_chat_conversation_from_meta_view(&meta))
}

fn ide_chat_select_workspace_permission(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<IdeChatWorkspacePermissionInput>(params)?;
    let conversation_id = input.conversation_id.trim();
    if conversation_id.is_empty() { return Err("conversationId is required".to_string()); }
    let access = match input.access.trim() {
        SHELL_WORKSPACE_ACCESS_READ_ONLY | SHELL_WORKSPACE_ACCESS_APPROVAL | SHELL_WORKSPACE_ACCESS_FULL_ACCESS => input.access.trim().to_string(),
        _ => return Err("Unsupported workspace access".to_string()),
    };
    let meta = conversation_service_v2().get_conversation_meta(state, conversation_id)?;
    let mut workspaces = meta.shell_workspaces.clone();
    let mut changed = false;
    for workspace in &mut workspaces { if normalize_shell_workspace_level_text(&workspace.level) == SHELL_WORKSPACE_LEVEL_MAIN { workspace.access = access.clone(); changed = true; } }
    if !changed {
        let path = input.workspace_path.as_deref().map(str::trim).unwrap_or_default();
        if path.is_empty() { return Err("当前会话没有主工作目录，无法设置权限。".to_string()); }
        let fallback = path.replace('\\', "/").trim_end_matches('/').rsplit('/').next().unwrap_or("VS Code").to_string();
        workspaces.push(ShellWorkspaceConfig { id: "vscode-sidebar-main-workspace".to_string(), name: input.workspace_name.as_deref().map(str::trim).filter(|v| !v.is_empty()).unwrap_or(fallback.as_str()).to_string(), path: path.to_string(), level: SHELL_WORKSPACE_LEVEL_MAIN.to_string(), access: access.clone(), built_in: false });
    }
    let updated = apply_conversation_chat_workspace_changes(state, conversation_id, Some(None), Some(normalize_conversation_shell_workspaces(state, &workspaces)), None, None)?;
    ide_chat_workspace_permission_payload(state, &updated)
}

fn ide_chat_workspace_layout_save(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_workspace_params::<SaveChatShellWorkspacesInput>(params)?;
    ide_chat_serialize(update_chat_shell_workspace_layout_inner(input, state)?)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceDirectoryListInput { path: String }

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatFileReaderReadInput { path: String }

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatFileReaderReadBlockInput { path: String, start_line: usize, line_count: usize }

fn ide_chat_workspace_list(state: &AppState, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_workspace_params::<ChatShellWorkspaceInput>(params)?;
    ide_chat_serialize(get_chat_shell_workspace_inner(input, state)?)
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct IdeChatWorkspaceGitRootCheckInput {
    workspace_path: String,
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
    let mut workspaces: Vec<IdeContextWorkspaceInput> = input
        .workspaces
        .into_iter()
        .filter(|workspace| !workspace.path.trim().is_empty())
        .collect();

    let mut snapshots = ide_context_runtime
        .snapshots
        .lock()
        .map_err(|_| "Failed to lock ide context snapshots".to_string())?;
    ide_context_prune_expired_snapshots(&mut snapshots);

    // Web 页面不会携带 VS Code 工作区；此时从仍有效的 VS Code 快照恢复工作区，
    // 以便展示 IDE 桥同步的当前打开文件。
    if workspaces.is_empty() {
        let mut workspace_paths = snapshots
            .values()
            .filter(|snapshot| snapshot.editor.eq_ignore_ascii_case("vscode"))
            .flat_map(|snapshot| snapshot.workspace_roots.iter())
            .map(|path| ide_context_display_path(path))
            .filter(|path| !path.trim().is_empty())
            .collect::<Vec<_>>();
        workspace_paths.sort_by(|left, right| {
            ide_context_compare_key(left).cmp(&ide_context_compare_key(right))
        });
        workspace_paths.dedup_by(|left, right| {
            ide_context_compare_key(left) == ide_context_compare_key(right)
        });
        workspaces = workspace_paths
            .into_iter()
            .map(|path| IdeContextWorkspaceInput { path, name: None })
            .collect();
    }
    if workspaces.is_empty() {
        return Ok(IdeContextQueryResultOutput {
            groups: Vec::new(),
            updated_at: String::new(),
        });
    }

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

#[cfg(test)]
mod ide_context_query_tests {
    use super::*;

    fn upsert_test_snapshot(
        runtime: &IdeContextRuntime,
        client_id: &str,
        workspace_root: &str,
        file_path: &str,
    ) {
        let result = upsert_ide_context_snapshot_internal(
            UpsertIdeContextSnapshotInput {
                client_id: client_id.to_string(),
                auth_token: None,
                editor: "vscode".to_string(),
                workspace_roots: vec![workspace_root.to_string()],
                references: vec![IdeContextReferenceInput {
                    id: "active".to_string(),
                    file_path: file_path.to_string(),
                    start_line: Some(1),
                    end_line: Some(1),
                    content: "const value = 1;".to_string(),
                    language_id: Some("typescript".to_string()),
                    source: "active_file".to_string(),
                    captured_at: now_iso(),
                }],
                updated_at: Some(now_iso()),
            },
            runtime,
        );
        assert!(result.is_ok());
    }

    #[test]
    fn empty_workspace_query_uses_active_vscode_snapshot_roots() {
        let runtime = IdeContextRuntime::new();
        upsert_test_snapshot(
            &runtime,
            "vscode-client",
            "E:/repo",
            "E:/repo/src/main.ts",
        );

        let result = query_ide_context_references_internal(
            IdeContextWorkspaceQueryInput { workspaces: Vec::new() },
            &runtime,
        );

        assert!(result.is_ok());
        let result = match result {
            Ok(value) => value,
            Err(error) => panic!("query failed: {error}"),
        };
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].workspace_path, "E:/repo");
        assert_eq!(result.groups[0].references.len(), 1);
        assert_eq!(result.groups[0].references[0].file_path, "E:/repo/src/main.ts");
    }

    #[test]
    fn explicit_workspace_query_stays_scoped_to_requested_workspace() {
        let runtime = IdeContextRuntime::new();
        upsert_test_snapshot(
            &runtime,
            "vscode-client-a",
            "E:/repo-a",
            "E:/repo-a/src/a.ts",
        );
        upsert_test_snapshot(
            &runtime,
            "vscode-client-b",
            "E:/repo-b",
            "E:/repo-b/src/b.ts",
        );

        let result = query_ide_context_references_internal(
            IdeContextWorkspaceQueryInput {
                workspaces: vec![IdeContextWorkspaceInput {
                    path: "E:/repo-b".to_string(),
                    name: Some("repo-b".to_string()),
                }],
            },
            &runtime,
        );

        assert!(result.is_ok());
        let result = match result {
            Ok(value) => value,
            Err(error) => panic!("query failed: {error}"),
        };
        assert_eq!(result.groups.len(), 1);
        assert_eq!(result.groups[0].references.len(), 1);
        assert_eq!(result.groups[0].references[0].file_path, "E:/repo-b/src/b.ts");
    }
}
