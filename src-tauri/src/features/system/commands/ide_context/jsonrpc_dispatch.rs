async fn ide_chat_handle_jsonrpc_request(
    request: IdeChatJsonRpcRequest,
    state: &AppState,
    app: &AppHandle,
    ide_context_runtime: &IdeContextRuntime,
    client_id: &str,
    opened_conversation_id: &mut Option<String>,
) -> Value {
    if request.jsonrpc.trim() != "2.0" {
        return ide_chat_jsonrpc_error(request.id, -32600, "jsonrpc must be 2.0");
    }
    let sidebar_label = ide_chat_sidebar_window_label(client_id);
    let sidebar_viewer_id = chat_viewer_id_for_window_label(&sidebar_label)
        .unwrap_or_else(|| format!("web:{}", client_id.trim()));
    let result = match request.method.as_str() {
        "bridge.ping" => Ok(serde_json::json!({
            "ok": true,
            "ts": chrono::Utc::now().to_rfc3339(),
        })),
        "conversation.list" => ide_chat_conversation_list(state, &sidebar_viewer_id),
        "conversation.open" => ide_chat_parse_params::<IdeChatConversationInput>(request.params)
            .and_then(|input| {
                let result = ide_chat_conversation_open_result(state, &input.conversation_id)?;
                ide_chat_register_sidebar_conversation(
                    state,
                    &input.conversation_id,
                    &sidebar_label,
                    opened_conversation_id,
                )?;
                if let Some(workspace_path) = input.workspace_path.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
                    let _ = ide_chat_ensure_sidebar_workspace(state, &input.conversation_id, workspace_path, input.workspace_name.as_deref());
                }
                Ok(result)
        }),
        "conversation.blockPage" => ide_chat_conversation_block_page(state, request.params),
        "conversation.fastRequestTurns" => ide_chat_conversation_fast_request_turns(state, request.params),
        "conversation.create" => (|| {
            let result = ide_chat_create_conversation(state, request.params)?;
            if let Some(conversation_id) = result
                .get("conversationId")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            {
                ide_chat_register_sidebar_conversation(
                    state,
                    conversation_id,
                    &sidebar_label,
                    opened_conversation_id,
                )?;
            }
            Ok(result)
        })(),
        "conversation.createOptions" => ide_chat_create_conversation_options(state),
        "conversation.delete" => ide_chat_delete_conversation(state, request.params),
        "conversation.batchArchive" => ide_chat_batch_archive_conversations(state, request.params).await,
        "conversation.rebindRecipient" => ide_chat_rebind_conversation_recipient(state, request.params),
        "conversation.rewindPreview" => ide_chat_rewind_preview(state, request.params).await,
        "conversation.rewind" => ide_chat_rewind_conversation(state, request.params).await,
        "conversation.branchFromMessage" => ide_chat_branch_conversation_from_message(state, request.params).await,
        "conversation.branchFromSelection" => ide_chat_branch_conversation(state, request.params).await,
        "list_unarchived_conversations" => ide_chat_list_unarchived_conversations_for_web_settings(state).await,
        "remote_im_list_contact_conversations" => {
            ide_chat_remote_im_list_contact_conversations_for_web_settings(state)
        }
        "list_delegate_conversations" => ide_chat_list_delegate_conversations_for_web_settings(state),
        "get_prompt_preview" => ide_chat_get_prompt_preview_for_web_settings(state, request.params).await,
        "get_system_prompt_preview" => ide_chat_get_system_prompt_preview_for_web_settings(state, request.params).await,
        "get_conversation_section_orders" => (|| -> Result<Value, String> {
            let runtime = state_read_runtime_state_cached(state)?;
            ide_chat_serialize(ConversationSectionOrdersOutput {
                local: runtime.conversation_section_orders.local,
                contact: runtime.conversation_section_orders.contact,
            })
        })(),
        "save_conversation_section_order" => (|| -> Result<Value, String> {
            let input =
                ide_chat_parse_param_field::<SaveConversationSectionOrderInput>(request.params, "input")?;
            let tab = normalize_conversation_section_order_tab(&input.tab)?;
            let ordered_keys = normalize_conversation_section_order_keys(&input.ordered_keys);
            let mut runtime = state_read_runtime_state_cached(state)?;
            match tab {
                "local" => runtime.conversation_section_orders.local = ordered_keys.clone(),
                "contact" => runtime.conversation_section_orders.contact = ordered_keys.clone(),
                _ => {}
            }
            state_write_runtime_state_cached(state, &runtime)?;
            runtime_log_info(format!(
                "[会话分组排序] 完成，任务=保存会话分组顺序，tab={}，group_count={}",
                tab,
                ordered_keys.len()
            ));
            ide_chat_serialize(SaveConversationSectionOrderOutput {
                tab: tab.to_string(),
                ordered_keys,
            })
        })(),
        "delegate.statuses" => ide_chat_delegate_statuses(state, request.params),
        "delegate.abort" => ide_chat_delegate_abort(state, request.params),
        "delegate.blockPage" => ide_chat_delegate_block_page(state, request.params),
        "delegate.submit" => ide_chat_submit_delegate(state, request.params).await,
        "task.list" => ide_chat_task_list(state),
        "task.create" => ide_chat_task_create(state, request.params),
        "task.update" => ide_chat_task_update(state, request.params),
        "task.delete" => ide_chat_task_delete(state, request.params),
        "task.optimizeDraft" => ide_chat_task_optimize_draft(state, request.params).await,
        "task.dispatchNow" => ide_chat_task_dispatch_now(state, request.params).await,
        "goal.current" => ide_chat_goal_current(state, request.params),
        "goal.create" => ide_chat_goal_create(state, request.params),
        "goal.cancel" => ide_chat_goal_cancel(state, request.params),
        "conversation.compactPreview" => ide_chat_compact_preview(state, request.params),
        "conversation.compact" => ide_chat_compact_conversation(state, request.params).await,
        "model.list" => ide_chat_model_list(state, request.params),
        "model.select" => ide_chat_select_model(state, app, request.params),
        "workspace.permission" => ide_chat_workspace_permission(state, request.params),
        "workspace.permission.select" => ide_chat_select_workspace_permission(state, request.params),
        "workspace.list" => ide_chat_workspace_list(state, request.params),
        "workspace.directory.list" => ide_chat_workspace_directory_list(request.params),
        "fileReader.directory.list" => ide_chat_file_reader_directory_list(request.params),
        "fileReader.readFile" => ide_chat_file_reader_read(request.params),
        "fileReader.readFileBlock" => ide_chat_file_reader_read_block(request.params),
        "read_chat_image_data_url" => (|| {
            let input = ide_chat_parse_param_field::<ChatImageDataUrlInput>(request.params, "input")?;
            let media_ref = input.media_ref.trim();
            if media_ref.is_empty() {
                return ide_chat_serialize(ChatImageDataUrlOutput {
                    data_url: String::new(),
                });
            }
            if stored_binary_ref_from_marker(media_ref).is_none() {
                return Err("Chat image mediaRef is invalid.".to_string());
            }
            let mime = input.mime.trim().to_ascii_lowercase();
            if !mime.starts_with("image/") {
                return Err("Chat image mime is invalid.".to_string());
            }
            let base64 = resolve_stored_binary_base64(&state.data_path, media_ref)?;
            ide_chat_serialize(ChatImageDataUrlOutput {
                data_url: format!("data:{mime};base64,{base64}"),
            })
        })(),
        "ideContext.query" => ide_chat_parse_params::<IdeContextWorkspaceQueryInput>(request.params)
            .and_then(|input| serde_json::to_value(query_ide_context_references_internal(input, ide_context_runtime)?)
                .map_err(|err| format!("serialize IDE context query result failed: {err}"))),
        "workspace.layout.save" => ide_chat_workspace_layout_save(state, request.params),
        "terminalApproval.resolve" => ide_chat_resolve_terminal_approval(state, request.params),
        "terminalApproval.approveForSession" => {
            ide_chat_approve_terminal_approval_for_session(state, request.params)
        }
        "terminalApproval.approveForWorkspace" => {
            ide_chat_approve_terminal_approval_for_workspace(state, request.params)
        }
        "conversation.planMode.set" => ide_chat_set_conversation_plan_mode(state, request.params),
        "conversation.plan.confirm" => ide_chat_confirm_plan(state, request.params).await,
        "conversation.plan.readFile" => ide_chat_read_plan_file(state, request.params),
        "settings.open" => ide_chat_open_settings(app),
        "is_backend_ready" => Ok(serde_json::json!(state.backend_ready.load(std::sync::atomic::Ordering::Acquire))),
        "load_config" => ide_chat_load_config_for_web_settings(state),
        "load_app_bootstrap_snapshot" => ide_chat_load_app_bootstrap_snapshot_for_web_settings(state),
        "save_config" => ide_chat_save_config_for_web_settings(state, app, ide_context_runtime, request.params),
        "load_agents" => ide_chat_load_agents_for_web_settings(state),
        "save_agents" => ide_chat_save_agents_for_web_settings(state, app, request.params),
        "load_chat_settings" => ide_chat_load_chat_settings_for_web_settings(state),
        "save_chat_settings" => ide_chat_save_chat_settings_for_web_settings(state, app, request.params),
        "patch_chat_settings" => ide_chat_patch_chat_settings_for_web_settings(state, app, request.params),
        "save_conversation_api_settings" => ide_chat_save_conversation_api_settings_for_web_settings(state, app, request.params),
        "patch_conversation_api_settings" => ide_chat_patch_conversation_api_settings_for_web_settings(state, app, request.params),
        "read_avatar_data_url" => ide_chat_avatar_data_url_for_web_settings(state, request.params),
        "save_agent_avatar" => ide_chat_save_agent_avatar_for_web_settings(state, request.params),
        "clear_agent_avatar" => ide_chat_clear_agent_avatar_for_web_settings(state, request.params),
        "sync_tray_icon" => ide_chat_sync_tray_icon_for_web_settings(app),
        "refresh_models" => ide_chat_refresh_models_for_web_settings(state, request.params).await,
        "quick_genai_chat" => ide_chat_quick_genai_chat_for_web_settings(state, request.params).await,
        "fetch_model_metadata" => ide_chat_fetch_model_metadata_for_web_settings(state, request.params).await,
        "resolve_model_adapter_kind" => ide_chat_resolve_model_adapter_kind_for_web_settings(request.params),
        "test_embedding_connection" => ide_chat_test_embedding_connection_for_web_settings(request.params).await,
        "test_rerank_connection" => ide_chat_test_rerank_connection_for_web_settings(request.params).await,
        "test_voice_connection" => ide_chat_test_voice_connection_for_web_settings(request.params).await,
        "test_memory_embedding_provider" => ide_chat_test_memory_embedding_provider_for_web_settings(state, request.params),
        "test_memory_rerank_provider" => ide_chat_test_memory_rerank_provider_for_web_settings(state, request.params),
        "check_tools_status" => ide_chat_check_tools_status_for_web_settings(state, request.params),
        "get_image_text_cache_stats" => ide_chat_get_image_text_cache_stats_for_web_settings(state),
        "clear_image_text_cache" => ide_chat_clear_image_text_cache_for_web_settings(state),
        "list_tool_catalog" => ide_chat_list_tool_catalog_for_web_settings(state).await,
        "list_department_permission_catalog" => ide_chat_list_department_permission_catalog_for_web_settings(state).await,
        "get_app_version" => Ok(serde_json::json!(env!("CARGO_PKG_VERSION").to_string())),
        "get_project_repository_url" => Ok(serde_json::json!(GITHUB_REPO_PAGE.to_string())),
        "fetch_project_changelog_markdown" => fetch_project_changelog_markdown().await.and_then(ide_chat_serialize),
        "get_web_access_info" => ide_chat_web_access_info_for_web_settings(app, state, ide_context_runtime).await,
        "open_external_url" => ide_chat_open_external_url_for_web_settings(request.params),
        "read_local_chat_image_thumbnail" => ide_chat_read_local_chat_image_thumbnail_for_web_settings(request.params),
        "read_local_chat_image_original" => ide_chat_read_local_chat_image_original_for_web_settings(request.params),
        "show_main_window" => ide_chat_show_window_for_web_settings(app, "main"),
        "show_chat_window" => ide_chat_show_window_for_web_settings(app, "chat"),
        "show_archives_window" => ide_chat_show_window_for_web_settings(app, "archives"),
        "show_quick_setup_window" => ide_chat_show_window_for_web_settings(app, "quick-setup"),
        "complete_quick_setup_and_open_chat" => (|| {
            complete_quick_setup_and_open_chat(app.clone())?;
            Ok(serde_json::json!(null))
        })(),
        "open_runtime_logs_window" => ide_chat_open_runtime_logs_window_for_web_settings(app),
        "list_recent_runtime_logs" => list_recent_runtime_logs().and_then(ide_chat_serialize),
        "clear_recent_runtime_logs" => clear_recent_runtime_logs().and_then(ide_chat_serialize),
        "demo_send_native_notification" => demo_send_native_notification(app.clone()).and_then(ide_chat_serialize),
        "demo_restart_app" => (|| {
            demo_restart_app(app.clone())?;
            Ok(serde_json::json!(null))
        })(),
        "set_webview_zoom_percent" => ide_chat_set_webview_zoom_percent_for_web_settings(app, request.params),
        "set_github_update_method" => ide_chat_set_github_update_method_for_web_settings(state, app, request.params),
        "set_skipped_github_update_version" => {
            ide_chat_set_skipped_github_update_version_for_web_settings(state, app, request.params)
        },
        "get_github_update_state" => ide_chat_get_github_update_state_for_web_settings(app),
        "check_github_update" => ide_chat_check_github_update_for_web_settings(app, request.params).await,
        "start_github_update" => ide_chat_start_github_update_for_web_settings(app, request.params).await,
        "cancel_github_update" => ide_chat_cancel_github_update_for_web_settings().await,
        "apply_prepared_github_update" => ide_chat_apply_prepared_github_update_for_web_settings(app).await,
        "codex_get_auth_status" => ide_chat_codex_get_auth_status_for_web_settings(request.params).await,
        "codex_start_oauth_login" => ide_chat_codex_start_oauth_login_for_web_settings(request.params).await,
        "codex_get_rate_limits" => ide_chat_codex_get_rate_limits_for_web_settings(request.params).await,
        "codex_consume_rate_limit_reset_credit" => ide_chat_codex_consume_rate_limit_reset_credit_for_web_settings(request.params).await,
        "codex_logout" => ide_chat_codex_logout_for_web_settings(request.params),
        "list_memories" => ide_chat_list_memories_for_web_settings(state),
        "delete_memory" => ide_chat_delete_memory_for_web_settings(state, request.params),
        "search_memories_mixed" => ide_chat_search_memories_mixed_for_web_settings(state, request.params),
        "search_chat_history_slices" => ide_chat_search_chat_history_slices_for_web_settings(state, request.params),
        "get_memory_provider_bindings" => ide_chat_get_memory_provider_bindings_for_web_settings(state),
        "get_memory_embedding_sync_progress" => ide_chat_get_memory_embedding_sync_progress_for_web_settings(state),
        "save_memory_embedding_binding" => ide_chat_save_memory_embedding_binding_for_web_settings(state, request.params),
        "save_memory_rerank_binding" => ide_chat_save_memory_rerank_binding_for_web_settings(state, request.params),
        "get_agent_private_memory_count" => ide_chat_get_agent_private_memory_count_for_web_settings(state, request.params),
        "set_agent_memory_recall_mode" => ide_chat_set_agent_memory_recall_mode_for_web_settings(state, request.params),
        "set_agent_private_memory_enabled" => ide_chat_set_agent_private_memory_enabled_for_web_settings(state, request.params),
        "export_agent_private_memories" => ide_chat_export_agent_private_memories_for_web_settings(state, request.params),
        "disable_agent_private_memory" => ide_chat_disable_agent_private_memory_for_web_settings(state, request.params),
        "export_memories" => ide_chat_export_memories_for_web_settings(state, request.params),
        "preview_export_memories" => ide_chat_preview_export_memories_for_web_settings(state),
        "export_memories_to_path" => ide_chat_export_memories_to_path_for_web_settings(state, request.params),
        "import_memories" => ide_chat_import_memories_for_web_settings(state, request.params),
        "preview_import_angel_memories" => ide_chat_preview_import_angel_memories_for_web_settings(request.params),
        "import_angel_memories" => ide_chat_import_angel_memories_for_web_settings(state, request.params),
        "task_list_tasks" => ide_chat_task_list_tasks_for_web_settings(state),
        "task_get_task" => ide_chat_task_get_task_for_web_settings(state, request.params),
        "task_create_task" => ide_chat_task_create_task_for_web_settings(state, request.params),
        "task_update_task" => ide_chat_task_update_task_for_web_settings(state, request.params),
        "task_complete_task" => ide_chat_task_complete_task_for_web_settings(state, request.params),
        "task_delete_task" => ide_chat_task_delete_task_for_web_settings(state, request.params),
        "task_list_run_logs" => ide_chat_task_list_run_logs_for_web_settings(state, request.params),
        "task_optimize_draft" => ide_chat_task_optimize_draft_for_web_settings(state, request.params).await,
        "mcp_list_servers" => ide_chat_mcp_list_servers_for_web_settings(state),
        "mcp_validate_definition" => ide_chat_mcp_validate_definition_for_web_settings(request.params),
        "mcp_save_server" => ide_chat_mcp_save_server_for_web_settings(state, request.params),
        "mcp_remove_server" => ide_chat_mcp_remove_server_for_web_settings(state, request.params).await,
        "mcp_list_server_tools" => ide_chat_mcp_list_server_tools_for_web_settings(state, request.params).await,
        "mcp_list_server_tools_cached" => ide_chat_mcp_list_server_tools_cached_for_web_settings(state, request.params),
        "mcp_deploy_server" => ide_chat_mcp_deploy_server_for_web_settings(state, request.params),
        "mcp_undeploy_server" => ide_chat_mcp_undeploy_server_for_web_settings(state, request.params).await,
        "mcp_set_tool_enabled" => ide_chat_mcp_set_tool_enabled_for_web_settings(state, request.params),
        "mcp_open_workspace_dir" => ide_chat_mcp_open_workspace_dir_for_web_settings(state),
        "mcp_list_skills" => ide_chat_mcp_list_skills_for_web_settings(state),
        "mcp_refresh_mcp_and_skills" => ide_chat_mcp_refresh_mcp_and_skills_for_web_settings(state).await,
        "skill_open_workspace_dir" => ide_chat_skill_open_workspace_dir_for_web_settings(state),
        "get_storage_usage_overview" => ide_chat_get_storage_usage_overview_for_web_settings(state).await,
        "refresh_storage_usage_overview" => {
            ide_chat_refresh_storage_usage_overview_for_web_settings(state).await
        }
        "get_usage_overview" => ide_chat_get_usage_overview_for_web_settings(state).await,
        "refresh_usage_overview" => ide_chat_refresh_usage_overview_for_web_settings(state).await,
        "open_storage_usage_item_directory" => ide_chat_open_storage_usage_item_directory_for_web_settings(state, request.params),
        "cleanup_storage_legacy_items" => ide_chat_cleanup_storage_legacy_items_for_web_settings(state, request.params),
        "export_config_migration_package" => ide_chat_export_config_migration_package_for_web_settings(state, request.params),
        "preview_import_config_migration_package" => ide_chat_preview_import_config_migration_package_for_web_settings(state, request.params),
        "apply_import_config_migration_package" => ide_chat_apply_import_config_migration_package_for_web_settings(state, app, request.params),
        "list_recent_llm_round_logs" => ide_chat_list_recent_llm_round_logs_for_web_settings(state),
        "get_recent_llm_round_log_section" => ide_chat_get_recent_llm_round_log_section_for_web_settings(state, request.params),
        "clear_recent_llm_round_logs" => ide_chat_clear_recent_llm_round_logs_for_web_settings(state),
        "list_terminal_shell_candidates" => ide_chat_list_terminal_shell_candidates_for_web_settings(state),
        "open_chat_shell_workspace_dir" => ide_chat_open_chat_shell_workspace_dir_for_web_settings(state, request.params),
        "reset_chat_shell_workspace" => ide_chat_reset_chat_shell_workspace_for_web_settings(state, request.params),
        "get_default_chat_shell_workspace_path" => ide_chat_get_default_chat_shell_workspace_path_for_web_settings(state),
        "migrate_shell_workspace_directory" => ide_chat_migrate_shell_workspace_directory_for_web_settings(app, request.params).await,
        "get_host_runtime_prerequisites" => ide_chat_get_host_runtime_prerequisites_for_web_settings(),
        "install_host_runtime_prerequisite" => ide_chat_install_host_runtime_prerequisite_for_web_settings(request.params).await,
        "remote_im_get_channel_status" => ide_chat_remote_im_get_channel_status_for_web_settings(state, request.params).await,
        "remote_im_restart_channel" => ide_chat_remote_im_restart_channel_for_web_settings(state, request.params).await,
        "remote_im_get_channel_logs" => ide_chat_remote_im_get_channel_logs_for_web_settings(state, request.params).await,
        "remote_im_get_contact_logs" => ide_chat_remote_im_get_contact_logs_for_web_settings(state, request.params).await,
        "remote_im_list_channels" => ide_chat_remote_im_list_channels_for_web_settings(state),
        "remote_im_list_contacts" => ide_chat_remote_im_list_contacts_for_web_settings(state),
        "remote_im_update_contact_allow_send" => ide_chat_remote_im_update_contact_allow_send_for_web_settings(state, request.params),
        "remote_im_update_contact_allow_send_files" => ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(state, request.params),
        "remote_im_update_contact_activation" => ide_chat_remote_im_update_contact_activation_for_web_settings(state, request.params),
        "remote_im_update_contact_department_binding" => ide_chat_remote_im_update_contact_department_binding_for_web_settings(state, request.params),
        "remote_im_update_contact_processing_mode" => ide_chat_remote_im_update_contact_processing_mode_for_web_settings(state, request.params),
        "remote_im_update_contact_workspace" => ide_chat_remote_im_update_contact_workspace_for_web_settings(state, request.params),
        "remote_im_delete_contact" => ide_chat_remote_im_delete_contact_for_web_settings(state, request.params),
        "remote_im_weixin_oc_start_login" => ide_chat_remote_im_weixin_oc_start_login_for_web_settings(state, request.params).await,
        "remote_im_weixin_oc_get_login_status" => ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(state, request.params).await,
        "remote_im_weixin_oc_sync_contacts" => ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(state, request.params).await,
        "remote_im_weixin_oc_logout" => ide_chat_remote_im_weixin_oc_logout_for_web_settings(state, request.params).await,
        "chat.queueAttachment" => ide_chat_queue_attachment(state, request.params),
        "chat.send" => ide_chat_send_message(state, request.params),
        "chat.stop" => ide_chat_stop_conversation(state, request.params),
        "chat.queueSnapshot" => ide_chat_queue_snapshot(state),
        "chat.sessionStateSnapshot" => ide_chat_session_state_snapshot(state),
        "chat.queueRecall" => ide_chat_recall_queue_event(state, request.params),
        "chat.queueMarkGuided" => ide_chat_mark_queue_event_guided(state, request.params),
        "toolReview.reports.list" => ide_chat_tool_review_reports(state, request.params),
        "toolReview.report.delete" => ide_chat_tool_review_delete_report(state, request.params),
        "toolReview.commitOptions.list" => ide_chat_tool_review_commit_options(state, request.params).await,
        "toolReview.code.submit" => ide_chat_tool_review_submit_code(state, request.params).await,
        "toolReview.batches.list" => ide_chat_tool_review_batches(state, request.params),
        "toolReview.item.detail" => ide_chat_tool_review_item_detail(state, request.params),
        "toolReview.item.review" => ide_chat_tool_review_item_review(state, request.params).await,
        "toolReview.batch.review" => ide_chat_tool_review_batch_review(state, request.params).await,
        "toolReview.item.decision" => ide_chat_tool_review_item_decision(state, request.params),
        _ => return ide_chat_jsonrpc_error(request.id, -32601, "method not found"),
    };
    match result {
        Ok(value) => ide_chat_jsonrpc_success(request.id, value),
        Err(err) => ide_chat_jsonrpc_error(request.id, -32000, err),
    }
}
