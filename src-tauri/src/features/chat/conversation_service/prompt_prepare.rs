impl ConversationServiceV2 {
    fn resolve_prompt_prepare_conversation_read_only(
        &self,
        data: &AppData,
        data_path: &PathBuf,
        runtime_conversation_id: Option<&str>,
        runtime_conversation: &Conversation,
        selected_api: &ApiConfig,
        effective_agent_id: &str,
        requested_conversation_id: Option<&str>,
    ) -> Result<Option<PromptPrepareConversationResolution>, String> {
        let mut cloned = data.clone();
        self.resolve_prompt_prepare_conversation_core_v2(
            &mut cloned,
            data_path,
            runtime_conversation_id,
            runtime_conversation,
            selected_api,
            effective_agent_id,
            requested_conversation_id,
            true,
        )
    }

    fn resolve_prompt_prepare_conversation_core_v2(
        &self,
        data: &mut AppData,
        data_path: &PathBuf,
        runtime_conversation_id: Option<&str>,
        runtime_conversation: &Conversation,
        selected_api: &ApiConfig,
        effective_agent_id: &str,
        requested_conversation_id: Option<&str>,
        read_only: bool,
    ) -> Result<Option<PromptPrepareConversationResolution>, String> {
        let requested_conversation_idx = requested_conversation_id.and_then(|conversation_id| {
            data.conversations
                .iter()
                .position(|item| item.id == conversation_id && conversation_is_unarchived(item))
        });
        let is_runtime_conversation = requested_conversation_id.is_some()
            && requested_conversation_idx.is_none()
            && runtime_conversation_id.is_some();
        let idx = if let Some(requested_idx) = requested_conversation_idx {
            Some(requested_idx)
        } else if is_runtime_conversation {
            None
        } else if let Some(conversation_id) = requested_conversation_id {
            if read_only {
                return Ok(None);
            }
            Some(
                data.conversations
                    .iter()
                    .position(|item| item.id == conversation_id && conversation_is_unarchived(item))
                    .ok_or_else(|| format!("指定会话不存在或不可用：{conversation_id}"))?,
            )
        } else if read_only {
            active_foreground_conversation_index_read_only(data, effective_agent_id)
        } else {
            Some(ensure_active_foreground_conversation_index_atomic(
                data,
                data_path,
                &selected_api.id,
                effective_agent_id,
            ))
        };
        if idx.is_some() && !read_only {
            for conversation in &mut data.conversations {
                if conversation_is_delegate(conversation) || conversation_is_archived(conversation)
                {
                    continue;
                }
                conversation.status = "active".to_string();
            }
        }

        let conversation_before = if let Some(actual_idx) = idx {
            data.conversations
                .get(actual_idx)
                .cloned()
                .ok_or_else(|| "前台会话索引无效".to_string())?
        } else {
            runtime_conversation.clone()
        };
        Ok(Some(self.build_prompt_prepare_resolution_v2(
            data,
            &conversation_before,
            selected_api,
            is_runtime_conversation,
        )))
    }

    fn build_prompt_prepare_resolution_v2(
        &self,
        data: &AppData,
        conversation_before: &Conversation,
        selected_api: &ApiConfig,
        is_runtime_conversation: bool,
    ) -> PromptPrepareConversationResolution {
        let is_remote_im_contact_conversation = conversation_is_remote_im_contact(conversation_before);
        let remote_im_contact_processing_mode = if is_remote_im_contact_conversation {
            remote_im_find_contact_by_conversation(data, &conversation_before.id)
                .map(|contact| normalize_contact_processing_mode(&contact.processing_mode))
                .unwrap_or_else(|| "continuous".to_string())
        } else {
            "continuous".to_string()
        };
        PromptPrepareConversationResolution {
            conversation_before: self.build_prompt_prepare_conversation_before_v2(
                conversation_before,
                is_remote_im_contact_conversation,
                &remote_im_contact_processing_mode,
            ),
            last_archive_summary: None,
            is_remote_im_contact_conversation,
            remote_im_contact_processing_mode,
            response_style_id: data.response_style_id.clone(),
            user_name: user_persona_name(data),
            user_intro: user_persona_intro(data),
            enable_pdf_images: data.pdf_read_mode == "image" && selected_api.enable_image,
            is_runtime_conversation,
        }
    }

    fn build_prompt_prepare_conversation_before_v2(
        &self,
        conversation_before: &Conversation,
        is_remote_im_contact_conversation: bool,
        remote_im_contact_processing_mode: &str,
    ) -> Conversation {
        if is_remote_im_contact_conversation && remote_im_contact_processing_mode == "qa" {
            let trimmed = remote_im_trim_conversation_for_qa_mode(conversation_before);
            runtime_log_info(format!(
                "[远程IM] 问答模式裁剪会话上下文: conversation_id={}, original_messages={}, trimmed_messages={}",
                conversation_before.id,
                conversation_before.messages.len(),
                trimmed.messages.len()
            ));
            return trimmed;
        }
        conversation_before.clone()
    }

    fn find_remote_im_contact_by_conversation_in_data<'a>(
        &self,
        data: &'a AppData,
        conversation_id: &str,
    ) -> Option<&'a RemoteImContact> {
        let conversation = data
            .conversations
            .iter()
            .find(|item| item.id == conversation_id)?;
        let contact_conversation_key = conversation
            .root_conversation_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(key) = contact_conversation_key {
            return data
                .remote_im_contacts
                .iter()
                .find(|contact| remote_im_contact_conversation_key(contact) == key);
        }
        data.remote_im_contacts.iter().find(|contact| {
            contact
                .bound_conversation_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                == Some(conversation_id)
        })
    }

    fn find_remote_im_contact_by_conversation_in_runtime<'a>(
        &self,
        runtime: &'a RuntimeStateFile,
        conversation_id: &str,
    ) -> Option<&'a RemoteImContact> {
        runtime.remote_im_contacts.iter().find(|contact| {
            contact
                .bound_conversation_id
                .as_deref()
                .map(str::trim)
                .filter(|value| !value.is_empty())
                == Some(conversation_id)
        })
    }

    fn try_get_conversation_snapshot_fast(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        self.try_read_persisted_conversation(state, conversation_id)
    }

    fn try_read_unarchived_conversation(
        &self,
        state: &AppState,
        conversation_id: &str,
    ) -> Result<Option<Conversation>, String> {
        Ok(self
            .try_get_conversation_snapshot_fast(state, conversation_id)?
            .filter(conversation_is_unarchived))
    }

    fn resolve_effective_agent_id_for_read(
        &self,
        state: &AppState,
        app_config: &mut AppConfig,
        runtime_agents: &[AgentProfile],
        assistant_department_agent_id: &str,
        requested_agent_id: &str,
    ) -> Result<String, String> {
        let runtime_snapshot =
            build_runtime_organization_snapshot_from_parts(&state.data_path, app_config, runtime_agents)?;
        *app_config = runtime_snapshot.config.clone();
        let runtime_agents = runtime_snapshot.agents;
        let requested_agent_id = requested_agent_id.trim();
        if !requested_agent_id.is_empty() {
            if runtime_agents
                .iter()
                .any(|agent| agent.id == requested_agent_id && !agent.is_built_in_user)
            {
                return Ok(requested_agent_id.to_string());
            }
            return Err(format!("Selected agent '{requested_agent_id}' not found."));
        }
        if runtime_agents.iter().any(|agent| {
            agent.id == assistant_department_agent_id && !agent.is_built_in_user
        }) {
            return Ok(assistant_department_agent_id.to_string());
        }
        runtime_agents
            .iter()
            .find(|agent| !agent.is_built_in_user)
            .map(|agent| agent.id.clone())
            .ok_or_else(|| "Selected agent not found.".to_string())
    }

}

