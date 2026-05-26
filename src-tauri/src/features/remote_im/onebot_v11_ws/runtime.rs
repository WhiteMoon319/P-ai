impl OnebotV11WsManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_stop_senders: Arc::new(RwLock::new(HashMap::new())),
            channel_shutdowns: Arc::new(RwLock::new(HashMap::new())),
            channel_logs: Arc::new(RwLock::new(HashMap::new())),
            listen_addrs: Arc::new(RwLock::new(HashMap::new())),
            channel_tasks: Arc::new(RwLock::new(HashMap::new())),
            channel_runtimes: Arc::new(RwLock::new(HashMap::new())),
            event_consumer_stop_senders: Arc::new(RwLock::new(HashMap::new())),
            event_consumer_tasks: Arc::new(RwLock::new(HashMap::new())),
            lifecycle_locks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    async fn channel_lifecycle_guard(
        &self,
        channel_id: &str,
    ) -> tokio::sync::OwnedMutexGuard<()> {
        let lock = {
            let mut locks = self.lifecycle_locks.lock().await;
            locks
                .entry(channel_id.to_string())
                .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
                .clone()
        };
        lock.lock_owned().await
    }

    /// 添加日志
    pub(crate) async fn add_log(&self, channel_id: &str, level: &str, message: &str) {
        let entry = ChannelLogEntry {
            timestamp: Utc::now(),
            level: level.to_string(),
            message: message.to_string(),
        };
        let mut logs = self.channel_logs.write().await;
        let channel_logs = logs.entry(channel_id.to_string()).or_insert_with(Vec::new);
        channel_logs.push(entry);
        if channel_logs.len() > CHANNEL_LOG_LIMIT {
            let start = channel_logs.len() - CHANNEL_LOG_LIMIT;
            channel_logs.drain(0..start);
        }
    }

    /// 停止单个渠道的 WebSocket 服务器
    pub async fn stop_channel(&self, channel_id: &str) -> Result<(), String> {
        let _guard = self.channel_lifecycle_guard(channel_id).await;
        self.stop_channel_inner(channel_id).await
    }

    async fn stop_onebot_connection_inner(&self, channel_id: &str) -> Result<(), String> {
        let stop_sender = {
            self.connection_stop_senders
                .write()
                .await
                .remove(channel_id)
        };
        if let Some(tx) = stop_sender {
            let _ = tx.send(true);
        }
        let stopped = tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let has_connection = self.connections.read().await.contains_key(channel_id);
                let has_stop_sender = self
                    .connection_stop_senders
                    .read()
                    .await
                    .contains_key(channel_id);
                if !has_connection && !has_stop_sender {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(25)).await;
            }
        })
        .await;
        if stopped.is_err() {
            eprintln!(
                "[远程IM][OneBot v11 WS] 停止渠道连接超时，连接可能仍在退出中: {}",
                channel_id
            );
        }
        self.connections.write().await.remove(channel_id);
        Ok(())
    }

    async fn stop_channel_runtime_tasks_inner(&self, channel_id: &str) {
        let runtime = self.channel_runtimes.write().await.remove(channel_id);
        let Some(runtime) = runtime else {
            return;
        };
        eprintln!(
            "[远程IM][OneBot v11 WS] 开始等待渠道连接任务组退出: channel_id={}",
            channel_id
        );
        runtime.cancel.cancel();
        runtime.tasks.close();
        runtime.tasks.wait().await;
        eprintln!(
            "[远程IM][OneBot v11 WS] 渠道连接任务组已退出: channel_id={}",
            channel_id
        );
    }

    async fn force_clear_channel_runtime_state(&self, channel_id: &str) {
        self.connections.write().await.remove(channel_id);
        self.connection_stop_senders.write().await.remove(channel_id);
        self.listen_addrs.write().await.remove(channel_id);
        self.channel_shutdowns.write().await.remove(channel_id);
        self.channel_tasks.write().await.remove(channel_id);
        self.channel_runtimes.write().await.remove(channel_id);
        self.event_consumer_stop_senders.write().await.remove(channel_id);
        self.event_consumer_tasks.write().await.remove(channel_id);
    }

    async fn channel_runtime_state_is_clear(&self, channel_id: &str) -> bool {
        !self.connections.read().await.contains_key(channel_id)
            && !self
                .connection_stop_senders
                .read()
                .await
                .contains_key(channel_id)
            && !self.channel_shutdowns.read().await.contains_key(channel_id)
            && !self.listen_addrs.read().await.contains_key(channel_id)
            && !self.channel_tasks.read().await.contains_key(channel_id)
            && !self.channel_runtimes.read().await.contains_key(channel_id)
            && !self
                .event_consumer_stop_senders
                .read()
                .await
                .contains_key(channel_id)
            && !self
                .event_consumer_tasks
                .read()
                .await
                .contains_key(channel_id)
    }

    async fn stop_channel_inner(&self, channel_id: &str) -> Result<(), String> {
        let started_at = std::time::Instant::now();
        eprintln!(
            "[远程IM][OneBot v11 WS] 开始强制停止渠道: channel_id={}",
            channel_id
        );
        eprintln!(
            "[远程IM][OneBot v11 WS] 停止阶段=事件消费器: channel_id={}",
            channel_id
        );
        if let Err(err) = self.stop_event_consumer_inner(channel_id).await {
            eprintln!(
                "[远程IM][OneBot v11 WS] 停止事件消费器失败，继续强制清理渠道: channel_id={}, error={}",
                channel_id, err
            );
        }
        eprintln!(
            "[远程IM][OneBot v11 WS] 停止阶段=活动连接: channel_id={}",
            channel_id
        );
        if let Err(err) = self.stop_onebot_connection_inner(channel_id).await {
            eprintln!(
                "[远程IM][OneBot v11 WS] 停止活动连接失败，继续强制清理渠道: channel_id={}, error={}",
                channel_id, err
            );
        }
        eprintln!(
            "[远程IM][OneBot v11 WS] 停止阶段=accept循环: channel_id={}",
            channel_id
        );
        // 发送关闭信号给该渠道的 accept 循环
        let shutdown_tx = { self.channel_shutdowns.write().await.remove(channel_id) };
        if let Some(tx) = shutdown_tx {
            let _ = tx.send(());
        }
        // 等待旧的 accept 循环任务退出，确保端口被释放
        let handle = { self.channel_tasks.write().await.remove(channel_id) };
        if let Some(handle) = handle {
            let mut handle = handle;
            match tokio::time::timeout(Duration::from_secs(8), &mut handle).await {
                Ok(join_result) => {
                    if let Err(err) = join_result {
                        eprintln!(
                            "[远程IM][OneBot v11 WS] 停止渠道任务失败，继续清理运行态: channel_id={}, error={}",
                            channel_id, err
                        );
                    }
                }
                Err(_) => {
                    handle.abort();
                    let _ = handle.await;
                    eprintln!(
                        "[远程IM][OneBot v11 WS] 停止渠道超时，已强制中止 accept 循环: {}",
                        channel_id
                    );
                }
            }
        }
        eprintln!(
            "[远程IM][OneBot v11 WS] 停止阶段=连接任务组: channel_id={}",
            channel_id
        );
        self.stop_channel_runtime_tasks_inner(channel_id).await;
        eprintln!(
            "[远程IM][OneBot v11 WS] 停止阶段=清理运行态: channel_id={}",
            channel_id
        );
        // 清除连接和监听地址
        self.connections.write().await.remove(channel_id);
        self.connection_stop_senders.write().await.remove(channel_id);
        self.listen_addrs.write().await.remove(channel_id);
        self.force_clear_channel_runtime_state(channel_id).await;
        while !self.channel_runtime_state_is_clear(channel_id).await {
            eprintln!(
                "[远程IM][OneBot v11 WS] 渠道运行态仍未清空，继续强制清理: channel_id={}",
                channel_id
            );
            self.force_clear_channel_runtime_state(channel_id).await;
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        self.add_log(channel_id, "info", "渠道服务器已停止").await;
        eprintln!(
            "[远程IM][OneBot v11 WS] 渠道已强制停止完成: channel_id={}, duration_ms={}",
            channel_id,
            started_at.elapsed().as_millis()
        );
        Ok(())
    }

    /// 依据渠道配置原子地收敛运行态：先停，再按 enabled/platform 决定是否启动
    pub(crate) async fn reconcile_channel_runtime(
        &self,
        channel: &RemoteImChannelConfig,
    ) -> Result<(), String> {
        let _guard = self.channel_lifecycle_guard(&channel.id).await;
        self.stop_channel_inner(&channel.id).await?;
        if channel.enabled && channel.platform == RemoteImPlatform::OnebotV11 {
            let credentials = OnebotV11WsCredentials::from_credentials(&channel.credentials);
            self.start_inner(channel.id.clone(), credentials).await?;
        }
        Ok(())
    }

    /// 获取渠道连接状态
    pub async fn get_connection_status(&self, channel_id: &str) -> ChannelConnectionStatus {
        let connections = self.connections.read().await;
        let listen_addrs = self.listen_addrs.read().await;

        if let Some(conn) = connections.get(channel_id) {
            ChannelConnectionStatus {
                channel_id: channel_id.to_string(),
                connected: true,
                peer_addr: conn.peer_addr.clone(),
                connected_at: conn.connected_at,
                listen_addr: listen_addrs.get(channel_id).cloned().unwrap_or_default(),
                status_text: None,
                last_error: None,
                account_id: None,
                base_url: None,
                login_session_key: None,
                qrcode_url: None,
            }
        } else {
            ChannelConnectionStatus {
                channel_id: channel_id.to_string(),
                connected: false,
                peer_addr: None,
                connected_at: None,
                listen_addr: listen_addrs.get(channel_id).cloned().unwrap_or_default(),
                status_text: None,
                last_error: None,
                account_id: None,
                base_url: None,
                login_session_key: None,
                qrcode_url: None,
            }
        }
    }

    /// 获取渠道日志
    pub async fn get_logs(&self, channel_id: &str) -> Vec<ChannelLogEntry> {
        let logs = self.channel_logs.read().await;
        logs.get(channel_id).cloned().unwrap_or_default()
    }

    /// 启动 WebSocket 服务器
    pub async fn start(
        &self,
        channel_id: String,
        credentials: OnebotV11WsCredentials,
    ) -> Result<(), String> {
        let _guard = self.channel_lifecycle_guard(&channel_id).await;
        self.start_inner(channel_id, credentials).await
    }

    async fn start_inner(
        &self,
        channel_id: String,
        credentials: OnebotV11WsCredentials,
    ) -> Result<(), String> {
        self.stop_channel_inner(&channel_id).await?;
        let addr: SocketAddr = format!("{}:{}", credentials.ws_host, credentials.ws_port)
            .parse()
            .map_err(|e| format!("无效地址: {}", e))?;

        let listener = TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("绑定端口失败: {}", e))?;

        let listen_addr = addr.to_string();
        eprintln!("[远程IM][OneBot v11 WS] 渠道 {} 开始监听 {}", channel_id, listen_addr);

        // 记录监听地址
        self.listen_addrs.write().await.insert(channel_id.clone(), listen_addr.clone());
        self.add_log(&channel_id, "info", &format!("服务器启动，监听 {}", listen_addr)).await;

        let connections = self.connections.clone();
        let connection_stop_senders = self.connection_stop_senders.clone();
        let channel_logs = self.channel_logs.clone();
        let active_connection_gate = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let channel_runtime = OnebotChannelRuntime {
            cancel: CancellationToken::new(),
            tasks: TaskTracker::new(),
        };
        self.channel_runtimes
            .write()
            .await
            .insert(channel_id.clone(), channel_runtime.clone());

        // 创建 per-channel 的关闭信号
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        self.channel_shutdowns.write().await.insert(channel_id.clone(), shutdown_tx.clone());
        let mut shutdown_rx = shutdown_tx.subscribe();

        // 创建连接通道
        let (conn_tx, _) = broadcast::channel::<String>(64);
        let (event_tx, _) = broadcast::channel::<Value>(256);
        let pending_responses = Arc::new(RwLock::new(HashMap::<String, oneshot::Sender<OneBotApiResponse>>::new()));

        let saved_channel_id = channel_id.clone();
        let task_handle = tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = channel_runtime.cancel.cancelled() => {
                        eprintln!("[远程IM][OneBot v11 WS] 渠道 {} 收到任务取消信号", channel_id);
                        break;
                    }
                    _ = shutdown_rx.recv() => {
                        eprintln!("[远程IM][OneBot v11 WS] 渠道 {} 收到关闭信号", channel_id);
                        let mut logs = channel_logs.write().await;
                        if let Some(l) = logs.get_mut(&channel_id) {
                            l.push(ChannelLogEntry {
                                timestamp: Utc::now(),
                                level: "info".to_string(),
                                message: "服务器关闭".to_string(),
                            });
                        }
                        break;
                    }
                    accept = listener.accept() => {
                        match accept {
                            Ok((stream, peer_addr)) => {
                                let channel_id = channel_id.clone();
                                let expected_token = credentials.ws_token.clone();
                                let conn_tx = conn_tx.clone();
                                let pending_responses = pending_responses.clone();
                                let event_tx = event_tx.clone();
                                let connections = connections.clone();
                                let connection_stop_senders = connection_stop_senders.clone();
                                let channel_logs = channel_logs.clone();
                                let active_connection_gate = active_connection_gate.clone();
                                let channel_id_for_task = channel_id.clone();
                                let connection_cancel = channel_runtime.cancel.child_token();
                                channel_runtime.tasks.spawn(async move {
                                    Self::handle_connection(
                                        stream,
                                        peer_addr,
                                        channel_id_for_task,
                                        expected_token,
                                        conn_tx,
                                        pending_responses,
                                        event_tx,
                                        connections,
                                        connection_stop_senders,
                                        channel_logs,
                                        active_connection_gate,
                                        connection_cancel,
                                    )
                                    .await;
                                });
                            }
                            Err(e) => {
                                eprintln!("[远程IM][OneBot v11 WS] 接受连接失败: {}", e);
                                let mut logs = channel_logs.write().await;
                                if let Some(l) = logs.get_mut(&channel_id) {
                                    l.push(ChannelLogEntry {
                                        timestamp: Utc::now(),
                                        level: "error".to_string(),
                                        message: format!("接受连接失败: {}", e),
                                    });
                                }
                            }
                        }
                    }
                }
            }
            // listener 在此处 drop，释放端口绑定
        });
        self.channel_tasks.write().await.insert(saved_channel_id, task_handle);

        Ok(())
    }

    async fn handle_connection(
        stream: tokio::net::TcpStream,
        peer_addr: SocketAddr,
        channel_id: String,
        expected_token: Option<String>,
        conn_tx: broadcast::Sender<String>,
        pending_responses: Arc<RwLock<HashMap<String, oneshot::Sender<OneBotApiResponse>>>>,
        event_tx: broadcast::Sender<Value>,
        connections: Arc<RwLock<HashMap<String, WsConnection>>>,
        connection_stop_senders: Arc<RwLock<HashMap<String, tokio::sync::watch::Sender<bool>>>>,
        channel_logs: Arc<RwLock<HashMap<String, Vec<ChannelLogEntry>>>>,
        active_connection_gate: Arc<std::sync::atomic::AtomicBool>,
        cancel: CancellationToken,
    ) {
        let expected_for_closure = expected_token.clone();
        let ws_result = tokio::select! {
            _ = cancel.cancelled() => {
                return;
            }
            result = tokio::time::timeout(
                Duration::from_secs(NAPCAT_WS_HANDSHAKE_TIMEOUT_SECS),
                accept_hdr_async(stream, move |req: &Request, response: Response| {
                    if let Err(mut err_response) = validate_ws_token(req, expected_for_closure.as_deref()) {
                        *err_response.headers_mut() = response.headers().clone();
                        return Err(err_response);
                    }
                    Ok(response)
                }),
            ) => result,
        };

        let ws_stream = match ws_result {
            Ok(Ok(ws)) => ws,
            Ok(Err(e)) => {
                eprintln!("[远程IM][OneBot v11 WS] WebSocket 握手失败 {}: {}", peer_addr, e);
                append_channel_log(
                    &channel_logs,
                    &channel_id,
                    "error",
                    format!("WebSocket 握手失败 {}: {}", peer_addr, e),
                )
                .await;
                return;
            }
            Err(_) => {
                eprintln!(
                    "[远程IM][OneBot v11 WS] WebSocket 握手超时 {}: timeout={}s",
                    peer_addr, NAPCAT_WS_HANDSHAKE_TIMEOUT_SECS
                );
                append_channel_log(
                    &channel_logs,
                    &channel_id,
                    "warn",
                    format!(
                        "WebSocket 握手超时 {}: timeout={}s",
                        peer_addr, NAPCAT_WS_HANDSHAKE_TIMEOUT_SECS
                    ),
                )
                .await;
                return;
            }
        };

        let mut ws_stream = ws_stream;
        if active_connection_gate
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            eprintln!(
                "[远程IM][OneBot v11 WS] 渠道 {} 检测到已有连接，正在尝试替换: {}",
                channel_id, peer_addr
            );
            let existing_sender = {
                connection_stop_senders
                    .read()
                    .await
                    .get(&channel_id)
                    .cloned()
            };
            if let Some(sender) = existing_sender {
                let _ = sender.send(true);
            }
            let replaced = tokio::time::timeout(
                Duration::from_millis(NAPCAT_ACTIVE_CONNECTION_REPLACE_TIMEOUT_MS),
                async {
                    loop {
                        if active_connection_gate
                            .compare_exchange(
                                false,
                                true,
                                std::sync::atomic::Ordering::SeqCst,
                                std::sync::atomic::Ordering::SeqCst,
                            )
                            .is_ok()
                        {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(25)).await;
                    }
                },
            )
            .await;
            if replaced.is_err() {
                eprintln!(
                    "[远程IM][OneBot v11 WS] 渠道 {} 替换旧连接超时，拒绝新连接: {}",
                    channel_id, peer_addr
                );
                append_channel_log(
                    &channel_logs,
                    &channel_id,
                    "warn",
                    format!("替换旧连接超时，拒绝新连接: {}", peer_addr),
                )
                .await;
                let _ = ws_stream.close(None).await;
                return;
            }
            append_channel_log(
                &channel_logs,
                &channel_id,
                "info",
                format!("新连接已接管旧连接: {}", peer_addr),
            )
            .await;
        }

        let (connection_stop_tx, connection_stop_rx) = tokio::sync::watch::channel(false);
        connection_stop_senders
            .write()
            .await
            .insert(channel_id.clone(), connection_stop_tx);

        let peer_addr_str = peer_addr.to_string();
        eprintln!("[远程IM][OneBot v11 WS] 渠道 {} 客户端已连接: {}", channel_id, peer_addr_str);

        append_channel_log(
            &channel_logs,
            &channel_id,
            "info",
            format!("客户端已连接: {}", peer_addr_str),
        )
        .await;

        // 更新连接状态
        {
            let conn = WsConnection {
                tx: conn_tx.clone(),
                pending_responses: pending_responses.clone(),
                event_tx: event_tx.clone(),
                peer_addr: Some(peer_addr_str.clone()),
                connected_at: Some(Utc::now()),
            };
            connections.write().await.insert(channel_id.clone(), conn);
        }

        let (ws_sender, ws_receiver) = ws_stream.split();
        let cmd_rx = conn_tx.subscribe();
        let channel_id_for_loop = channel_id.clone();
        run_message_loop(
            ws_sender,
            ws_receiver,
            cmd_rx,
            connection_stop_rx,
            pending_responses,
            event_tx,
            connections,
            channel_logs,
            channel_id_for_loop,
            peer_addr_str,
            cancel,
        )
        .await;
        connection_stop_senders.write().await.remove(&channel_id);
        active_connection_gate.store(false, std::sync::atomic::Ordering::SeqCst);
    }
}
