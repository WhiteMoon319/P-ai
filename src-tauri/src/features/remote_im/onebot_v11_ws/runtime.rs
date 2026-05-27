impl OnebotV11WsManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            connection_stop_senders: Arc::new(RwLock::new(HashMap::new())),
            channel_shutdowns: Arc::new(RwLock::new(HashMap::new())),
            channel_event_senders: Arc::new(RwLock::new(HashMap::new())),
            channel_logs: Arc::new(RwLock::new(HashMap::new())),
            listen_addrs: Arc::new(RwLock::new(HashMap::new())),
            channel_status_texts: Arc::new(RwLock::new(HashMap::new())),
            channel_last_errors: Arc::new(RwLock::new(HashMap::new())),
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
        self.channel_status_texts.write().await.remove(channel_id);
        self.channel_last_errors.write().await.remove(channel_id);
        self.channel_shutdowns.write().await.remove(channel_id);
        self.channel_event_senders.write().await.remove(channel_id);
        self.channel_tasks.write().await.remove(channel_id);
        self.channel_runtimes.write().await.remove(channel_id);
        self.event_consumer_stop_senders.write().await.remove(channel_id);
        self.event_consumer_tasks.write().await.remove(channel_id);
    }

    async fn clear_channel_runtime_state_keep_diagnostics(&self, channel_id: &str) {
        self.connections.write().await.remove(channel_id);
        self.connection_stop_senders.write().await.remove(channel_id);
        self.channel_shutdowns.write().await.remove(channel_id);
        self.channel_event_senders.write().await.remove(channel_id);
        self.channel_tasks.write().await.remove(channel_id);
        self.channel_runtimes.write().await.remove(channel_id);
        self.event_consumer_stop_senders.write().await.remove(channel_id);
        self.event_consumer_tasks.write().await.remove(channel_id);
    }

    async fn channel_server_is_listening_at(&self, channel_id: &str, addr: SocketAddr) -> bool {
        let expected_addr = addr.to_string();
        let listen_addr_matches = self
            .listen_addrs
            .read()
            .await
            .get(channel_id)
            .map(|value| value == &expected_addr)
            .unwrap_or(false);
        if !listen_addr_matches {
            return false;
        }
        let has_live_accept_task = self
            .channel_tasks
            .read()
            .await
            .get(channel_id)
            .map(|handle| !handle.is_finished())
            .unwrap_or(false);
        has_live_accept_task && self.channel_runtimes.read().await.contains_key(channel_id)
    }

    async fn channel_start_is_in_progress_at(&self, channel_id: &str, addr: SocketAddr) -> bool {
        let expected_addr = addr.to_string();
        let listen_addr_matches = self
            .listen_addrs
            .read()
            .await
            .get(channel_id)
            .map(|value| value == &expected_addr)
            .unwrap_or(false);
        if !listen_addr_matches {
            return false;
        }
        let status_is_binding = self
            .channel_status_texts
            .read()
            .await
            .get(channel_id)
            .map(|value| value == "binding" || value == "binding_retry")
            .unwrap_or(false);
        status_is_binding && self.channel_runtimes.read().await.contains_key(channel_id)
    }

    async fn channel_runtime_matches(&self, channel_id: &str, runtime_id: &str) -> bool {
        self.channel_runtimes
            .read()
            .await
            .get(channel_id)
            .map(|runtime| runtime.id == runtime_id)
            .unwrap_or(false)
    }

    async fn channel_runtime_state_is_clear(&self, channel_id: &str) -> bool {
        !self.connections.read().await.contains_key(channel_id)
            && !self
                .connection_stop_senders
                .read()
                .await
                .contains_key(channel_id)
            && !self.channel_shutdowns.read().await.contains_key(channel_id)
            && !self
                .channel_event_senders
                .read()
                .await
                .contains_key(channel_id)
            && !self.listen_addrs.read().await.contains_key(channel_id)
            && !self
                .channel_status_texts
                .read()
                .await
                .contains_key(channel_id)
            && !self
                .channel_last_errors
                .read()
                .await
                .contains_key(channel_id)
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
        self.channel_status_texts.write().await.remove(channel_id);
        self.channel_last_errors.write().await.remove(channel_id);
        self.channel_event_senders.write().await.remove(channel_id);
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
        let start_plan = {
            let _guard = self.channel_lifecycle_guard(&channel.id).await;
            self.stop_channel_inner(&channel.id).await?;
            if !channel.enabled {
                self.channel_status_texts
                    .write()
                    .await
                    .insert(channel.id.clone(), "disabled".to_string());
                self.channel_last_errors.write().await.remove(&channel.id);
                self.add_log(&channel.id, "info", "渠道已禁用，跳过启动").await;
                return Ok(());
            }
            if channel.platform != RemoteImPlatform::OnebotV11 {
                self.channel_status_texts
                    .write()
                    .await
                    .insert(channel.id.clone(), "unsupported_platform".to_string());
                self.add_log(&channel.id, "warn", "渠道不是 OneBot v11，跳过启动").await;
                return Ok(());
            }
            let credentials = OnebotV11WsCredentials::from_credentials(&channel.credentials);
            let addr = onebot_listen_addr_from_credentials(&credentials)?;
            let runtime = self.prepare_start_after_stop_at(&channel.id, addr).await;
            (credentials, addr, runtime)
        };
        self.finish_start_after_prepare(
            channel.id.clone(),
            start_plan.0,
            start_plan.1,
            start_plan.2,
        )
        .await
    }

    /// 获取渠道连接状态
    pub async fn get_connection_status(&self, channel_id: &str) -> ChannelConnectionStatus {
        let connections = self.connections.read().await;
        let listen_addrs = self.listen_addrs.read().await;
        let status_texts = self.channel_status_texts.read().await;
        let last_errors = self.channel_last_errors.read().await;
        let status_text = status_texts
            .get(channel_id)
            .filter(|value| !value.trim().is_empty())
            .cloned();
        let last_error = last_errors
            .get(channel_id)
            .filter(|value| !value.trim().is_empty())
            .cloned();

        if let Some(conn) = connections.get(channel_id) {
            ChannelConnectionStatus {
                channel_id: channel_id.to_string(),
                connected: true,
                peer_addr: conn.peer_addr.clone(),
                connected_at: conn.connected_at,
                listen_addr: listen_addrs.get(channel_id).cloned().unwrap_or_default(),
                status_text,
                last_error,
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
                status_text,
                last_error,
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
        self.start_inner(channel_id, credentials).await
    }

    async fn start_inner(
        &self,
        channel_id: String,
        credentials: OnebotV11WsCredentials,
    ) -> Result<(), String> {
        let addr = onebot_listen_addr_from_credentials(&credentials)?;
        let runtime = {
            let _guard = self.channel_lifecycle_guard(&channel_id).await;
            if self.channel_server_is_listening_at(&channel_id, addr).await {
                self.channel_status_texts
                    .write()
                    .await
                    .insert(channel_id.clone(), "listening".to_string());
                self.channel_last_errors.write().await.remove(&channel_id);
                self.add_log(&channel_id, "info", "服务器已在监听，跳过重复启动").await;
                return Ok(());
            }
            if self.channel_start_is_in_progress_at(&channel_id, addr).await {
                self.add_log(&channel_id, "info", "服务器启动流程已存在，跳过重复启动").await;
                return Ok(());
            }
            self.stop_channel_inner(&channel_id).await?;
            self.prepare_start_after_stop_at(&channel_id, addr).await
        };
        self.finish_start_after_prepare(channel_id, credentials, addr, runtime)
            .await
    }

    async fn prepare_start_after_stop_at(
        &self,
        channel_id: &str,
        addr: SocketAddr,
    ) -> OnebotChannelRuntime {
        let channel_runtime = OnebotChannelRuntime {
            id: Uuid::new_v4().to_string(),
            cancel: CancellationToken::new(),
            tasks: TaskTracker::new(),
        };
        self.channel_runtimes
            .write()
            .await
            .insert(channel_id.to_string(), channel_runtime.clone());

        self.listen_addrs
            .write()
            .await
            .insert(channel_id.to_string(), addr.to_string());
        self.channel_status_texts
            .write()
            .await
            .insert(channel_id.to_string(), "binding".to_string());
        self.channel_last_errors.write().await.remove(channel_id);
        channel_runtime
    }

    async fn finish_start_after_prepare(
        &self,
        channel_id: String,
        credentials: OnebotV11WsCredentials,
        addr: SocketAddr,
        channel_runtime: OnebotChannelRuntime,
    ) -> Result<(), String> {
        let listener_result = bind_onebot_listener_with_retry(
            &channel_id,
            addr,
            self.channel_status_texts.clone(),
            self.channel_last_errors.clone(),
            channel_runtime.cancel.clone(),
        )
        .await;
        let listener = match listener_result {
            Ok(listener) => listener,
            Err(err) => {
                let was_cancelled = err.kind() == std::io::ErrorKind::Interrupted
                    || !self.channel_runtime_matches(&channel_id, &channel_runtime.id).await;
                if was_cancelled {
                    return Err(format!("绑定端口已取消: {}", err));
                }
                let message = describe_onebot_bind_error(addr, &err);
                self.clear_channel_runtime_state_keep_diagnostics(&channel_id).await;
                self.listen_addrs
                    .write()
                    .await
                    .insert(channel_id.clone(), addr.to_string());
                self.channel_status_texts
                    .write()
                    .await
                    .insert(channel_id.clone(), "bind_failed".to_string());
                self.channel_last_errors
                    .write()
                    .await
                    .insert(channel_id.clone(), message.clone());
                self.add_log(&channel_id, "error", &message).await;
                return Err(format!("绑定端口失败: {}", message));
            }
        };
        let _guard = self.channel_lifecycle_guard(&channel_id).await;
        if !self
            .channel_runtime_matches(&channel_id, &channel_runtime.id)
            .await
        {
            self.add_log(&channel_id, "info", "服务器启动已被新的渠道状态取代，丢弃本次监听").await;
            return Ok(());
        }
        self.channel_status_texts
            .write()
            .await
            .insert(channel_id.clone(), "listening".to_string());
        self.channel_last_errors.write().await.remove(&channel_id);

        let listen_addr = addr.to_string();
        eprintln!("[远程IM][OneBot v11 WS] 渠道 {} 开始监听 {}", channel_id, listen_addr);

        // 记录监听地址
        self.listen_addrs.write().await.insert(channel_id.clone(), listen_addr.clone());
        self.add_log(&channel_id, "info", &format!("服务器启动，监听 {}", listen_addr)).await;

        let connections = self.connections.clone();
        let connection_stop_senders = self.connection_stop_senders.clone();
        let channel_logs = self.channel_logs.clone();
        let active_connection_gate = Arc::new(std::sync::atomic::AtomicBool::new(false));

        // 创建 per-channel 的关闭信号
        let (shutdown_tx, _) = broadcast::channel::<()>(1);
        self.channel_shutdowns.write().await.insert(channel_id.clone(), shutdown_tx.clone());
        let mut shutdown_rx = shutdown_tx.subscribe();

        // 创建连接通道
        let (conn_tx, _) = broadcast::channel::<String>(64);
        let (event_tx, _) = broadcast::channel::<Value>(256);
        self.channel_event_senders
            .write()
            .await
            .insert(channel_id.clone(), event_tx.clone());
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

fn bind_onebot_listener(addr: SocketAddr) -> std::io::Result<TcpListener> {
    let socket = Socket::new(Domain::for_address(addr), Type::STREAM, Some(Protocol::TCP))?;
    socket.set_reuse_address(true)?;
    socket.bind(&addr.into())?;
    socket.listen(128)?;
    socket.set_nonblocking(true)?;
    let std_listener: std::net::TcpListener = socket.into();
    TcpListener::from_std(std_listener)
}

fn onebot_listen_addr_from_credentials(
    credentials: &OnebotV11WsCredentials,
) -> Result<SocketAddr, String> {
    format!("{}:{}", credentials.ws_host, credentials.ws_port)
        .parse()
        .map_err(|err| format!("无效地址: {}", err))
}

fn describe_onebot_bind_error(addr: SocketAddr, err: &std::io::Error) -> String {
    if err.kind() == std::io::ErrorKind::PermissionDenied && err.raw_os_error() == Some(10013) {
        return format!(
            "监听地址不可绑定: addr={}，error={}。Windows 返回 10013，常见原因是同一 PAI 旧监听尚未释放、重复启动流程抢占，或其他进程/系统策略占用该端口。",
            addr,
            err
        );
    }
    if err.kind() == std::io::ErrorKind::AddrInUse {
        return format!("监听地址仍被占用: addr={}，error={}", addr, err);
    }
    format!("监听地址绑定失败: addr={}，error={}", addr, err)
}

async fn bind_onebot_listener_with_retry(
    channel_id: &str,
    addr: SocketAddr,
    channel_status_texts: Arc<RwLock<HashMap<String, String>>>,
    channel_last_errors: Arc<RwLock<HashMap<String, String>>>,
    cancel: CancellationToken,
) -> std::io::Result<TcpListener> {
    let started_at = std::time::Instant::now();
    let mut attempts = 0_u32;
    loop {
        attempts = attempts.saturating_add(1);
        match bind_onebot_listener(addr) {
            Ok(listener) => {
                if attempts > 1 {
                    eprintln!(
                        "[远程IM][OneBot v11 WS] 固定端口重试绑定成功: channel_id={}, addr={}, attempts={}, duration_ms={}",
                        channel_id,
                        addr,
                        attempts,
                        started_at.elapsed().as_millis()
                    );
                }
                return Ok(listener);
            }
            Err(err)
                if err.kind() == std::io::ErrorKind::AddrInUse
                    && started_at.elapsed() < Duration::from_secs(NAPCAT_BIND_RETRY_TIMEOUT_SECS) =>
            {
                let message = format!(
                    "固定端口仍被占用，{} 秒后重试: addr={}, attempts={}, elapsed_ms={}, error={}",
                    NAPCAT_BIND_RETRY_INTERVAL_MS / 1000,
                    addr,
                    attempts,
                    started_at.elapsed().as_millis(),
                    err
                );
                eprintln!("[远程IM][OneBot v11 WS] {}: channel_id={}", message, channel_id);
                channel_status_texts
                    .write()
                    .await
                    .insert(channel_id.to_string(), "binding_retry".to_string());
                channel_last_errors
                    .write()
                    .await
                    .insert(channel_id.to_string(), message);
                tokio::select! {
                    _ = cancel.cancelled() => {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::Interrupted,
                            "渠道已停止，取消固定端口绑定重试",
                        ));
                    }
                    _ = tokio::time::sleep(Duration::from_millis(NAPCAT_BIND_RETRY_INTERVAL_MS)) => {}
                }
            }
            Err(err) => return Err(err),
        }
    }
}
