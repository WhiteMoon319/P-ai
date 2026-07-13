fn lock_remote_im_contact_runtime_states(
    state: &AppState,
) -> Result<std::sync::MutexGuard<'_, std::collections::HashMap<String, RemoteImContactRuntimeState>>, String>
{
    state
        .remote_im_contact_runtime_states
        .lock()
        .map_err(|_| "无法获取远程 IM 联系人运行时状态的锁".to_string())
}

#[derive(Clone)]
struct RemoteImAssistantDebounce {
    token: String,
    contact_id: String,
    sender_id: String,
    start_message_id: String,
    end_message_id: String,
    event: ChatPendingEvent,
}

#[derive(Clone)]
struct RemoteImSecretaryDebounce {
    token: String,
    contact_id: String,
    start_message_id: String,
    end_message_id: String,
    must_reply: bool,
    event: ChatPendingEvent,
}

#[derive(Default)]
struct RemoteImDebounceState {
    assistant_by_sender: std::collections::HashMap<String, RemoteImAssistantDebounce>,
    secretary_by_contact: std::collections::HashMap<String, RemoteImSecretaryDebounce>,
}

#[derive(Clone)]
struct RemoteImReplyDebounceReady {
    start_message_id: String,
    end_message_id: String,
    must_reply: bool,
    event: ChatPendingEvent,
}

fn remote_im_debounce_state() -> &'static std::sync::Mutex<RemoteImDebounceState> {
    static STATE: std::sync::OnceLock<std::sync::Mutex<RemoteImDebounceState>> =
        std::sync::OnceLock::new();
    STATE.get_or_init(|| std::sync::Mutex::new(RemoteImDebounceState::default()))
}

fn remote_im_assistant_debounce_key(
    state: &AppState,
    contact_id: &str,
    sender_id: &str,
) -> String {
    format!(
        "{}::{}::{}",
        state.data_path.to_string_lossy(),
        contact_id.trim(),
        sender_id.trim()
    )
}

fn remote_im_secretary_debounce_key(state: &AppState, contact_id: &str) -> String {
    format!("{}::{}", state.data_path.to_string_lossy(), contact_id.trim())
}

fn remote_im_event_latest_user_message(event: &ChatPendingEvent) -> Option<&ChatMessage> {
    event
        .messages
        .iter()
        .rev()
        .find(|message| message.role.trim().eq_ignore_ascii_case("user"))
}

fn remote_im_event_hits_wake(contact: &RemoteImContact, event: &ChatPendingEvent) -> bool {
    let Some(message) = remote_im_event_latest_user_message(event) else {
        return false;
    };
    remote_im_should_activate_while_away(contact, &render_message_content_for_model(message)).0
}

fn remote_im_contact_is_muted(state: &AppState, contact_id: &str) -> Result<bool, String> {
    Ok(lock_remote_im_contact_runtime_states(state)?
        .get(contact_id)
        .and_then(|runtime| runtime.mute_until.as_deref())
        .is_some())
}

fn clear_remote_im_debounces_for_contact(
    state: &AppState,
    contact_id: &str,
) -> Result<(), String> {
    let assistant_prefix = format!(
        "{}::{}::",
        state.data_path.to_string_lossy(),
        contact_id.trim()
    );
    let secretary_key = remote_im_secretary_debounce_key(state, contact_id);
    let mut debounces = remote_im_debounce_state()
        .lock()
        .map_err(|_| "无法获取远程联系人防抖状态锁".to_string())?;
    debounces
        .assistant_by_sender
        .retain(|key, _| !key.starts_with(&assistant_prefix));
    debounces.secretary_by_contact.remove(&secretary_key);
    Ok(())
}

