fn send_reasoning_delta_event(
    on_delta: &tauri::ipc::Channel<AssistantDeltaEvent>,
    reasoning: &str,
) {
    if reasoning.is_empty() {
        return;
    }
    let _ = on_delta.send(AssistantDeltaEvent {
        delta: reasoning.to_string(),
        kind: Some("activity_reasoning_delta".to_string()),
        request_id: None,
        activation_id: None,
        phase_id: None,
        reason: None,
        tool_name: None,
        tool_call_id: None,
        tool_status: None,
        tool_args: None,
        message: None,
        stream_cache: None,
    });
}

async fn collect_streaming_model_reply_genai<S>(
    mut stream: S,
    on_delta: Option<&tauri::ipc::Channel<AssistantDeltaEvent>>,
) -> Result<ModelReply, String>
where
    S: futures_util::Stream<Item = Result<genai::chat::ChatStreamEvent, genai::Error>> + Unpin,
{
    let mut assistant_text = String::new();
    let mut activity_reasoning_text = String::new();
    let mut trusted_input_tokens: Option<u64> = None;
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(genai::chat::ChatStreamEvent::Start) => {}
            Ok(genai::chat::ChatStreamEvent::Chunk(text)) => {
                assistant_text.push_str(&text.content);
                if let Some(channel) = on_delta {
                    let _ = channel.send(AssistantDeltaEvent {
                        delta: text.content,
                        kind: None,
                        request_id: None,
                        activation_id: None,
                        phase_id: None,
                        reason: None,
                        tool_name: None,
                        tool_call_id: None,
                        tool_status: None,
                        tool_args: None,
                        message: None,
                        stream_cache: None,
                    });
                }
            }
            Ok(genai::chat::ChatStreamEvent::ReasoningChunk(reasoning)) => {
                if !reasoning.content.is_empty() {
                    activity_reasoning_text.push_str(&reasoning.content);
                    if let Some(channel) = on_delta {
                        send_reasoning_delta_event(channel, &reasoning.content);
                    }
                }
            }
            Ok(genai::chat::ChatStreamEvent::ThoughtSignatureChunk(_)) => {}
            Ok(genai::chat::ChatStreamEvent::ToolCallChunk(_)) => {}
            Ok(genai::chat::ChatStreamEvent::End(end)) => {
                if assistant_text.is_empty() {
                    if let Some(captured_texts) = end
                        .captured_content
                        .as_ref()
                        .map(|content| content.texts())
                        .filter(|texts| !texts.is_empty())
                    {
                        let joined = captured_texts.join("\n");
                        assistant_text = joined.clone();
                        if let Some(channel) = on_delta {
                            let _ = channel.send(AssistantDeltaEvent {
                                delta: joined,
                                kind: None,
                                request_id: None,
                                activation_id: None,
                                phase_id: None,
                                reason: None,
                                tool_name: None,
                                tool_call_id: None,
                                tool_status: None,
                                tool_args: None,
                                message: None,
                                stream_cache: None,
                            });
                        }
                    }
                }
                if activity_reasoning_text.is_empty() {
                    if let Some(captured_reasoning) = end
                        .captured_reasoning_content
                        .as_deref()
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                    {
                        activity_reasoning_text = captured_reasoning.to_string();
                        if let Some(channel) = on_delta {
                            send_reasoning_delta_event(channel, captured_reasoning);
                        }
                    }
                }
                trusted_input_tokens = end
                    .captured_usage
                    .as_ref()
                    .and_then(|usage| usage.prompt_tokens)
                    .and_then(|value| u64::try_from(value).ok())
                    .filter(|value| *value > 0);
            }
            Err(err) => {
                runtime_log_error(format!(
                    "[聊天] GenAI 流式收集失败: error={:?}",
                    err
                ));
                return Err(format!("GenAI 流式收集失败：{:?}", err));
            }
        }
    }
    Ok(ModelReply {
        assistant_text: assistant_text.clone(),
        final_response_text: assistant_text,
        activity_reasoning_text,
        assistant_provider_meta: None,
        tool_history_events: Vec::new(),
        suppress_assistant_message: false,
        trusted_input_tokens,
        round_logs_recorded_internally: false,
    })
}

#[cfg(test)]
mod stream_collect_tests {
    use super::*;

    #[tokio::test]
    async fn captured_reasoning_at_stream_end_should_emit_reasoning_delta() {
        let events = std::sync::Arc::new(std::sync::Mutex::new(Vec::<AssistantDeltaEvent>::new()));
        let events_for_channel = events.clone();
        let channel = tauri::ipc::Channel::new(move |body| {
            let parsed_event = match body {
                tauri::ipc::InvokeResponseBody::Json(json) => {
                    serde_json::from_str::<AssistantDeltaEvent>(&json).ok()
                }
                tauri::ipc::InvokeResponseBody::Raw(bytes) => {
                    serde_json::from_slice::<AssistantDeltaEvent>(&bytes).ok()
                }
            };
            if let Some(event) = parsed_event {
                if let Ok(mut guard) = events_for_channel.lock() {
                    guard.push(event);
                }
            }
            Ok(())
        });
        let stream = futures_util::stream::iter(vec![Ok(genai::chat::ChatStreamEvent::End(
            genai::chat::StreamEnd {
                captured_reasoning_content: Some("先判断工具是否需要执行".to_string()),
                ..Default::default()
            },
        ))]);

        let reply = collect_streaming_model_reply_genai(stream, Some(&channel))
            .await
            .expect("stream collection should succeed");

        assert_eq!(reply.activity_reasoning_text, "先判断工具是否需要执行");
        let guard = events.lock().expect("events should be readable");
        assert_eq!(guard.len(), 1);
        assert_eq!(
            guard[0].kind.as_deref(),
            Some("activity_reasoning_delta")
        );
        assert_eq!(guard[0].delta, "先判断工具是否需要执行");
    }
}
