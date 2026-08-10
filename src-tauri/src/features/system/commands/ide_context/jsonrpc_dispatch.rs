const WEB_NATIVE_CAPABILITY_UNAVAILABLE: &str = "WEB_NATIVE_CAPABILITY_UNAVAILABLE";

fn ide_chat_web_native_only_method(method: &str) -> bool {
    matches!(
        method,
            | "list_file_reader_directory"
            | "list_file_reader_directory_open_targets"
            | "read_file_reader_file"
            | "read_file_reader_file_block"
            | "read_plan_file_content"
            | "open_file_reader_directory_target"
            | "open_file_reader_directory_shell"
            | "open_file_with_default_program"
            | "open_local_file_directory"
            | "open_workspace_file"
            | "open_storage_usage_item_directory"
            | "open_chat_shell_workspace_dir"
            | "mcp_open_workspace_dir"
            | "skill_open_workspace_dir"
            | "copy_local_chat_image_to_clipboard"
            | "save_local_chat_image_as"
            | "export_archive_to_file"
            | "archives.export"
            | "conversation.importShare"
            | "export_memories_to_path"
            | "export_agent_private_memories"
            | "write_base64_file_to_path"
            | "write_utf8_text_file_to_path"
            | "queue_local_file_attachment"
            | "attachment_transfer_begin"
            | "attachment_transfer_chunk"
            | "attachment_transfer_complete"
            | "attachment_transfer_abort"
            | "attachment_ingest_local_path"
            | "update_file_reader_watch_targets"
            | "migrate_shell_workspace_directory"
            | "desktop_screenshot"
            | "demo_send_native_notification"
            | "demo_test_notification"
            | "demo_restart_app"
            | "xcap"
            | "start_current_window_drag"
            | "toggle_current_window_maximize"
            | "hide_current_window"
            | "update_record_hotkey"
            | "update_record_background_wake"
            | "install_host_runtime_prerequisite"
            | "get_host_runtime_prerequisites"
            | "reset_chat_shell_workspace"
            | "get_default_chat_shell_workspace_path"
            | "open_external_url"
            | "show_main_window"
            | "show_chat_window"
            | "show_archives_window"
            | "open_runtime_logs_window"
            | "sync_tray_icon"
            | "get_github_update_state"
            | "check_github_update"
            | "start_github_update"
            | "cancel_github_update"
            | "apply_prepared_github_update"
            | "bind_active_chat_view_stream"
            | "probe_active_chat_view_stream"
            | "unbind_active_chat_view_stream"
            | "clear_window_chat_view_stream_bindings_command"
            | "set_chat_window_active"
            | "open_file_reader_window_command"
            | "read_local_binary_file"
            | "set_chat_window_side_expanded"
            | "show_quick_setup_window"
            | "complete_quick_setup_and_open_chat"
    )
}

fn ide_chat_web_native_only_error(method: &str) -> String {
    format!(
        "{}: Web 端不支持本机能力：{}",
        WEB_NATIVE_CAPABILITY_UNAVAILABLE, method
    )
}



#[cfg(test)]
mod web_native_capability_tests {
    use super::*;

