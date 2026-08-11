#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdeContextReference {
    pub(crate) id: String,
    pub(crate) file_path: String,
    #[serde(default)]
    pub(crate) start_line: Option<u32>,
    #[serde(default)]
    pub(crate) end_line: Option<u32>,
    pub(crate) content: String,
    #[serde(default)]
    pub(crate) language_id: Option<String>,
    pub(crate) source: String,
    pub(crate) captured_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdeContextSnapshot {
    pub(crate) client_id: String,
    pub(crate) editor: String,
    pub(crate) workspace_roots: Vec<String>,
    pub(crate) references: Vec<IdeContextReference>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecentLlmRoundLogs {
    pub(crate) pipeline_logs: std::collections::VecDeque<LlmRoundLogEntry>,
    pub(crate) other_logs: std::collections::VecDeque<LlmRoundLogEntry>,
}

/// 非 tauri 应用句柄抽象：桌面端包装 tauri::AppHandle（事件广播/路径解析），
/// Android 原生模式不依赖 tauri crate（事件走 pollEvents 队列旁路）。
/// 这是 tauri 剥离的最后一道抽象层：所有共享代码只依赖本类型而非 tauri::AppHandle。
#[derive(Clone, Default)]
pub(crate) struct NativeAppHandle {
    #[cfg(not(target_os = "android"))]
    pub(crate) inner: Option<tauri::AppHandle>,
    #[cfg(target_os = "android")]
    pub(crate) _android: std::marker::PhantomData<()>,
}

impl NativeAppHandle {

    #[cfg(target_os = "android")]
    pub(crate) fn noop() -> Self {
        Self {
            _android: std::marker::PhantomData,
        }
    }

    /// 事件广播：桌面端转发 tauri NativeAppHandle.emit，Android 原生模式为空操作
    /// （事件统一由 dispatch_assistant_delta_to_active_view 旁路 push 原生队列）。
    pub(crate) fn emit<S: serde::Serialize + Clone>(
        &self,
        event: &str,
        payload: S,
    ) -> Result<(), String> {
        #[cfg(target_os = "android")]
        {
            let _ = (event, payload);
            Ok(())
        }
    }

    /// 访问 tauri 通知插件（仅桌面端有效；Android 通知走 pollEvents 原生队列）。

    /// 按 label 查找 Webview 窗口（仅桌面端；Android 无窗口概念）。

    /// 访问 tauri asset resolver（桌面 Web 侧边栏静态资源服务）。

    /// Android 占位：无 asset resolver（桌面 Web 侧边栏静态资源在 Android 不可用）。
    #[cfg(target_os = "android")]
    pub(crate) fn asset_resolver(&self) -> Result<(), String> {
        Err("Android 原生模式不支持 asset resolver".to_string())
    }
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) app_handle: Arc<Mutex<Option<NativeAppHandle>>>,
    pub(crate) config_path: PathBuf,
    pub(crate) data_path: PathBuf,
    pub(crate) llm_workspace_path: PathBuf,
    pub(crate) shared_http_client: reqwest::Client,
    pub(crate) terminal_shell: TerminalShellProfile,
    pub(crate) terminal_shell_candidates: Vec<TerminalShellProfile>,
    pub(crate) conversation_lock: Arc<ConversationDomainLock>,
    pub(crate) memory_lock: Arc<Mutex<()>>,
    pub(crate) cached_config: Arc<Mutex<Option<AppConfig>>>,
    pub(crate) cached_config_mtime: Arc<Mutex<Option<std::time::SystemTime>>>,
    pub(crate) cached_agents: Arc<Mutex<Option<Vec<AgentProfile>>>>,
    pub(crate) cached_agents_mtime: Arc<Mutex<Option<std::time::SystemTime>>>,
    pub(crate) cached_runtime_state: Arc<Mutex<Option<RuntimeStateFile>>>,
    pub(crate) cached_runtime_state_mtime: Arc<Mutex<Option<std::time::SystemTime>>>,
    pub(crate) cached_chat_index: Arc<Mutex<Option<ChatIndexFile>>>,
    pub(crate) cached_conversation_metadata:
        Arc<Mutex<std::collections::HashMap<String, message_store::ConversationShardMeta>>>,
    pub(crate) cached_conversation_field_metadata_ids:
        Arc<Mutex<std::collections::HashSet<String>>>,
    pub(crate) cached_conversation_mtimes:
        Arc<Mutex<std::collections::HashMap<String, Option<std::time::SystemTime>>>>,
    pub(crate) cached_app_data: Arc<Mutex<Option<AppData>>>,
    pub(crate) cached_app_data_signature: Arc<Mutex<Option<AppDataCacheSignature>>>,
    pub(crate) cached_app_data_dirty: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) app_data_persist_pending: Arc<Mutex<Option<PendingAppDataPersist>>>,
    pub(crate) app_data_persist_notify: Arc<tokio::sync::Notify>,
    pub(crate) app_data_persist_started: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) app_data_persist_latest_seq: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) conversation_persist_pending: Arc<Mutex<Option<PendingConversationPersist>>>,
    pub(crate) conversation_persist_notify: Arc<tokio::sync::Notify>,
    pub(crate) conversation_persist_started: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) conversation_persist_latest_seq: Arc<std::sync::atomic::AtomicU64>,
    pub(crate) cached_conversation_dirty_ids: Arc<Mutex<std::collections::HashSet<String>>>,
    pub(crate) cached_deleted_conversation_ids: Arc<Mutex<std::collections::HashSet<String>>>,
    pub(crate) app_data_persist_write_lock: Arc<Mutex<()>>,
    pub(crate) last_panic_snapshot: Arc<Mutex<Option<String>>>,
    pub(crate) inflight_chat_abort_handles: Arc<Mutex<std::collections::HashMap<String, AbortHandle>>>,
    pub(crate) inflight_tool_abort_handles: Arc<Mutex<std::collections::HashMap<String, AbortHandle>>>,
    pub(crate) inflight_completed_tool_history:
        Arc<Mutex<std::collections::HashMap<String, Vec<Value>>>>,
    pub(crate) terminal_session_roots: Arc<Mutex<std::collections::HashMap<String, String>>>,
    pub(crate) terminal_live_sessions: Arc<
        tokio::sync::Mutex<std::collections::HashMap<String, TerminalLiveShellSessionHandle>>,
    >,
    pub(crate) terminal_pending_approvals:
        Arc<Mutex<std::collections::HashMap<String, PendingTerminalApprovalRequest>>>,
    pub(crate) llm_round_logs: Arc<Mutex<RecentLlmRoundLogs>>,
    pub(crate) conversation_runtime_slots:
        Arc<Mutex<std::collections::HashMap<String, ConversationRuntimeSlot>>>,
    pub(crate) conversation_processing_claims: Arc<Mutex<std::collections::HashSet<String>>>,
    pub(crate) goal_continue_suppressed_conversation_ids:
        Arc<Mutex<std::collections::HashSet<String>>>,
    pub(crate) pending_chat_result_senders: Arc<
        Mutex<
            std::collections::HashMap<
                String,
                tokio::sync::oneshot::Sender<Result<SendChatResult, String>>,
            >,
        >,
    >,
    pub(crate) pending_chat_delta_channels:
        Arc<Mutex<std::collections::HashMap<String, DeltaChannel>>>,
    pub(crate) accepted_submit_trace_ids: Arc<Mutex<std::collections::VecDeque<String>>>,
    pub(crate) active_chat_view_bindings:
        Arc<Mutex<std::collections::HashMap<String, ActiveChatViewBinding>>>,
    pub(crate) conversation_list_activity_marks:
        Arc<Mutex<std::collections::HashMap<String, ConversationListActivityMark>>>,
    pub(crate) dequeue_lock: Arc<Mutex<()>>,
    pub(crate) task_scheduler_notify: Arc<tokio::sync::Notify>,
    pub(crate) delegate_runtime_threads:
        Arc<Mutex<std::collections::HashMap<String, DelegateRuntimeThread>>>,
    pub(crate) delegate_recent_threads:
        Arc<Mutex<std::collections::VecDeque<DelegateRuntimeThread>>>,
    pub(crate) provider_streaming_disabled_keys: Arc<Mutex<std::collections::HashMap<String, i64>>>,
    pub(crate) provider_system_message_user_fallback_keys:
        Arc<Mutex<std::collections::HashSet<String>>>,
    pub(crate) provider_request_gates:
        Arc<tokio::sync::Mutex<std::collections::HashMap<String, Arc<ProviderRequestGate>>>>,
    pub(crate) remote_im_contact_runtime_states:
        Arc<Mutex<std::collections::HashMap<String, RemoteImContactRuntimeState>>>,
    pub(crate) remote_im_reply_delegate_runtimes:
        Arc<Mutex<std::collections::HashMap<String, RemoteImReplyDelegateRuntime>>>,
    pub(crate) remote_im_reply_delegate_semaphore: Arc<tokio::sync::Semaphore>,
    pub(crate) remote_im_channel_state_write_locks:
        Arc<Mutex<std::collections::HashMap<String, Arc<Mutex<()>>>>>,
    pub(crate) hidden_skill_snapshot_cache: Arc<Mutex<String>>,
    pub(crate) preferred_release_source: Arc<Mutex<String>>,
    pub(crate) migration_preview_dirs: Arc<Mutex<std::collections::HashMap<String, String>>>,
    /// 当前活跃的委托线程 conversation_id 集合。
    /// 工具审批链路通过查表判断当前是否应跳过弹窗（有委托活跃 → 不弹窗，默认拒绝）。
    pub(crate) delegate_active_ids: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    /// 后端 setup 完成标记，前端在此标记为 true 之前不应发起数据加载。
    pub(crate) backend_ready: Arc<std::sync::atomic::AtomicBool>,
}

