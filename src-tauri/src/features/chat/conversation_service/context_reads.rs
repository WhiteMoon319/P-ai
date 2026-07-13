impl ConversationServiceV2 {
    fn ensure_unarchived_conversation(
        &self,
        conversation: &Conversation,
        conversation_id: &str,
    ) -> Result<(), String> {
        if !conversation_is_unarchived(conversation) {
            return Err(format!(
                "Unarchived conversation not found: {}",
                conversation_id.trim()
            ));
        }
        Ok(())
    }

    fn read_persisted_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Err("conversationId is required.".to_string());
        }
        let conversation_meta =
            self.get_conversation_meta(state, normalized_conversation_id)?;
        let store_paths =
            message_store::message_store_paths(&state.data_path, normalized_conversation_id)?;
        ensure_ready_message_store_from_legacy_conversation(
            state,
            normalized_conversation_id,
            &store_paths,
        )?;
        let messages =
            message_store::read_ready_message_store_all_messages(&store_paths)?.unwrap_or_default();
        let mut conversation = self.build_conversation_record_from_meta_view(&conversation_meta);
        conversation.messages = messages;
        Ok(conversation)
    }

    fn read_archive_pipeline_source_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        self.read_persisted_conversation(state, conversation_id)
    }

    /// 普通 A 的压缩输入复用保留对话读取器，只是其边界为固定 10K 字符。
    fn read_archive_pipeline_cross_message_context(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Conversation, String> {
        const COMPACTION_CONTEXT_MAX_CHARS: usize = 10_000;
        let conversation_meta = self.get_conversation_meta(state, conversation_id)?;
        let mut context_messages = self.read_preserved_conversation_messages(
            state,
            conversation_id,
            None,
            true,
            None,
            0,
            COMPACTION_CONTEXT_MAX_CHARS,
        )?;
        materialize_chat_message_parts_from_media_refs(&mut context_messages, &state.data_path);
        let mut context = self.build_conversation_record_from_meta_view(&conversation_meta);
        context.messages = context_messages;
        Ok(context)
    }

    /// 从全局消息序列向前读取真实会话正文。读取不以 block 为边界：每页仅取四条，
    /// 每条先过滤旧压缩消息、非 user/assistant 与空正文，随后才计算窗口。
    ///
    /// `end_message_id` 为可选上界；传空时使用全局最新消息。远程唤醒在并发下会
    /// 传入触发消息并排除该消息，防止把触发后到达的新消息提前写进旧摘要。
    fn read_preserved_conversation_messages(
        &self,
        state: &AppState,
        conversation_id: &str,
        end_message_id: Option<&str>,
        include_end_message: bool,
        max_history_seconds: Option<i64>,
        min_message_count: usize,
        max_chars: usize,
    ) -> Result<Vec<ChatMessage>, String> {
        const PAGE_SIZE: usize = 4;
        let conversation_id = conversation_id.trim();
        if conversation_id.is_empty() {
            return Err("读取保留对话失败：缺少会话 ID".to_string());
        }
        if max_chars == 0 {
            return Err("读取保留对话失败：最大字符数必须大于 0".to_string());
        }
        let store_paths = message_store::message_store_paths(&state.data_path, conversation_id)?;
        let end_message_id = end_message_id.map(str::trim).filter(|id| !id.is_empty());
        let Some(anchor_message) = (match end_message_id {
            Some(message_id) => {
                message_store::read_ready_message_store_message_by_id(&store_paths, message_id)?
            }
            None => message_store::read_ready_message_store_recent_messages(&store_paths, 1)?
                .and_then(|mut messages| messages.pop()),
        }) else {
            return Ok(Vec::new());
        };
        if anchor_message.id.trim().is_empty() {
            return Ok(Vec::new());
        }
        let anchor_at = parse_iso(&anchor_message.created_at).unwrap_or_else(now_utc);
        let mut selected_newest_first = Vec::<ChatMessage>::new();
        let mut selected_chars = 0usize;
        let mut page_anchor_message_id = anchor_message.id.clone();
        if include_end_message {
            match Self::select_preserved_conversation_message(
                anchor_message,
                anchor_at,
                max_history_seconds,
                min_message_count,
                selected_newest_first.len(),
                selected_chars,
                max_chars,
            ) {
                PreservedConversationMessageSelection::Select { message, chars } => {
                    selected_chars = selected_chars.saturating_add(chars);
                    selected_newest_first.push(message);
                }
                PreservedConversationMessageSelection::Skip
                | PreservedConversationMessageSelection::Stop => {}
            }
        }
        runtime_log_debug(format!(
            "[上下文整理] 开始，任务=消息锚定向前读取，conversation_id={}，target_message_id={}，selected_message_count={}，selected_chars={}，max_chars={}",
            conversation_id,
            page_anchor_message_id,
            selected_newest_first.len(),
            selected_chars,
            max_chars
        ));
        let mut page_index = 0usize;
        while selected_newest_first.len() < min_message_count || selected_chars < max_chars {
            runtime_log_debug(format!(
                "[上下文整理] 开始，任务=向前读取消息页，conversation_id={}，page_index={}，anchor_message_id={}，selected_message_count={}，selected_chars={}，max_chars={}",
                conversation_id,
                page_index + 1,
                page_anchor_message_id,
                selected_newest_first.len(),
                selected_chars,
                max_chars
            ));
            let Some(page) = message_store::read_ready_message_store_messages_before(
                &store_paths,
                &page_anchor_message_id,
                PAGE_SIZE,
            )?
            else {
                runtime_log_debug(format!(
                    "[上下文整理] 完成，任务=向前读取消息页，conversation_id={}，page_index={}，result=store_unavailable",
                    conversation_id,
                    page_index + 1
                ));
                break;
            };
            if page.messages.is_empty() {
                runtime_log_debug(format!(
                    "[上下文整理] 完成，任务=向前读取消息页，conversation_id={}，page_index={}，result=empty_page，has_more={}",
                    conversation_id,
                    page_index + 1,
                    page.has_more
                ));
                break;
            }
            page_index = page_index.saturating_add(1);
            let next_anchor_message_id = page.messages.first().map(|message| message.id.clone());
            let page_message_count = page.messages.len();
            let mut page_selected_count = 0usize;
            let mut page_skipped_count = 0usize;
            let mut reached_boundary = false;
            for message in page.messages.into_iter().rev() {
                match Self::select_preserved_conversation_message(
                    message,
                    anchor_at,
                    max_history_seconds,
                    min_message_count,
                    selected_newest_first.len(),
                    selected_chars,
                    max_chars,
                ) {
                    PreservedConversationMessageSelection::Select { message, chars } => {
                        selected_chars = selected_chars.saturating_add(chars);
                        page_selected_count = page_selected_count.saturating_add(1);
                        selected_newest_first.push(message);
                    }
                    PreservedConversationMessageSelection::Skip => {
                        page_skipped_count = page_skipped_count.saturating_add(1);
                    }
                    PreservedConversationMessageSelection::Stop => {
                        reached_boundary = true;
                        break;
                    }
                }
            }
            runtime_log_debug(format!(
                "[上下文整理] 完成，任务=向前读取消息页，conversation_id={}，page_index={}，page_message_count={}，page_selected_count={}，page_skipped_count={}，selected_message_count={}，selected_chars={}，max_chars={}，has_more={}，next_anchor_message_id={}",
                conversation_id,
                page_index,
                page_message_count,
                page_selected_count,
                page_skipped_count,
                selected_newest_first.len(),
                selected_chars,
                max_chars,
                page.has_more,
                next_anchor_message_id.as_deref().unwrap_or("")
            ));
            if reached_boundary
                || (selected_newest_first.len() >= min_message_count && selected_chars >= max_chars)
                || !page.has_more
            {
                break;
            }
            let Some(next_anchor_message_id) = next_anchor_message_id else {
                break;
            };
            page_anchor_message_id = next_anchor_message_id;
        }
        selected_newest_first.reverse();
        Ok(selected_newest_first)
    }

    fn select_preserved_conversation_message(
        message: ChatMessage,
        anchor_at: OffsetDateTime,
        max_history_seconds: Option<i64>,
        min_message_count: usize,
        selected_message_count: usize,
        selected_chars: usize,
        max_chars: usize,
    ) -> PreservedConversationMessageSelection {
        let role = message.role.trim().to_ascii_lowercase();
        if !matches!(role.as_str(), "user" | "assistant")
            || is_context_compaction_message(&message, role.as_str())
        {
            return PreservedConversationMessageSelection::Skip;
        }
        let body = render_prompt_message_text(&message);
        let message_chars = body.chars().count();
        if message_chars == 0 {
            return PreservedConversationMessageSelection::Skip;
        }
        let must_keep_for_minimum = selected_message_count < min_message_count;
        let within_history_window = max_history_seconds
            .map(|seconds| {
                parse_iso(&message.created_at)
                    .map(|created_at| {
                        (anchor_at - created_at).whole_seconds().abs() <= seconds
                    })
                    .unwrap_or(false)
            })
            .unwrap_or(true);
        let exceeds_dynamic_char_limit = max_history_seconds.is_some()
            && selected_chars.saturating_add(message_chars) > max_chars;
        if !must_keep_for_minimum && (!within_history_window || exceeds_dynamic_char_limit) {
            return PreservedConversationMessageSelection::Stop;
        }
        if must_keep_for_minimum || message_chars <= max_chars.saturating_sub(selected_chars) {
            return PreservedConversationMessageSelection::Select {
                message,
                chars: message_chars,
            };
        }

        let visible = body
            .chars()
            .take(max_chars.saturating_sub(selected_chars).saturating_sub(1))
            .collect::<String>();
        let mut truncated = message;
        truncated.parts = vec![MessagePart::Text {
            text: format!("{visible}…"),
            reasoning_content: None,
        }];
        truncated.extra_text_blocks.clear();
        truncated.provider_meta = None;
        PreservedConversationMessageSelection::Select {
            message: truncated,
            chars: max_chars.saturating_sub(selected_chars),
        }
    }

    fn try_read_persisted_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        let normalized_conversation_id = conversation_id.trim();
        if normalized_conversation_id.is_empty() {
            return Ok(None);
        }
        match self.read_persisted_conversation(state, normalized_conversation_id) {
            Ok(conversation) => Ok(Some(conversation)),
            Err(err) if err.contains("not found") || err.contains("不存在") => Ok(None),
            Err(err) => Err(err),
        }
    }

    fn collect_unarchived_conversation_summaries_cached(
        &self,
        state: &AppState,
        app_config: &AppConfig,
    ) -> Result<Vec<UnarchivedConversationSummary>, String> {
        let runtime_snapshot = load_runtime_organization_snapshot(state)?;
        let runtime_app_config = if runtime_snapshot.config.departments.is_empty() {
            app_config.clone()
        } else {
            runtime_snapshot.config
        };
        let runtime = state_read_runtime_state_cached(state)?;
        let main_conversation_id = runtime
            .main_conversation_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .to_string();
        let chat_index = state_read_chat_index_cached(state)?;
        let visible_conversations = chat_index
            .conversations
            .iter()
            .filter(|item| !chat_index_item_is_archived(item))
            .filter_map(|item| {
                let conversation_meta = match self.get_conversation_meta(state, item.id.as_str()) {
                    Ok(conversation_meta) => conversation_meta,
                    Err(err) => {
                        runtime_log_error(format!(
                            "[会话索引读取] 状态=失败，任务=collect_unarchived_conversation_summaries_cached，conversation_id={}，error={}",
                            item.id, err
                        ));
                        return None;
                    }
                };
                (self.conversation_meta_is_unarchived_meta_view(&conversation_meta)
                    && conversation_meta.visible_in_foreground_lists)
                    .then_some(conversation_meta)
            })
            .collect::<Vec<_>>();
        let visible_ids = visible_conversations
            .iter()
            .map(|conversation_meta| conversation_meta.id.trim().to_string())
            .filter(|conversation_id: &String| !conversation_id.is_empty())
            .collect::<std::collections::HashSet<_>>();
        let mut seen_pins = std::collections::HashSet::<String>::new();
        let pinned_conversation_ids = runtime
            .pinned_conversation_ids
            .iter()
            .map(|item| item.trim().to_string())
            .filter(|item| !item.is_empty())
            .filter(|item| *item != main_conversation_id)
            .filter(|item| visible_ids.contains(item))
            .filter(|item| seen_pins.insert(item.clone()))
            .collect::<Vec<_>>();
        let summaries = visible_conversations
            .iter()
            .map(|conversation_meta| {
                let hydrated_conversation_meta =
                    self.fill_summary_preview_messages_fallback(state, conversation_meta);
                build_unarchived_conversation_summary_from_meta_view(
                    state,
                    &runtime_app_config,
                    &main_conversation_id,
                    &pinned_conversation_ids,
                    &hydrated_conversation_meta,
                    Some(DESKTOP_CHAT_VIEWER_ID),
                )
            })
            .collect::<Vec<_>>();
        Ok(sort_unarchived_conversation_summaries(summaries))
    }

}

