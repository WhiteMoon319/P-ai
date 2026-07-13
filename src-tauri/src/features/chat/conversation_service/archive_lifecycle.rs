impl ConversationServiceV2 {
    fn list_archives(
        &self,
        state: &AppState,
    ) -> Result<Vec<ArchiveSummary>, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;

        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let chat_index = state_read_chat_index_cached(state)?;
        let mut summaries = chat_index
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .filter_map(|item| match self.get_conversation_meta(state, item.id.as_str()) {
                Ok(conversation_meta) => Some(conversation_meta),
                Err(err) => {
                    runtime_log_error(format!(
                        "[会话索引读取] 状态=失败，任务=list_archives，conversation_id={}，error={}",
                        item.id, err
                    ));
                    None
                }
            })
            .filter(|archive_meta| archive_meta.status.trim() == "archived")
            .map(|archive_meta| {
                let api_config_id = runtime_department_by_id(
                    &runtime_snapshot,
                    archive_meta.department_id.trim(),
                )
                .or_else(|| {
                    runtime_department_for_agent(&runtime_snapshot, archive_meta.agent_id.as_str())
                })
                .map(department_primary_api_config_id)
                .unwrap_or_default();
                let title = if archive_meta.title.trim().is_empty() {
                    let store_paths =
                        message_store::message_store_paths(&state.data_path, &archive_meta.id).ok();
                    store_paths
                        .and_then(|paths| {
                            message_store::read_ready_message_store_index_summary(&paths)
                                .ok()
                                .flatten()
                        })
                        .and_then(|summary| summary.first_user_text_preview)
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or_else(|| "无内容".to_string())
                } else {
                    archive_meta.title.trim().to_string()
                };
                ArchiveSummary {
                    archive_id: archive_meta.id.to_string(),
                    archived_at: archive_meta
                        .archived_at
                        .clone()
                        .unwrap_or_else(|| archive_meta.updated_at.to_string()),
                    title,
                    message_count: archive_meta.message_count,
                    api_config_id,
                    agent_id: archive_meta.agent_id.to_string(),
                }
            })
            .collect::<Vec<_>>();
        summaries.sort_by(|a, b| b.archived_at.cmp(&a.archived_at));
        drop(guard);
        Ok(summaries)
    }

    fn get_archive_messages(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<Vec<ChatMessage>, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_archive_id)?;
        if let Some(mut messages) =
            message_store::read_ready_message_store_all_messages(&store_paths)?
        {
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            return Ok(messages);
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        ensure_archive_ready_message_store_from_legacy(state, normalized_archive_id, &store_paths)?;
        drop(guard);
        let mut messages = message_store::read_ready_message_store_all_messages(&store_paths)?
            .ok_or_else(|| format!("归档消息仓库不可读，archive_id={normalized_archive_id}"))?;
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(messages)
    }

    fn get_archive_block_page(
        &self,
        state: &AppState,
        archive_id: &str,
        block_id: Option<u32>,
    ) -> Result<ConversationBlockPageResult, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, normalized_archive_id)?;
        if let Some(page) = message_store::read_ready_message_store_block_page(&store_paths, block_id)? {
            let mut messages = page.messages;
            materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
            return Ok(ConversationBlockPageResult {
                blocks: page
                    .blocks
                    .into_iter()
                    .map(|item| ConversationBlockSummaryResult {
                        block_id: item.block_id,
                        message_count: item.message_count,
                        first_message_id: item.first_message_id,
                        last_message_id: item.last_message_id,
                        first_created_at: item.first_created_at,
                        last_created_at: item.last_created_at,
                        is_latest: item.is_latest,
                    })
                    .collect(),
                selected_block_id: page.selected_block_id,
                messages,
                has_prev_block: page.has_prev_block,
                has_next_block: page.has_next_block,
            });
        }

        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        ensure_archive_ready_message_store_from_legacy(state, normalized_archive_id, &store_paths)?;
        drop(guard);
        let page = message_store::read_ready_message_store_block_page(&store_paths, block_id)?
            .ok_or_else(|| format!("归档块分页不可读，archive_id={normalized_archive_id}"))?;
        let mut messages = page.messages;
        materialize_chat_message_parts_from_media_refs(&mut messages, &state.data_path);
        Ok(ConversationBlockPageResult {
            blocks: page
                .blocks
                .into_iter()
                .map(|item| ConversationBlockSummaryResult {
                    block_id: item.block_id,
                    message_count: item.message_count,
                    first_message_id: item.first_message_id,
                    last_message_id: item.last_message_id,
                    first_created_at: item.first_created_at,
                    last_created_at: item.last_created_at,
                    is_latest: item.is_latest,
                })
                .collect(),
            selected_block_id: page.selected_block_id,
            messages,
            has_prev_block: page.has_prev_block,
            has_next_block: page.has_next_block,
        })
    }

    fn get_archive_summary(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<String, String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let summary = self
            .get_conversation_meta(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())
            .and_then(|conversation_meta| {
                if conversation_meta.status.trim() != "archived" {
                    Err("Archive not found".to_string())
                } else {
                    Ok(conversation_meta.summary)
                }
            })?;
        drop(guard);
        Ok(summary)
    }

    fn delete_archive(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<(), String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let conversation_meta = self
            .get_conversation_meta(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())?;
        if conversation_meta.status.trim() != "archived" {
            drop(guard);
            return Err("Archive not found".to_string());
        }
        state_schedule_conversation_delete(state, normalized_archive_id)?;
        drop(guard);
        Ok(())
    }

    fn unarchive_archive(
        &self,
        state: &AppState,
        archive_id: &str,
    ) -> Result<(), String> {
        let normalized_archive_id = archive_id.trim();
        if normalized_archive_id.is_empty() {
            return Err("archiveId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            named_lock_error("conversation_lock", file!(), line!(), module_path!(), &err)
        })?;
        let conversation_meta = self
            .get_conversation_meta(state, normalized_archive_id)
            .map_err(|_| "Archive not found".to_string())?;
        if conversation_meta.status.trim() != "archived"
            || !conversation_meta.visible_in_foreground_lists
            || conversation_meta.conversation_kind.trim() != CONVERSATION_KIND_CHAT
        {
            drop(guard);
            return Err("该归档会话无法恢复为普通会话".to_string());
        }

        let now = now_iso();
        let (conversation, (), _) = state_update_conversation_metadata_cached(
            state,
            normalized_archive_id,
            |conversation| {
                conversation.status = "active".to_string();
                conversation.archived_at = None;
                conversation.updated_at = now.clone();
                Ok(())
            },
        )?;
        runtime_log_info(format!(
            "[归档] 完成，任务=取消归档，conversation_id={}",
            conversation.id
        ));
        drop(guard);
        Ok(())
    }

    fn resolve_archive_target_conversation(
        &self,
        state: &AppState,
        input: &SessionSelector,
    ) -> Result<(ApiConfig, ResolvedApiConfig, Conversation, String), String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let app_config = runtime_snapshot.config;
        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_agents = runtime_snapshot.agents;
        let selected_api = resolve_selected_api_config(&app_config, input.api_config_id.as_deref())
            .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
        let resolved_api = resolve_api_config(&app_config, Some(selected_api.id.as_str()))?;
        let requested_agent_id = input.agent_id.trim();
        let effective_agent_id = if runtime_agents
            .iter()
            .any(|agent| agent.id == requested_agent_id && !agent.is_built_in_user)
        {
            requested_agent_id.to_string()
        } else if runtime_agents.iter().any(|agent| {
            agent.id == runtime.assistant_department_agent_id && !agent.is_built_in_user
        }) {
            runtime.assistant_department_agent_id.clone()
        } else {
            runtime_agents
                .iter()
                .find(|agent| !agent.is_built_in_user)
                .map(|agent| agent.id.clone())
                .ok_or_else(|| "Selected agent not found.".to_string())?
        };
        let source_conversation_id = input
            .conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let source_conversation_id = if let Some(conversation_id) = source_conversation_id {
            let conversation_meta = self
                .get_conversation_meta(state, conversation_id)
                .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
            if self.conversation_meta_is_local_normal_chat_meta_view(&conversation_meta) {
                Some(conversation_meta.id.to_string())
            } else {
                None
            }
        } else {
            self.resolve_latest_foreground_conversation_id(state, &effective_agent_id)?
        }
        .ok_or_else(|| "当前没有可归档的活动对话。".to_string())?;
        let source_meta = self
            .get_conversation_meta(state, &source_conversation_id)
            .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
        if !self.conversation_meta_is_local_normal_chat_meta_view(&source_meta) {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }
        let source_agent_id = source_meta.agent_id.trim();
        if source_agent_id.is_empty() {
            drop(guard);
            return Err(format!(
                "会话未绑定人格，无法归档: conversation_id={}",
                source_meta.id
            ));
        }
        if !runtime_agents
            .iter()
            .any(|agent| agent.id == source_agent_id && !agent.is_built_in_user)
        {
            drop(guard);
            return Err(format!(
                "会话绑定人格不存在或不可用，无法归档: conversation_id={}, agent_id={}",
                source_meta.id, source_agent_id
            ));
        }
        let source = self.get_conversation_snapshot(state, &source_meta.id)?;
        let effective_agent_id = source_agent_id.to_string();
        drop(guard);
        Ok((selected_api, resolved_api, source, effective_agent_id))
    }

    fn resolve_archive_request_conversation_by_id(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<(ApiConfig, ResolvedApiConfig, Conversation, String), String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required".to_string());
        }
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let app_config = &runtime_snapshot.config;
        let source_meta = self
            .get_conversation_meta(state, normalized_conversation_id)
            .map_err(|_| "当前没有可归档的活动对话。".to_string())?;
        if !self.conversation_meta_is_local_normal_chat_meta_view(&source_meta)
            && source_meta.status.trim() != "archived"
        {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }
        let department_id = source_meta.department_id.trim();
        let department = if department_id.is_empty() {
            runtime_log_warn(format!(
                "[归档] 跳过部门校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，原因=会话未绑定部门，改为直接归档并跳过归档反思",
                source_meta.id
            ));
            None
        } else {
            match runtime_department_by_id(&runtime_snapshot, department_id) {
                Some(department) => Some(department),
                None => {
                    runtime_log_warn(format!(
                        "[归档] 跳过部门校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，department_id={}，原因=会话绑定部门不存在，改为直接归档并跳过归档反思",
                        source_meta.id, department_id
                    ));
                    None
                }
            }
        };
        let effective_agent_id = source_meta.agent_id.trim();
        let effective_agent_id = if effective_agent_id.is_empty() {
            runtime_log_warn(format!(
                "[归档] 跳过人格校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，原因=会话未绑定人格，不再回退到其他人格；后续如无法确定归档归属，将直接跳过归档反思",
                source_meta.id
            ));
            String::new()
        } else if runtime_snapshot
            .agents
            .iter()
            .any(|agent| agent.id == effective_agent_id && !agent.is_built_in_user)
        {
            effective_agent_id.to_string()
        } else {
            runtime_log_warn(format!(
                "[归档] 跳过人格校验，任务=resolve_archive_request_conversation_by_id，conversation_id={}，agent_id={}，原因=会话绑定人格不存在或不可用，不再回退到其他人格；后续如无法确定归档归属，将直接跳过归档反思",
                source_meta.id, effective_agent_id
            ));
            effective_agent_id.to_string()
        };
        let preferred_api_id = source_meta
            .preferred_api_config_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|api_id| resolve_department_chat_api_config_id(app_config, api_id));
        let selected_api_id = preferred_api_id.or_else(|| {
            department.and_then(|department| department_primary_chat_api_config_id(app_config, department))
        });
        let selected_api = resolve_selected_api_config(app_config, selected_api_id.as_deref())
            .ok_or_else(|| "No API config configured. Please add one.".to_string())?;
        let resolved_api = resolve_api_config(app_config, Some(selected_api.id.as_str()))?;
        let source = self.get_conversation_snapshot(state, &source_meta.id)?;
        drop(guard);
        Ok((selected_api, resolved_api, source, effective_agent_id))
    }

    fn delete_main_conversation_and_activate_latest(
        &self,
        state: &AppState,
        selected_api: &ApiConfig,
        source: &Conversation,
    ) -> Result<String, String> {
        let guard = state
            .conversation_lock
            .lock()
            .map_err(|err| format!("Failed to lock state mutex at {}:{} {}: {err}", file!(), line!(), module_path!()))?;
        let mut runtime = state_read_runtime_state_cached(state)?;
        let agents = state_read_agents_cached(state)?;
        let source_conversation = read_conversation_for_backup_cleanup(state, &source.id)
            .map_err(|_| "活动对话已变化，请重试归档。".to_string())?;
        if !conversation_is_archived(&source_conversation) || conversation_is_delegate(&source_conversation) {
            drop(guard);
            return Err("活动对话已变化，请重试归档。".to_string());
        }
        match cleanup_backup_records_from_messages(&state.data_path, &source_conversation.messages) {
            Ok(cleaned) if cleaned > 0 => {
                runtime_log_info(format!(
                    "[会话删除] apply_patch 备份清理完成: conversation={}, cleaned={}",
                    source.id, cleaned
                ));
            }
            Err(err) => {
                runtime_log_error(format!(
                    "[会话删除] apply_patch 备份清理失败: conversation={}, error={}",
                    source.id, err
                ));
            }
            _ => {}
        }
        state_schedule_conversation_delete(state, &source.id)?;
        let system_notification_exists = self
            .get_conversation_meta(state, SYSTEM_NOTIFICATION_CONVERSATION_ID)
            .ok()
            .filter(|conversation_meta| {
                self.conversation_meta_is_unarchived_meta_view(conversation_meta)
                    && conversation_meta.visible_in_foreground_lists
                    && self.conversation_meta_is_system_notification_meta_view(conversation_meta)
            })
            .is_some();
        if !system_notification_exists {
            let system_notification = build_system_notification_conversation_record();
            state_schedule_conversation_persist(state, &system_notification)?;
        }
        if runtime.main_conversation_id.as_deref().map(str::trim)
            != Some(SYSTEM_NOTIFICATION_CONVERSATION_ID)
        {
            runtime.main_conversation_id = Some(SYSTEM_NOTIFICATION_CONVERSATION_ID.to_string());
            state_write_runtime_state_cached(state, &runtime)?;
        }
        let chat_index = state_read_chat_index_cached(state)?;
        let active_conversation_id = chat_index
            .conversations
            .iter()
            .filter_map(|item| self.get_conversation_meta(state, item.id.as_str()).ok())
            .find(|conversation_meta| {
                conversation_meta.id != source.id
                    && !conversation_meta.is_delegate
                    && self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .map(|conversation_meta| conversation_meta.id.to_string());
        let active_conversation_id = if let Some(active_conversation_id) = active_conversation_id {
            active_conversation_id
        } else {
            let replacement = build_archive_replacement_conversation(
                state,
                &agents,
                &runtime.assistant_department_agent_id,
                selected_api,
                &source_conversation,
            )?;
            let replacement_id = replacement.id.clone();
            state_schedule_conversation_persist(state, &replacement)?;
            replacement_id
        };
        drop(guard);

        cleanup_pdf_session_memory_cache_for_conversation(&source.id);
        Ok(active_conversation_id)
    }

    fn remote_im_apply_dynamic_wake_compaction(
        &self,
        state: &AppState,
        conversation_id: &str,
        trigger_message_id: &str,
        include_history: bool,
    ) -> Result<ChatMessage, String> {
        const WINDOW_SECONDS: i64 = 60 * 60;
        const WINDOW_MAX_CHARS: usize = 10_000;
        const WINDOW_MIN_MESSAGES: usize = 7;

        let conversation_id = conversation_id.trim();
        let trigger_message_id = trigger_message_id.trim();
        if conversation_id.is_empty() || trigger_message_id.is_empty() {
            return Err("远程唤醒压缩失败：缺少会话或触发消息 ID".to_string());
        }
        let mutation_gate = conversation_mutation_gate(&state.data_path, conversation_id)?;
        let _guard = mutation_gate.lock().map_err(|err| {
            named_lock_error(
                "conversation_mutation_gate",
                file!(),
                line!(),
                module_path!(),
                &err,
            )
        })?;
        let conversation_meta = self.get_conversation_meta(state, conversation_id)?;
        if !conversation_meta.is_remote_im_contact {
            return Err(format!(
                "远程唤醒压缩失败：目标不是远程联系人会话，conversation_id={conversation_id}"
            ));
        }
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(state, conversation_id, &store_paths)?;
        let trigger = message_store::read_ready_message_store_message_by_id(
            &store_paths,
            trigger_message_id,
        )?
        .ok_or_else(|| format!("远程唤醒压缩失败：触发消息不存在，message_id={trigger_message_id}"))?;
        let trigger_index = message_store::read_ready_message_store_message_sequence(
            &store_paths,
            trigger_message_id,
        )?
        .ok_or_else(|| format!("远程唤醒压缩失败：触发消息缺少序号，message_id={trigger_message_id}"))?;
        let selected = if include_history {
            // 触发消息可能不是当前批次的最后一条。把它作为读取上界而不包含它，
            // 触发后已落库的新消息会留在新 block，不能提前写进这次唤醒摘要。
            self.read_preserved_conversation_messages(
                state,
                conversation_id,
                Some(trigger_message_id),
                false,
                Some(WINDOW_SECONDS),
                WINDOW_MIN_MESSAGES,
                WINDOW_MAX_CHARS,
            )?
        } else {
            Vec::new()
        };
        let preserved_dialogue = selected
            .iter()
            .map(|message| {
                let role = if message.role.trim().eq_ignore_ascii_case("assistant") {
                    "助理"
                } else {
                    "用户"
                };
                format!("{role}：{}", render_prompt_message_text(message).trim())
            })
            .filter(|line| !line.trim().is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let summary = build_compaction_message(
            "",
            Some("远程唤醒上下文"),
            if include_history {
                "remote_im_wake_dynamic"
            } else {
                "remote_im_wake_empty_fallback"
            },
            None,
            (!preserved_dialogue.trim().is_empty()).then_some(preserved_dialogue.as_str()),
        );
        let mut persisted_conversation = self.build_conversation_record_from_meta_view(&conversation_meta);
        persisted_conversation.updated_at = now_iso();
        let cached_metadata = state_read_conversation_metadata_cached(state, conversation_id)?;
        let persist_meta = message_store::ConversationPersistMeta::from_conversation_with_spliced_messages(
            &persisted_conversation,
            &cached_metadata,
            std::slice::from_ref(&trigger),
            &[summary.clone(), trigger.clone()],
        );
        message_store::write_jsonl_snapshot_spliced_messages_shard(
            &store_paths,
            &persist_meta,
            trigger_index,
            1,
            &[summary.clone(), trigger.clone()],
        )?;
        state_mark_conversation_metadata_direct_persisted(state, conversation_id)?;
        let next_messages = message_store::read_ready_message_store_messages_after(
            &store_paths,
            &summary.id,
            1,
        )?
        .map(|page| page.messages)
        .unwrap_or_default();
        if next_messages.first().map(|message| message.id.as_str()) != Some(trigger_message_id) {
            return Err(format!(
                "远程唤醒压缩写入校验失败：摘要和触发消息顺序错误，conversation_id={conversation_id}"
            ));
        }
        Ok(summary)
    }

    fn persist_compaction_message(
        &self,
        state: &AppState,
        source: &Conversation,
        compression_message: &ChatMessage,
        refreshed_user_profile_snapshot: Option<String>,
    ) -> Result<CompactionMessagePersistResult, String> {
        let mutation_gate = conversation_mutation_gate(&state.data_path, &source.id)?;
        let guard = mutation_gate.lock().map_err(|err| {
            named_lock_error(
                "conversation_mutation_gate",
                file!(),
                line!(),
                module_path!(),
                &err,
            )
        })?;
        let source_meta = self
            .get_conversation_meta(state, &source.id)
            .map_err(|_| "活动对话已变化，请重试上下文整理。".to_string())?;
        if !self.conversation_meta_is_unarchived_meta_view(&source_meta) {
            drop(guard);
            return Err("活动对话已变化，请重试上下文整理。".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, &source.id)?;
        ensure_ready_message_store_from_legacy_conversation(state, &source.id, &store_paths)?;
        let previous_latest_block_id = message_store::read_ready_message_store_block_page(
            &store_paths,
            None,
        )?
        .map(|page| page.selected_block_id);
        let compression_message_id = compression_message.id.clone();
        let now = now_iso();
        let (conversation_meta, (), _) = state_update_conversation_meta_cached(
            state,
            &source.id,
            |cached| {
                let mut metadata_conversation =
                    self.build_conversation_snapshot_from_meta(cached, Vec::new());
                metadata_conversation.user_profile_snapshot =
                    refreshed_user_profile_snapshot.clone().unwrap_or_default();
                metadata_conversation.updated_at = now.clone();
                metadata_conversation.last_user_at = Some(now.clone());
                cached.apply_metadata_fields_from_conversation(&metadata_conversation);
                cached.apply_appended_messages(std::slice::from_ref(compression_message));
                Ok(())
            },
        )?;
        let metadata_conversation =
            self.build_conversation_snapshot_from_meta(&conversation_meta, Vec::new());
        state_upsert_chat_index_conversation_cached(state, &metadata_conversation)?;
        let active_conversation_id = Some(metadata_conversation.id.clone());
        let mut ready_meta = message_store::read_ready_message_store_meta(&store_paths)?
            .ok_or_else(|| {
                format!(
                    "写入上下文整理消息失败：缺少 ready 消息元数据，conversation_id={}",
                    metadata_conversation.id
                )
            })?;
        ready_meta.apply_metadata_fields_from_meta(&conversation_meta);
        ready_meta.apply_appended_messages(std::slice::from_ref(compression_message));
        message_store::write_jsonl_snapshot_appended_messages_shard_from_meta(
            &store_paths,
            &ready_meta,
            std::slice::from_ref(compression_message),
        )?;
        // v3 保留远程联系人的完整 JSONL 历史；压缩消息只作为新消息追加。
        // 旧的“仅保留最后 block”策略会删掉正文并触发整会话 snapshot 重写。
        self.mark_conversation_metadata_cached_persisted(state, &metadata_conversation.id)?;

        drop(guard);

        let persisted = message_store::read_ready_message_store_message_by_id(
            &store_paths,
            &compression_message_id,
        )?
        .is_some();
        if !persisted {
            return Err(
                "上下文整理消息写入校验失败：已执行整理但未找到落盘消息，请重试。".to_string(),
            );
        }
        let latest_block = message_store::read_ready_message_store_block_page(&store_paths, None)?
            .ok_or_else(|| {
                format!(
                    "上下文整理消息写入校验失败：缺少最新块，conversation_id={}",
                    source.id
                )
            })?;
        if previous_latest_block_id.is_some()
            && Some(latest_block.selected_block_id) == previous_latest_block_id
        {
            return Err(format!(
                "上下文整理消息写入校验失败：未创建新的摘要块，conversation_id={}",
                source.id
            ));
        }
        let first_message_id = latest_block
            .blocks
            .iter()
            .find(|block| block.block_id == latest_block.selected_block_id)
            .map(|block| block.first_message_id.as_str())
            .unwrap_or_default();
        if first_message_id.trim() != compression_message_id {
            return Err(format!(
                "上下文整理消息写入校验失败：摘要消息不是新块首条消息，conversation_id={}",
                source.id
            ));
        }

        Ok(CompactionMessagePersistResult {
            active_conversation_id,
            compression_message_id,
        })
    }

    fn import_archives(
        &self,
        state: &AppState,
        incoming_archives: &mut Vec<ConversationArchive>,
    ) -> Result<ImportArchivesMutationResult, String> {
        let guard = state.conversation_lock.lock().map_err(|err| {
            format!(
                "Failed to lock state mutex at {}:{} {}: {err}",
                file!(),
                line!(),
                module_path!()
            )
        })?;
        let chat_index = state_read_chat_index_cached(state)?;
        let existing_archive_ids = chat_index
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .map(|item| item.id.clone())
            .collect::<std::collections::HashSet<_>>();

        let mut imported_count = 0usize;
        let mut replaced_count = 0usize;
        let mut skipped_count = 0usize;
        let mut selected_archive_id: Option<String> = None;
        let mut seen_conversation_ids = std::collections::HashSet::<String>::new();

        for archive in incoming_archives.iter_mut() {
            normalize_archive_for_import(archive, &state.data_path);
        }

        for archive in incoming_archives.drain(..) {
            let archive_id = archive.archive_id.clone();
            let conversation = archive_to_conversation(archive);
            let conversation_id = conversation.id.clone();
            if !seen_conversation_ids.insert(conversation_id.clone()) {
                skipped_count += 1;
                continue;
            }
            self.import_conversation_snapshot(
                state,
                &format!("archive_import_{}", archive_id),
                "archive_import",
                "archive_json_import",
                &conversation,
            )?;
            if existing_archive_ids.contains(&conversation_id) {
                replaced_count += 1;
            } else {
                imported_count += 1;
            }
            if selected_archive_id.is_none() {
                selected_archive_id = Some(archive_id);
            }
        }
        drop(guard);
        let total_count = state_read_chat_index_cached(state)?
            .conversations
            .iter()
            .filter(|item| chat_index_item_is_archived(item))
            .count();

        Ok(ImportArchivesMutationResult {
            imported_count,
            replaced_count,
            skipped_count,
            total_count,
            selected_archive_id,
        })
    }
    fn archive_conversation(
        &self,
        state: &AppState,
        selected_api: &ApiConfig,
        source: &Conversation,
        archive_reason: &str,
    ) -> Result<InstantArchiveConversationMutationResult, String> {
        let mutation_gate = conversation_mutation_gate(&state.data_path, &source.id)?;
        let guard = mutation_gate.lock().map_err(|err| {
            named_lock_error(
                "conversation_mutation_gate",
                file!(),
                line!(),
                module_path!(),
                &err,
            )
        })?;
        let source_conversation_meta = self
            .get_conversation_meta(state, &source.id)
            .map_err(|err| format!("当前没有可归档的活动对话：{}", err))?;
        let source_conversation =
            self.build_conversation_record_from_meta_view(&source_conversation_meta);
        let already_archived = source_conversation_meta.status.trim() == "archived";
        if !already_archived
            && !self.conversation_meta_is_local_normal_chat_meta_view(&source_conversation_meta)
        {
            drop(guard);
            return Err("当前没有可归档的活动对话。".to_string());
        }

        let runtime = state_read_runtime_state_cached(state)?;
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let agents = runtime_snapshot.agents;
        let chat_index = state_read_chat_index_cached(state)?;
        let active_conversation_id = if let Some(conversation_id) = chat_index
            .conversations
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let conversation_meta = self.get_conversation_meta(state, item.id.as_str()).ok()?;
                Some((idx, conversation_meta))
            })
            .filter(|(_, conversation_meta)| {
                conversation_meta.id != source.id
                    && self.conversation_meta_is_local_normal_chat_meta_view(conversation_meta)
            })
            .max_by(|(idx_a, a), (idx_b, b)| {
                let a_updated = a.updated_at.trim();
                let b_updated = b.updated_at.trim();
                let a_created = a.created_at.trim();
                let b_created = b.created_at.trim();
                a_updated
                    .cmp(b_updated)
                    .then_with(|| a_created.cmp(b_created))
                    .then_with(|| idx_a.cmp(idx_b))
            })
            .map(|(_, conversation_meta)| conversation_meta.id.to_string())
        {
            conversation_id
        } else {
            let conversation = build_archive_replacement_conversation(
                state,
                &agents,
                &runtime.assistant_department_agent_id,
                selected_api,
                source,
            )?;
            let conversation_id = conversation.id.clone();
            state_schedule_conversation_persist(state, &conversation)?;
            conversation_id
        };

        if !already_archived {
            let previous_status = source_conversation.status.clone();
            let now = now_iso();
            let (conversation, (), _) = state_update_conversation_metadata_cached(
                state,
                &source.id,
                |conversation| {
                    conversation.status = "archived".to_string();
                    conversation.summary.clear();
                    conversation.fast_request_turns.clear();
                    conversation.archived_at = Some(now.clone());
                    conversation.updated_at = now.clone();
                    Ok(())
                },
            )?;
            runtime_log_info(format!(
                "[归档] 完成，任务=即时标记归档，conversation_id={}，previous_status={}，reason={}，archived_at={}",
                conversation.id,
                previous_status,
                archive_reason,
                conversation.archived_at.as_deref().unwrap_or("")
            ));
        }
        let app_config = runtime_snapshot.config;
        let unarchived_conversations =
            self.collect_unarchived_conversation_summaries_cached(state, &app_config)?;
        let overview_payload = UnarchivedConversationOverviewUpdatedPayload {
            preferred_conversation_id: Some(active_conversation_id.clone()),
            unarchived_conversations,
        };
        drop(guard);
        Ok(InstantArchiveConversationMutationResult {
            active_conversation_id,
            overview_payload,
            already_archived,
        })
    }

}