impl std::fmt::Debug for AppState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config_path", &self.config_path)
            .field("data_path", &self.data_path)
            .field("llm_workspace_path", &self.llm_workspace_path)
            .field("terminal_shell", &self.terminal_shell)
            .field("terminal_shell_candidates", &self.terminal_shell_candidates)
            .finish_non_exhaustive()
    }
}

pub(crate) fn current_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
}

pub(crate) fn portable_marker_path_from_exe_dir(exe_dir: &Path) -> PathBuf {
    exe_dir.join("PORTABLE")
}

pub(crate) fn portable_data_root_from_exe_dir(exe_dir: &Path) -> PathBuf {
    exe_dir.join("data")
}

pub(crate) fn detect_portable_runtime_root() -> Option<PathBuf> {
    let exe_dir = current_exe_dir()?;
    if portable_marker_path_from_exe_dir(&exe_dir).exists() {
        Some(portable_data_root_from_exe_dir(&exe_dir))
    } else {
        None
    }
}

pub(crate) fn resolve_standard_config_dirs() -> Result<(PathBuf, PathBuf), String> {
    let legacy_project_dirs = ProjectDirs::from("ai", "easycall", "easy-call-ai")
        .ok_or_else(|| "Failed to resolve legacy config directory".to_string())?;
    let next_project_dirs = ProjectDirs::from("ai", "easycall", "p-ai")
        .ok_or_else(|| "Failed to resolve new config directory".to_string())?;
    Ok((
        legacy_project_dirs.config_dir().to_path_buf(),
        next_project_dirs.config_dir().to_path_buf(),
    ))
}

