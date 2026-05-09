fn build_reject_response(
    status: tokio_tungstenite::tungstenite::http::StatusCode,
) -> HttpResponse<Option<String>> {
    tokio_tungstenite::tungstenite::http::Response::builder()
        .status(status)
        .body(None)
        .unwrap_or_else(|e| {
            eprintln!("[远程IM][OneBot v11 WS] 构建拒绝响应失败: {}", e);
            tokio_tungstenite::tungstenite::http::Response::builder()
                .status(tokio_tungstenite::tungstenite::http::StatusCode::INTERNAL_SERVER_ERROR)
                .body(None)
                .unwrap_or_else(|_| tokio_tungstenite::tungstenite::http::Response::new(None))
        })
}

fn validate_ws_token(req: &Request, expected: Option<&str>) -> Result<(), HttpResponse<Option<String>>> {
    let mut received_token: Option<String> = None;
    if let Some(query) = req.uri().query() {
        for pair in query.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                if key == "access_token" {
                    received_token = Some(value.to_string());
                }
            }
        }
    }
    if let Some(auth_header) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                received_token = Some(token.to_string());
            }
        }
    }
    if let Some(expect) = expected {
        match received_token.as_deref() {
            Some(token) if token == expect => Ok(()),
            _ => Err(build_reject_response(
                tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN,
            )),
        }
    } else {
        Ok(())
    }
}

async fn append_channel_log(
    channel_logs: &Arc<RwLock<HashMap<String, Vec<ChannelLogEntry>>>>,
    channel_id: &str,
    level: &str,
    message: String,
) {
    let mut logs = channel_logs.write().await;
    let entries = logs.entry(channel_id.to_string()).or_insert_with(Vec::new);
    entries.push(ChannelLogEntry {
        timestamp: Utc::now(),
        level: level.to_string(),
        message,
    });
    if entries.len() > CHANNEL_LOG_LIMIT {
        let start = entries.len() - CHANNEL_LOG_LIMIT;
        entries.drain(0..start);
    }
}

async fn run_message_loop(
    mut ws_sender: WsSender,
    mut ws_receiver: WsReceiver,
    mut cmd_rx: broadcast::Receiver<String>,
    mut stop_rx: tokio::sync::watch::Receiver<bool>,
    pending_responses: Arc<RwLock<HashMap<String, oneshot::Sender<OneBotApiResponse>>>>,
    event_tx: broadcast::Sender<Value>,
    connections: Arc<RwLock<HashMap<String, WsConnection>>>,
    channel_logs: Arc<RwLock<HashMap<String, Vec<ChannelLogEntry>>>>,
    channel_id: String,
    peer_addr_str: String,
    cancel: CancellationToken,
) {
    let mut disconnect_level = "info".to_string();
    let mut disconnect_message = format!("客户端断开: {}", peer_addr_str);
    loop {
        tokio::select! {
            _ = cancel.cancelled() => {
                let _ = ws_sender.send(Message::Close(None)).await;
                disconnect_level = "info".to_string();
                disconnect_message = format!("收到渠道取消信号: {}", peer_addr_str);
                break;
            }
            cmd = cmd_rx.recv() => {
                match cmd {
                    Ok(payload) => {
                        if ws_sender.send(Message::Text(payload.into())).await.is_err() {
                            disconnect_level = "warn".to_string();
                            disconnect_message = format!("向客户端发送消息失败: {}", peer_addr_str);
                            break;
                        }
                    }
                    Err(_) => {
                        disconnect_level = "warn".to_string();
                        disconnect_message = "命令广播通道已关闭".to_string();
                        break;
                    }
                }
            }
            changed = stop_rx.changed() => {
                match changed {
                    Ok(()) => {
                        if *stop_rx.borrow() {
                            let _ = ws_sender.send(Message::Close(None)).await;
                            disconnect_level = "info".to_string();
                            disconnect_message = format!("收到连接停止信号: {}", peer_addr_str);
                            break;
                        }
                    }
                    Err(_) => {
                        disconnect_level = "warn".to_string();
                        disconnect_message = "连接停止通道已关闭".to_string();
                        break;
                    }
                }
            }
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        if let Ok(value) = serde_json::from_str::<Value>(&text) {
                            if let Some(echo) = value.get("echo").and_then(Value::as_str) {
                                if let Ok(response) = serde_json::from_value::<OneBotApiResponse>(value.clone()) {
                                    if let Some(tx) = pending_responses.write().await.remove(echo) {
                                        let _ = tx.send(response);
                                    }
                                }
                            } else if value.get("post_type").is_some() {
                                let _ = event_tx.send(value);
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        let _ = ws_sender.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => {
                        break;
                    }
                    Some(Ok(_)) => {}
                    Some(Err(e)) => {
                        eprintln!("[远程IM][OneBot v11 WS] 接收错误: {}", e);
                        disconnect_level = "error".to_string();
                        disconnect_message = format!("接收错误: {}", e);
                        break;
                    }
                }
            }
        }
    }

    eprintln!("[远程IM][OneBot v11 WS] 渠道 {} 客户端断开: {}", channel_id, peer_addr_str);
    append_channel_log(&channel_logs, &channel_id, &disconnect_level, disconnect_message).await;
    connections.write().await.remove(&channel_id);
}