fn observe_remote_im_persisted_event(
    state: &AppState,
    contact: &RemoteImContact,
    event: &ChatPendingEvent,
) -> Result<(), String> {
    if remote_im_contact_is_muted(state, &contact.id)? {
        clear_remote_im_debounces_for_contact(state, &contact.id)?;
        return Ok(());
    }
    let Some(message) = remote_im_event_latest_user_message(event) else {
        return Ok(());
    };
    let sender_id = event
        .sender_info
        .as_ref()
        .map(|sender| sender.sender_id.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "远程联系人消息缺少群友 ID".to_string())?;
    let hits_wake = remote_im_event_hits_wake(contact, event);
    let assistant_key = remote_im_assistant_debounce_key(state, &contact.id, &sender_id);
    let secretary_key = remote_im_secretary_debounce_key(state, &contact.id);
    let assistant_token = Uuid::new_v4().to_string();
    let secretary_token = Uuid::new_v4().to_string();
    let mut schedule_assistant = None;
    let mut schedule_secretary = None;
    {
        let mut debounces = remote_im_debounce_state()
            .lock()
            .map_err(|_| "无法获取远程联系人防抖状态锁".to_string())?;
        let has_assistant = debounces
            .assistant_by_sender
            .values()
            .any(|item| item.contact_id == contact.id);
        if let Some(existing) = debounces.assistant_by_sender.get_mut(&assistant_key) {
            existing.token = assistant_token.clone();
            existing.end_message_id = message.id.clone();
            existing.event = event.clone();
            schedule_assistant = Some(existing.clone());
        } else if hits_wake && !debounces.secretary_by_contact.contains_key(&secretary_key) {
            let debounce = RemoteImAssistantDebounce {
                token: assistant_token.clone(),
                contact_id: contact.id.clone(),
                sender_id: sender_id.clone(),
                start_message_id: message.id.clone(),
                end_message_id: message.id.clone(),
                event: event.clone(),
            };
            debounces
                .assistant_by_sender
                .insert(assistant_key.clone(), debounce.clone());
            schedule_assistant = Some(debounce);
        }

        let has_assistant = has_assistant || schedule_assistant.is_some();
        if !has_assistant {
            if let Some(existing) = debounces.secretary_by_contact.get_mut(&secretary_key) {
            existing.token = secretary_token.clone();
            existing.end_message_id = message.id.clone();
            existing.event = event.clone();
            if hits_wake {
                existing.must_reply = true;
            }
            schedule_secretary = Some(existing.clone());
            } else if !hits_wake && !remote_im_contact_is_away(state, &contact.id)? {
                let debounce = RemoteImSecretaryDebounce {
                    token: secretary_token.clone(),
                    contact_id: contact.id.clone(),
                    start_message_id: message.id.clone(),
                    end_message_id: message.id.clone(),
                    must_reply: false,
                    event: event.clone(),
                };
                debounces
                    .secretary_by_contact
                    .insert(secretary_key.clone(), debounce.clone());
                schedule_secretary = Some(debounce);
            }
        }
    }
    if let Some(debounce) = schedule_assistant {
        let state_clone = state.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let ready = remote_im_debounce_state().lock().ok().and_then(|mut state| {
                let current = state.assistant_by_sender.get(&assistant_key)?;
                if current.token != debounce.token {
                    return None;
                }
                state.assistant_by_sender.remove(&assistant_key)
            });
            let Some(ready) = ready else { return; };
            let payload = RemoteImReplyDebounceReady {
                start_message_id: ready.start_message_id,
                end_message_id: ready.end_message_id,
                must_reply: true,
                event: ready.event,
            };
            if let Err(err) = process_remote_im_reply_debounce(&state_clone, payload).await {
                runtime_log_error(format!(
                    "[远程联系人助理防抖] 失败，contact_id={}，sender_id={}，error={}",
                    ready.contact_id, ready.sender_id, err
                ));
            }
        });
    }
    if let Some(debounce) = schedule_secretary {
        let state_clone = state.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(if debounce.must_reply {
                1
            } else {
                7
            }))
            .await;
            let ready = remote_im_debounce_state().lock().ok().and_then(|mut state| {
                let current = state.secretary_by_contact.get(&secretary_key)?;
                if current.token != debounce.token {
                    return None;
                }
                state.secretary_by_contact.remove(&secretary_key)
            });
            let Some(ready) = ready else { return; };
            let payload = RemoteImReplyDebounceReady {
                start_message_id: ready.start_message_id,
                end_message_id: ready.end_message_id,
                must_reply: ready.must_reply,
                event: ready.event,
            };
            if let Err(err) = process_remote_im_reply_debounce(&state_clone, payload).await {
                runtime_log_error(format!(
                    "[远程联系人秘书防抖] 失败，contact_id={}，error={}",
                    ready.contact_id, err
                ));
            }
        });
    }
    Ok(())
}