    fn collect_frontend_source_files(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(root) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_frontend_source_files(&path, out);
            } else if matches!(path.extension().and_then(|value| value.to_str()), Some("ts" | "vue"))
                && !path.file_name().and_then(|value| value.to_str()).is_some_and(|name| {
                    name.ends_with(".spec.ts") || name.ends_with(".test.ts")
                })
            {
                out.push(path);
            }
        }
    }

    fn quoted_value_at(source: &str, quote_index: usize) -> Option<(String, usize)> {
        let quote = *source.as_bytes().get(quote_index)?;
        if quote != b'\'' && quote != b'"' {
            return None;
        }
        let bytes = source.as_bytes();
        let mut index = quote_index + 1;
        while index < bytes.len() {
            if bytes[index] == b'\\' {
                index = index.saturating_add(2);
                continue;
            }
            if bytes[index] == quote {
                return Some((source[quote_index + 1..index].to_string(), index + 1));
            }
            index += 1;
        }
        None
    }

    fn static_invoke_tauri_methods(source: &str) -> Vec<String> {
        let mut methods = Vec::new();
        let mut offset = 0usize;
        while let Some(relative) = source[offset..].find("invokeTauri") {
            let start = offset + relative + "invokeTauri".len();
            let Some(open_relative) = source[start..].find('(') else {
                break;
            };
            let mut index = start + open_relative + 1;
            while source.as_bytes().get(index).is_some_and(u8::is_ascii_whitespace) {
                index += 1;
            }
            if let Some((method, end)) = quoted_value_at(source, index) {
                methods.push(method);
                offset = end;
            } else {
                offset = index.saturating_add(1);
            }
        }
        methods
    }

    fn web_covered_methods(source: &str) -> std::collections::HashSet<String> {
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let dispatch_start = production
            .find("async fn ide_chat_handle_jsonrpc_request")
            .unwrap_or(production.len());
        let mut covered = std::collections::HashSet::new();
        let mut index = 0usize;
        while index < production.len() {
            let Some(relative) = production[index..].find(['\'', '"']) else {
                break;
            };
            let quote_index = index + relative;
            let Some((value, end)) = quoted_value_at(production, quote_index) else {
                break;
            };
            let is_native_declaration = quote_index < dispatch_start;
            let is_dispatch_arm = production[end..].trim_start().starts_with("=>");
            if is_native_declaration || is_dispatch_arm {
                covered.insert(value);
            }
            index = end;
        }
        covered
    }

    #[test]
    fn every_static_frontend_tauri_command_should_have_web_behavior() {
        let frontend_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../src");
        let mut files = Vec::new();
        collect_frontend_source_files(&frontend_root, &mut files);
        let mut invoked = std::collections::BTreeSet::new();
        for path in files {
            let source = std::fs::read_to_string(&path)
                .unwrap_or_else(|err| panic!("failed to read {}: {err}", path.display()));
            invoked.extend(static_invoke_tauri_methods(&source));
        }
        let covered = web_covered_methods(include_str!("jsonrpc_dispatch.rs"));
        let missing = invoked
            .into_iter()
            .filter(|method| !covered.contains(method))
            .collect::<Vec<_>>();
        assert!(
            missing.is_empty(),
            "frontend invokeTauri commands must be handled or explicitly rejected on Web: {missing:?}"
        );
    }

    #[test]
    fn local_file_and_window_methods_should_be_explicitly_native_only() {
        for method in [
            "read_file_reader_file",
            "open_storage_usage_item_directory",
            "mcp_open_workspace_dir",
            "migrate_shell_workspace_directory",
            "desktop_screenshot",
            "demo_send_native_notification",
            "demo_test_notification",
            "demo_restart_app",
            "show_main_window",
            "sync_tray_icon",
            "get_github_update_state",
            "check_github_update",
            "start_github_update",
            "cancel_github_update",
            "apply_prepared_github_update",
            "export_memories_to_path",
            "export_agent_private_memories",
            "bind_active_chat_view_stream",
            "probe_active_chat_view_stream",
            "unbind_active_chat_view_stream",
            "set_chat_window_active",
        ] {
            assert!(
                ide_chat_web_native_only_method(method),
                "method should be native-only: {method}"
            );
            assert!(
                ide_chat_web_native_only_error(method)
                    .starts_with("WEB_NATIVE_CAPABILITY_UNAVAILABLE:"),
                "method should use stable error code: {method}"
            );
        }
    }

    #[test]
    fn portable_business_methods_should_not_be_marked_native_only() {
        for method in [
            "conversation.list",
            "conversation.resumeSubscription",
            "conversation.streamProbe",
            "workspace.list",
            "check_git_workspace_root",
            "get_chat_shell_workspace",
            "update_chat_shell_workspace_layout",
            "workspace.directory.list",
            "fileReader.directory.list",
            "fileReader.readFile",
            "fileReader.readFileBlock",
            "read_local_chat_image_thumbnail",
            "read_local_chat_image_original",
            "conversation.plan.readFile",
            "conversation.rewindPreview",
            "conversation.archive",
            "conversation.compact",
            "conversation.foregroundLightSnapshot",
            "chat.send",
            "remote_im_list_contacts",
            "task.list",
            "mcp_list_servers",
            "set_github_update_method",
            "set_skipped_github_update_version",
            "list_recent_llm_round_logs",
            "get_usage_overview",
            "refresh_usage_overview",
            "get_usage_trail",
            "queue_inline_file_attachment",
            "attachment.transfer.begin",
            "attachment.transfer.complete",
            "attachment.transfer.abort",
            "submit_chat_message",
            "stop_chat_message",
            "get_chat_queue_snapshot",
            "get_main_session_state_snapshot",
            "recall_chat_queue_event",
            "mark_chat_queue_event_guided",
            "get_conversation_fast_request_turns",
            "get_conversation_runtime_snapshot",
            "get_foreground_conversation_light_snapshot",
            "get_foreground_conversation_freshness_snapshot",
            "get_unarchived_conversation_block_page",
            "get_unarchived_conversation_message_by_id",
            "get_active_conversation_messages_before",
            "request_conversation_messages_after_async",
            "mark_conversation_read",
            "set_active_unarchived_conversation",
            "rebind_unarchived_conversation_recipient",
            "rewind_conversation_from_message",
            "set_conversation_plan_mode",
            "set_conversation_preferred_model",
            "confirm_plan_and_continue",
            "resolve_terminal_approval",
            "goal_get_current",
            "goal_create_goal",
            "goal_cancel_goal",
            "query_ide_context_references",
            "list_archives",
            "get_archive_block_page",
            "get_archive_summary",
            "delete_archive",
            "unarchive_archive",
            "archive_conversation",
            "batch_archive_conversations",
            "list_conversation_delegate_statuses",
            "abort_delegate_conversation",
            "get_delegate_conversation_block_page",
            "delete_delegate_conversation",
            "branch_unarchived_conversation_from_selection",
            "create_conversation_branch_from_message",
            "submit_user_async_delegate",
            "delete_unarchived_conversation",
            "read_chat_image_data_url",
            "read_avatar_data_url",
            "messageStore.migration.check",
            "messageStore.migration.run",
            "stt_transcribe",
            "get_storage_usage_overview",
            "refresh_storage_usage_overview",
            "cleanup_storage_legacy_items",
            "configMigration.export",
            "configMigration.preview",
            "configMigration.apply",
            "export_config_migration_package",
            "preview_import_config_migration_package",
            "apply_import_config_migration_package",
            "codex_get_auth_status",
            "codex_start_oauth_login",
            "codex_get_rate_limits",
            "codex_consume_rate_limit_reset_credit",
            "codex_logout",
            "save_agent_avatar",
            "clear_agent_avatar",
            "generate_image",
            "check_tools_status",
            "list_terminal_shell_candidates",
            "preview_rewind_conversation_from_message",
            "convert_private_agent_to_main",
            "set_agent_private_memory_enabled",
            "remote_im_get_default_group_response_guidance",
            "remote_im_patch_contact_settings",
            "remote_im_reconfigure_channel_behavior",
            "create_side_chat_conversation",
        ] {
            assert!(
                !ide_chat_web_native_only_method(method),
                "portable method should remain available: {method}"
            );
        }
    }

    #[test]
    fn chat_send_stop_and_rewind_should_use_canonical_tauri_request_shapes() {
        let send = serde_json::json!({
            "payload": {
                "text": "hello",
                "images": [],
                "attachments": [],
                "extraTextBlocks": ["context"]
            },
            "session": {
                "apiConfigId": null,
                "departmentId": "department-1",
                "agentId": "agent-1",
                "conversationId": "conversation-1"
            }
        });
        let stop = serde_json::json!({
            "session": {
                "apiConfigId": null,
                "departmentId": "department-1",
                "agentId": "agent-1",
                "conversationId": "conversation-1"
            },
            "partialAssistantText": "visible text",
            "partialStreamBlocks": []
        });
        let rewind = serde_json::json!({
            "session": {
                "apiConfigId": null,
                "departmentId": "department-1",
                "agentId": "agent-1",
                "conversationId": "conversation-1"
            },
            "messageId": "message-1",
            "undoApplyPatch": true
        });

        assert!(serde_json::from_value::<SendChatRequest>(send).is_ok());
        assert!(serde_json::from_value::<StopChatRequest>(stop).is_ok());
        assert!(serde_json::from_value::<RewindConversationInput>(rewind).is_ok());
        assert!(serde_json::from_value::<SendChatRequest>(serde_json::json!({
            "conversationId": "conversation-1",
            "text": "legacy"
        }))
        .is_err());
        assert!(serde_json::from_value::<RewindConversationInput>(serde_json::json!({
            "conversationId": "conversation-1",
            "messageId": "message-1"
        }))
        .is_err());
    }
}