pub(crate) fn resolve_standard_config_dir() -> Result<(PathBuf, PathBuf), String> {
    let (legacy_config_dir, next_config_dir) = resolve_standard_config_dirs()?;
    let legacy_exists = legacy_config_dir.exists();
    let next_exists = next_config_dir.exists();
    let config_dir = if next_exists {
        next_config_dir.clone()
    } else if legacy_exists {
        if let Some(parent) = next_config_dir.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "Create new config parent directory failed ({}): {err}",
                    parent.display()
                )
            })?;
        }
        fs::rename(&legacy_config_dir, &next_config_dir).map_err(|err| {
            format!(
                "Migrate legacy config directory failed ({} -> {}): {err}",
                legacy_config_dir.display(),
                next_config_dir.display()
            )
        })?;
        next_config_dir.clone()
    } else {
        fs::create_dir_all(&next_config_dir).map_err(|err| {
            format!(
                "Create new config directory failed ({}): {err}",
                next_config_dir.display()
            )
        })?;
        next_config_dir.clone()
    };
    fs::create_dir_all(&config_dir)
        .map_err(|err| format!("Create config directory failed: {err}"))?;
    Ok((config_dir, legacy_config_dir))
}

impl AppState {
    pub(crate) fn new() -> Result<Self, String> {
        let (config_dir, _legacy_config_dir, app_root, legacy_app_root) =
            if let Some(portable_root) = detect_portable_runtime_root() {
                let config_dir = portable_root.join("config");
                fs::create_dir_all(&config_dir).map_err(|err| {
                    format!(
                        "Create portable config directory failed ({}): {err}",
                        config_dir.display()
                    )
                })?;
                (
                    config_dir,
                    portable_root.join("legacy-unused"),
                    portable_root.clone(),
                    portable_root,
                )
            } else {
                let (config_dir, legacy_config_dir) = resolve_standard_config_dir()?;
                let app_root = config_dir.clone();
                let legacy_app_root = legacy_config_dir
                    .parent()
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| legacy_config_dir.clone());
                (config_dir, legacy_config_dir, app_root, legacy_app_root)
            };
        Self::init_from_dirs(config_dir, app_root, legacy_app_root)
    }

    /// Android / embedded entry point: construct AppState from a caller-supplied root directory,
    /// skipping portable detection and `ProjectDirs` resolution (which depends on `$HOME`/XDG,
    /// not reliably set on Android).
    pub(crate) fn new_with_root(app_root: PathBuf) -> Result<Self, String> {
        let config_dir = app_root.join("config");
        fs::create_dir_all(&config_dir).map_err(|err| {
            format!(
                "Create config directory from root failed ({}): {err}",
                config_dir.display()
            )
        })?;
        // No legacy migration needed on a fresh mobile data directory.
        let legacy_app_root = app_root.clone();
        Self::init_from_dirs(config_dir, app_root, legacy_app_root)
    }

    pub(crate) fn init_from_dirs(
        config_dir: PathBuf,
        app_root: PathBuf,
        legacy_app_root: PathBuf,
    ) -> Result<Self, String> {
        for dir_name in ["avatars", "media", "exports"] {
            let legacy = config_dir.join(dir_name);
            let target = app_root.join(dir_name);
            if legacy.exists() && !target.exists() {
                fs::rename(&legacy, &target).map_err(|err| {
                    format!(
                        "Migrate legacy {dir_name} dir failed ({} -> {}): {err}",
                        legacy.display(),
                        target.display()
                    )
                })?;
            }
        }
        let llm_workspace_path = app_root.join("llm-workspace");
        for legacy_llm_workspace_path in [
            legacy_app_root.join("llm-workspace"),
            config_dir.join("llm-workspace"),
        ] {
            if legacy_llm_workspace_path.exists() && !llm_workspace_path.exists() {
                fs::rename(&legacy_llm_workspace_path, &llm_workspace_path).map_err(|err| {
                    format!(
                        "Migrate llm workspace failed ({} -> {}): {err}",
                        legacy_llm_workspace_path.display(),
                        llm_workspace_path.display()
                    )
                })?;
                break;
            }
        }
        fs::create_dir_all(&llm_workspace_path)
            .map_err(|err| format!("Create llm workspace failed: {err}"))?;
        let terminal_shell_candidates = detect_terminal_shell_candidates();
        let terminal_shell = detect_default_terminal_shell();
        let mut shared_http_client_builder = reqwest::Client::builder()
            .user_agent(app_http_user_agent())
            .default_headers(app_identity_headers())
            .timeout(std::time::Duration::from_secs(12))
            .connect_timeout(std::time::Duration::from_secs(8))
            .pool_idle_timeout(std::time::Duration::from_secs(90))
            .redirect(reqwest::redirect::Policy::limited(10));
        #[cfg(target_os = "android")]
        {
            shared_http_client_builder = shared_http_client_builder.tls_certs_only(
                webpki_root_certs::TLS_SERVER_ROOT_CERTS
                    .iter()
                    .map(|der| reqwest::tls::Certificate::from_der(der.as_ref()).unwrap()),
            );
        }
        let shared_http_client = shared_http_client_builder
            .build()
            .map_err(|err| format!("Build shared HTTP client failed: {err}"))?;

        Ok(Self {
            app_handle: Arc::new(Mutex::new(None)),
            config_path: config_dir.join("app_config.toml"),
            data_path: config_dir.join("app_data.json"),
            llm_workspace_path,
            shared_http_client,
            terminal_shell,
            terminal_shell_candidates,
            conversation_lock: Arc::new(ConversationDomainLock::new()),
            memory_lock: Arc::new(Mutex::new(())),
            cached_config: Arc::new(Mutex::new(None)),
            cached_config_mtime: Arc::new(Mutex::new(None)),
            cached_agents: Arc::new(Mutex::new(None)),
            cached_agents_mtime: Arc::new(Mutex::new(None)),
            cached_runtime_state: Arc::new(Mutex::new(None)),
            cached_runtime_state_mtime: Arc::new(Mutex::new(None)),
            cached_chat_index: Arc::new(Mutex::new(None)),
            cached_conversation_metadata: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_conversation_field_metadata_ids: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            cached_conversation_mtimes: Arc::new(Mutex::new(std::collections::HashMap::new())),
            cached_app_data: Arc::new(Mutex::new(None)),
            cached_app_data_signature: Arc::new(Mutex::new(None)),
            cached_app_data_dirty: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_pending: Arc::new(Mutex::new(None)),
            app_data_persist_notify: Arc::new(tokio::sync::Notify::new()),
            app_data_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            app_data_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            conversation_persist_pending: Arc::new(Mutex::new(None)),
            conversation_persist_notify: Arc::new(tokio::sync::Notify::new()),
            conversation_persist_started: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            conversation_persist_latest_seq: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            cached_conversation_dirty_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            cached_deleted_conversation_ids: Arc::new(Mutex::new(std::collections::HashSet::new())),
            app_data_persist_write_lock: Arc::new(Mutex::new(())),
            last_panic_snapshot: Arc::new(Mutex::new(None)),
            inflight_chat_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_tool_abort_handles: Arc::new(Mutex::new(std::collections::HashMap::new())),
            inflight_completed_tool_history:
                Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_session_roots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            terminal_live_sessions: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            terminal_pending_approvals: Arc::new(Mutex::new(std::collections::HashMap::new())),
            llm_round_logs: Arc::new(Mutex::new(RecentLlmRoundLogs::default())),
            conversation_runtime_slots: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_processing_claims: Arc::new(Mutex::new(std::collections::HashSet::new())),
            goal_continue_suppressed_conversation_ids: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            pending_chat_result_senders: Arc::new(Mutex::new(std::collections::HashMap::new())),
            pending_chat_delta_channels: Arc::new(Mutex::new(std::collections::HashMap::new())),
            accepted_submit_trace_ids: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            active_chat_view_bindings: Arc::new(Mutex::new(std::collections::HashMap::new())),
            conversation_list_activity_marks: Arc::new(Mutex::new(std::collections::HashMap::new())),
            dequeue_lock: Arc::new(Mutex::new(())),
            task_scheduler_notify: Arc::new(tokio::sync::Notify::new()),
            delegate_runtime_threads: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_recent_threads: Arc::new(Mutex::new(std::collections::VecDeque::new())),
            provider_streaming_disabled_keys: Arc::new(Mutex::new(std::collections::HashMap::new())),
            provider_system_message_user_fallback_keys: Arc::new(Mutex::new(
                std::collections::HashSet::new(),
            )),
            provider_request_gates: Arc::new(tokio::sync::Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_contact_runtime_states: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_reply_delegate_runtimes: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            remote_im_reply_delegate_semaphore: Arc::new(tokio::sync::Semaphore::new(8)),
            remote_im_channel_state_write_locks: Arc::new(Mutex::new(
                std::collections::HashMap::new(),
            )),
            hidden_skill_snapshot_cache: Arc::new(Mutex::new(String::new())),
            preferred_release_source: Arc::new(Mutex::new("github".to_string())),
            migration_preview_dirs: Arc::new(Mutex::new(std::collections::HashMap::new())),
            delegate_active_ids: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
            backend_ready: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }
}

pub(crate) fn app_root_from_data_path(data_path: &PathBuf) -> PathBuf {
    let parent = data_path
        .parent()
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| PathBuf::from("."));
    let is_config_dir = parent
        .file_name()
        .and_then(|v| v.to_str())
        .map(|v| v.eq_ignore_ascii_case("config"))
        .unwrap_or(false);
    if is_config_dir {
        if let Some(root) = parent.parent() {
            return root.to_path_buf();
        }
    }
    parent
}

pub(crate) fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

pub(crate) fn now_iso() -> String {
    now_utc_rfc3339()
}

pub(crate) fn parse_iso(value: &str) -> Option<OffsetDateTime> {
    parse_rfc3339_time(value)
}
