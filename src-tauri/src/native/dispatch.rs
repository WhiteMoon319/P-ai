use pai_android_bridge::{DefaultTaskManager, DispatchResult, NativeDispatcher, TaskManager, TaskState};

/// 原生 dispatch 包装器，实现 pai_android_bridge::NativeDispatcher trait。
struct NativeDispatcherImpl(Arc<NativeRuntime>);

impl NativeDispatcher for NativeDispatcherImpl {
    fn dispatch(&self, method: &str, params: Value, id: Option<Value>) -> DispatchResult {
        let runtime = self.0.clone();
        let runtime_for_task = runtime.clone();
        let method = method.to_string();
        let join = runtime.runtime.spawn(async move {
            let state = &runtime_for_task.state;
            let native_app = NativeAppHandle::noop();
            let ide_context_runtime = &runtime_for_task.ide_context_runtime;
            let viewer_id = "android-native";
            let mut opened_conversation_id: Option<String> = None;

            match method.as_str() {
                "bridge.ping" => Ok(serde_json::json!({
                    "ok": true,
                    "ts": chrono::Utc::now().to_rfc3339(),
                })),
                "webview.ping" => Ok(serde_json::json!(true)),
                "webview_pong" => Ok(serde_json::json!(true)),
                "conversation.list" => ide_chat_conversation_list(state, viewer_id),
                "conversation.setActive" => ide_chat_set_active_conversation_command(state, params),
                "conversation.resumeSubscription" => ide_chat_resume_sidebar_subscription(
                    state,
                    params,
                    viewer_id,
                    &mut opened_conversation_id,
                ),
                "conversation.create" => ide_chat_create_conversation(state, params)
                    .await
                    .and_then(|result| Ok(result)),
                "conversation.createOptions" => ide_chat_create_conversation_options(state),
                "conversation.blockPage" => ide_chat_conversation_block_page(state, params),
                "conversation.messageById" => ide_chat_conversation_message_by_id_command(state, params),
                "conversation.messagesBefore" => ide_chat_conversation_messages_before_command(state, params),
                "conversation.markRead" => ide_chat_mark_conversation_read(state, params),
                "conversation.setPreferredModel" => ide_chat_set_preferred_model_command(state, params),
                "model.list" => ide_chat_model_list(state, params),
                "get_prompt_preview" => ide_chat_get_prompt_preview_for_web_settings(state, params).await,
                "prompt.systemPreview" => ide_chat_get_system_prompt_preview_for_web_settings(state, params).await,
                "conversation.rewind" => ide_chat_rewind_conversation_command(state, params).await,
                "conversation.rewindPreview" => ide_chat_preview_rewind_conversation(state, params).await,
                "conversation.compact" => ide_chat_compact_conversation(state, params).await,
                "conversation.compactPreview" => ide_chat_compact_preview(state, params),
                "conversation.autoPush" => ide_chat_set_auto_push_command(state, params),
                "read_chat_image_data_url" => {
                    let input = ide_chat_parse_param_field::<ChatImageDataUrlInput>(params, "input")?;
                    ide_chat_serialize(read_chat_image_data_url_inner(input, state)?)
                }
                "read_avatar_data_url" => {
                    let input = ide_chat_parse_param_field::<AvatarDataPathInput>(params, "input")?;
                    ide_chat_serialize(read_avatar_data_url_inner(input, state)?)
                }
                // ---- 长期目标（goal）----
                "goal.current" => ide_chat_goal_current(state, params),
                "goal.create" => ide_chat_goal_create(state, params),
                "goal.cancel" => ide_chat_goal_cancel(state, params),
                // ---- 委托任务（delegate）----
                "delegate.conversations.list" => ide_chat_list_delegate_conversations_for_web_settings(state),
                "delegate.statuses" => ide_chat_delegate_statuses_command(state, params),
                "delegate.abort" => ide_chat_delegate_abort_command(state, params),
                "delegate.blockPage" => ide_chat_delegate_block_page_command(state, params),
                "delegate.submit" => ide_chat_submit_delegate_command(state, params).await,
                "delegate.delete" => ide_chat_delete_delegate_command(state, params),
                "conversation.rename" => ide_chat_rename_conversation_command(state, params),
                "conversation.pin" => ide_chat_toggle_pin_command(state, params),
                "conversation.delete" => ide_chat_delete_conversation(state, params).await,
                "conversation.batchArchive" => ide_chat_batch_archive_conversations(state, params).await,
                "conversation.exportShare" => ide_chat_export_conversation_share_command(state, params),
                // ---- 归档会话管理（对齐 Vue ArchivesWindow）----
                "archives.list" => ide_chat_list_archives_command(state),
                "archives.blockPage" => ide_chat_archive_block_page_command(state, params),
                "archives.summary" => ide_chat_archive_summary_command(state, params),
                "archives.delete" => ide_chat_delete_archive_command(state, params),
                "archives.unarchive" => ide_chat_unarchive_command(state, params),
                "conversation.runtimeSnapshot" => ide_chat_conversation_runtime_snapshot(state, params),
                "conversation.fastRequestTurns" => ide_chat_conversation_fast_request_turns(state, params),
                "conversation.freshnessSnapshot" => ide_chat_conversation_freshness_snapshot(state, params).await,
                "chat.send" => ide_chat_send_message(state, params).await,
                "chat.stop" => ide_chat_stop_conversation(state, params),
                "load_config" => ide_chat_load_config_for_web_settings(state),
                "check_message_store_migration" | "messageStore.migration.check" => {
                    check_message_store_migration_inner(state).and_then(ide_chat_serialize)
                }
                "run_message_store_migration" | "messageStore.migration.run" => {
                    let input = ide_chat_parse_workspace_params::<RunMessageStoreMigrationInput>(params)?;
                    run_message_store_migration_inner(&native_app, state, input).and_then(ide_chat_serialize)
                }
                "load_chat_settings" => ide_chat_load_chat_settings_for_web_settings(state),
                "save_config" => ide_chat_save_config_for_web_settings(state, &native_app, ide_context_runtime, params),
                "patch_config" => ide_chat_patch_config_for_web_settings(state, &native_app, ide_context_runtime, params),
                "load_agents" => ide_chat_load_agents_for_web_settings(state),
                "save_agents" => ide_chat_save_agents_for_web_settings(state, &native_app, params),
                "save_chat_settings" => ide_chat_save_chat_settings_for_web_settings(state, &native_app, params),
                "patch_chat_settings" => ide_chat_patch_chat_settings_for_web_settings(state, &native_app, params),
                "save_conversation_api_settings" => ide_chat_save_conversation_api_settings_for_web_settings(state, &native_app, params),
                "patch_conversation_api_settings" => ide_chat_patch_conversation_api_settings_for_web_settings(state, &native_app, params),
                "set_ui_language" => ide_chat_set_ui_language_command(state, &native_app, params),
                "app.language.set" => ide_chat_set_ui_language_command(state, &native_app, params),
                "set_department_primary_api_config" => ide_chat_set_department_primary_api_command(state, &native_app, params),
                "department.primaryApi.set" => ide_chat_set_department_primary_api_command(state, &native_app, params),
                "set_github_update_method" => ide_chat_set_github_update_method_for_web_settings(state, &native_app, params),
                "set_skipped_github_update_version" => ide_chat_set_skipped_github_update_version_for_web_settings(state, &native_app, params),
                "convert_private_agent_to_main" => ide_chat_convert_private_agent_to_main_for_web_settings(state, &native_app, params),
                "set_agent_private_memory_enabled" => ide_chat_set_agent_private_memory_enabled_for_web_settings(state, params),
                "set_agent_memory_recall_mode" => ide_chat_set_agent_memory_recall_mode_for_web_settings(state, params),
                "check_github_update" => ide_chat_serialize(check_github_update_android(
                    &native_app,
                    params.get("updateMethod").and_then(Value::as_str).map(str::to_string),
                    params.get("respectCooldown").and_then(Value::as_bool),
                ).await?),
                "test_text_connection" => ide_chat_serialize(test_text_connection_inner(
                    ide_chat_parse_param_field::<ApiConfig>(params, "input")?, state).await?),
                "api_config.create" => ide_chat_serialize(api_config_create_inner(
                    ide_chat_parse_param_field::<ApiConfig>(params, "input")?, &native_app, state, ide_context_runtime)?),
                "api_config.update" => ide_chat_serialize(api_config_update_inner(
                    ide_chat_parse_param_field::<ApiConfig>(params, "input")?, &native_app, state, ide_context_runtime)?),
                "api_config.delete" => ide_chat_serialize(api_config_delete_inner(
                    ide_chat_parse_param_field::<ApiConfigDeleteInput>(params, "input")?, &native_app, state, ide_context_runtime)?),
                "check_tools_status" => ide_chat_check_tools_status_for_web_settings(state, params),
                "stt_transcribe" => ide_chat_stt_transcribe_for_web_settings(state, params).await,
                "attachment.ingestLocalPath" => ide_chat_serialize(attachment_ingest_local_path_inner(
                    ide_chat_parse_param_field::<AttachmentIngestLocalPathInput>(params, "input")?, state).await?),
                "get_web_access_info" => ide_chat_web_access_info_for_web_settings(&native_app, state, ide_context_runtime).await,
                "transport.accessInfo" => ide_chat_web_access_info_for_web_settings(&native_app, state, ide_context_runtime).await,
                "app.bootstrapSnapshot" => ide_chat_load_app_bootstrap_snapshot_for_web_settings(state),
                // ---- Vue 设置页对齐：存储/用量/日志 ----
                "get_storage_usage_overview" => ide_chat_get_storage_usage_overview_for_web_settings(state).await,
                "refresh_storage_usage_overview" => ide_chat_refresh_storage_usage_overview_for_web_settings(state).await,
                "cleanup_storage_legacy_items" => ide_chat_cleanup_storage_legacy_items_for_web_settings(state, params),
                "get_usage_overview" => ide_chat_get_usage_overview_for_web_settings(state).await,
                "list_recent_runtime_logs" => list_recent_runtime_logs().and_then(ide_chat_serialize),
                "list_runtime_logs_since" => {
                    let since = params.get("sinceCreatedAt").and_then(Value::as_str).map(str::to_string);
                    list_runtime_logs_since(since).and_then(ide_chat_serialize)
                }
                "clear_recent_runtime_logs" => clear_recent_runtime_logs().and_then(ide_chat_serialize),
                "append_runtime_log_probe" => append_runtime_log_probe(
                    params.get("message").and_then(Value::as_str).map(str::to_string),
                ).and_then(ide_chat_serialize),
                // ---- LLM 轮次日志（诊断）----
                "list_recent_llm_round_logs" => ide_chat_list_recent_llm_round_logs_for_web_settings(state),
                "get_recent_llm_round_log_section" => ide_chat_get_recent_llm_round_log_section_for_web_settings(state, params),
                "clear_recent_llm_round_logs" => ide_chat_clear_recent_llm_round_logs_for_web_settings(state),
                // ---- Vue 设置页对齐：记忆 ----
                "list_memories" => ide_chat_list_memories_for_web_settings(state),
                "delete_memory" => ide_chat_delete_memory_for_web_settings(state, params),
                "search_memories_mixed" => ide_chat_search_memories_mixed_for_web_settings(state, params),
                "search_memories_recall" => ide_chat_search_memories_recall_command(state, params),
                "get_memory_provider_bindings" => ide_chat_get_memory_provider_bindings_for_web_settings(state),
                "get_memory_embedding_sync_progress" => ide_chat_get_memory_embedding_sync_progress_for_web_settings(state),
                "save_memory_embedding_binding" => ide_chat_save_memory_embedding_binding_for_web_settings(state, params),
                "save_memory_rerank_binding" => ide_chat_save_memory_rerank_binding_for_web_settings(state, params),
                "get_agent_private_memory_count" => ide_chat_get_agent_private_memory_count_for_web_settings(state, params),
                // ---- Vue 设置页对齐：MCP / 技能 ----
                "mcp_list_servers" => ide_chat_mcp_list_servers_for_web_settings(state),
                "mcp_validate_definition" => ide_chat_mcp_validate_definition_for_web_settings(params),
                "mcp_fix_definition" => ide_chat_mcp_fix_definition_for_web_settings(state, params).await,
                "mcp_save_server" => ide_chat_mcp_save_server_for_web_settings(state, params),
                "mcp_remove_server" => ide_chat_mcp_remove_server_for_web_settings(state, params).await,
                "mcp_list_server_tools" => ide_chat_mcp_list_server_tools_for_web_settings(state, params).await,
                "mcp_list_server_tools_cached" => ide_chat_mcp_list_server_tools_cached_for_web_settings(state, params),
                "mcp_deploy_server" => ide_chat_mcp_deploy_server_for_web_settings(state, params).await,
                "mcp_undeploy_server" => ide_chat_mcp_undeploy_server_for_web_settings(state, params).await,
                "mcp_list_skills" => ide_chat_mcp_list_skills_for_web_settings(state),
                // ---- Vue 设置页对齐：任务 ----
                "task_list_tasks" => ide_chat_task_list_tasks_for_web_settings(state),
                "task_get_task" => ide_chat_task_get_task_for_web_settings(state, params),
                "task_create_task" => ide_chat_task_create_task_for_web_settings(state, params),
                "task_update_task" => ide_chat_task_update_task_for_web_settings(state, params),
                "task_complete_task" => ide_chat_task_complete_task_for_web_settings(state, params),
                "task_delete_task" => ide_chat_task_delete_task_for_web_settings(state, params),
                "task_list_run_logs" => ide_chat_task_list_run_logs_for_web_settings(state, params),
                "task_optimize_draft" => ide_chat_task_optimize_draft_for_web_settings(state, params).await,
                // ---- Vue 设置页对齐：远程 IM ----
                "remote_im_list_channels" => ide_chat_remote_im_list_channels_for_web_settings(state),
                "remote_im_list_contacts" => ide_chat_remote_im_list_contacts_for_web_settings(state),
                "remote_im_get_channel_status" => ide_chat_remote_im_get_channel_status_for_web_settings(state, params).await,
                "remote_im_restart_channel" => ide_chat_remote_im_restart_channel_for_web_settings(state, params).await,
                "remote_im_get_channel_logs" => ide_chat_remote_im_get_channel_logs_for_web_settings(state, params).await,
                "remote_im_get_contact_logs" => ide_chat_remote_im_get_contact_logs_for_web_settings(state, params).await,
                "remote_im_update_contact_allow_send" => ide_chat_remote_im_update_contact_allow_send_for_web_settings(state, params),
                "remote_im_update_contact_allow_send_files" => ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(state, params),
                "remote_im_update_contact_blocked_message_prefixes" => ide_chat_remote_im_update_contact_blocked_message_prefixes_for_web_settings(state, params),
                "remote_im_update_contact_activation" => ide_chat_remote_im_update_contact_activation_for_web_settings(state, params),
                "remote_im_update_contact_department_binding" => ide_chat_remote_im_update_contact_department_binding_for_web_settings(state, params),
                "remote_im_update_contact_processing_mode" => ide_chat_remote_im_update_contact_processing_mode_for_web_settings(state, params),
                "remote_im_update_contact_workspace" => ide_chat_remote_im_update_contact_workspace_for_web_settings(state, params),
                "remote_im_delete_contact" => ide_chat_remote_im_delete_contact_for_web_settings(state, params),
                "remote_im_weixin_oc_start_login" => ide_chat_remote_im_weixin_oc_start_login_for_web_settings(state, params).await,
                "remote_im_weixin_oc_get_login_status" => ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(state, params).await,
                "remote_im_weixin_oc_logout" => ide_chat_remote_im_weixin_oc_logout_for_web_settings(state, params).await,
                "remote_im_weixin_oc_sync_contacts" => ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(state, params).await,
                "frontend_ready_start_remote_im_services" => {
                    ide_chat_start_remote_im_services_for_web_settings(state).await
                }
                "remoteIm.services.start" => {
                    ide_chat_start_remote_im_services_for_web_settings(state).await
                }
                "get_android_workspace_status" => ide_chat_serialize(get_android_workspace_status_ws_inner(state)?),
                "init_android_workspace" => ide_chat_serialize(init_android_workspace_ws_inner(state, Some(&native_app)).await?),
                "repair_android_workspace_runtime" => ide_chat_serialize(repair_android_workspace_runtime_ws_inner(state, Some(&native_app))?),
                "reset_android_workspace_runtime" => ide_chat_serialize(reset_android_workspace_runtime_ws_inner(
                    state, Some(&native_app), &android_workspace_root(state))?),
                "reset_android_workspace_state" => ide_chat_serialize(reset_android_workspace_state_ws_inner(state, Some(&native_app))?),
                "import_android_workspace_rootfs_archive" => ide_chat_serialize(import_android_workspace_rootfs_archive_ws_inner(
                    state,
                    Some(&native_app),
                    params.get("fileName").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("dataBase64").and_then(Value::as_str).unwrap_or_default().to_string(),
                ).await?),
                "android_workspace.list" => ide_chat_serialize(list_android_workspace_files_ws_inner(
                    state,
                    params.get("path").and_then(Value::as_str).map(str::to_string),
                )?),
                "android_workspace.readText" => ide_chat_serialize(read_android_workspace_text_ws_inner(
                    state,
                    params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
                )?),
                "android_workspace.writeText" => ide_chat_serialize(write_android_workspace_text_ws_inner(
                    state,
                    params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("text").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("overwrite").and_then(Value::as_bool),
                )?),
                "android_workspace.move" => ide_chat_serialize(move_android_workspace_file_ws_inner(
                    state,
                    params.get("source").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("target").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("overwrite").and_then(Value::as_bool),
                )?),
                "android_workspace.glob" => ide_chat_serialize(glob_android_workspace_files_ws_inner(
                    state,
                    params.get("pattern").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("path").and_then(Value::as_str).map(str::to_string),
                )?),
                "android_workspace.grep" => ide_chat_serialize(grep_android_workspace_files_ws_inner(
                    state,
                    params.get("query").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("path").and_then(Value::as_str).map(str::to_string),
                    params.get("regex").and_then(Value::as_bool),
                    params.get("ignoreCase").and_then(Value::as_bool),
                    params.get("includeGlob").and_then(Value::as_str).map(str::to_string),
                )?),
                "android_workspace.delete" => ide_chat_serialize(delete_file_from_android_workspace_ws_inner(
                    state,
                    params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
                )?),
                "android_workspace.import" => ide_chat_serialize(import_file_to_android_workspace_ws_inner(
                    state,
                    params.get("fileName").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("mime").and_then(Value::as_str).map(str::to_string),
                    params.get("dataBase64").and_then(Value::as_str).unwrap_or_default().to_string(),
                    params.get("targetPath").and_then(Value::as_str).map(str::to_string),
                )?),
                "android_workspace.export" => ide_chat_serialize(export_file_from_android_workspace_ws_inner(
                    state,
                    params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
                )?),
                "get_app_version" => Ok(serde_json::json!(android_current_app_version())),
                "get_project_repository_url" => Ok(serde_json::json!(GITHUB_REPO_PAGE.to_string())),
                "list_terminal_shell_candidates" => ide_chat_list_terminal_shell_candidates_for_web_settings(state),
                "list_tool_catalog" => ide_chat_list_tool_catalog_for_web_settings(state).await,
                "list_department_permission_catalog" => ide_chat_list_department_permission_catalog_for_web_settings(state).await,
                // ---- 原生任务状态机（长任务进度追踪）----
                "task.create" => {
                    let task_id = params.get("taskId")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let handle = DefaultTaskManager.create_task(task_id)?;
                    ide_chat_serialize(&handle)
                }
                "task.update" => {
                    let task_id = params.get("taskId")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let state_str = params.get("state")
                        .and_then(Value::as_str)
                        .unwrap_or("Running");
                    let progress = params.get("progress")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0);
                    let message = params.get("message")
                        .and_then(Value::as_str)
                        .unwrap_or("");
                    let task_state = TaskState::from_str(state_str)?;
                    let handle = DefaultTaskManager.update_task(task_id, task_state, progress, message)?;
                    ide_chat_serialize(&handle)
                }
                "task.get" => {
                    let task_id = params.get("taskId")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let handle = DefaultTaskManager.get_task(task_id)?;
                    ide_chat_serialize(&handle)
                }
                "task.cancel" => {
                    let task_id = params.get("taskId")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let handle = DefaultTaskManager.cancel_task(task_id)?;
                    ide_chat_serialize(&handle)
                }
                _ => Err(format!("原生桥 method not found: {method}")),
            }
        });
        runtime.runtime.block_on(join).unwrap_or_else(|err| {
            Err(format!("原生 dispatch 任务异常: {err}"))
        })
    }
}