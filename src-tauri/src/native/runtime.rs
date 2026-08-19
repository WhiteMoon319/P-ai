/// 全局原生运行时：Tokio runtime + 业务状态 + IDE 上下文运行时。
struct NativeRuntime {
    runtime: tokio::runtime::Runtime,
    state: AppState,
    ide_context_runtime: IdeContextRuntime,
}

static NATIVE_RUNTIME: OnceLock<Result<Arc<NativeRuntime>, String>> = OnceLock::new();

/// 原生流式事件队列：Kotlin 通过 pollEvents 轮询弹出。
/// dispatch_assistant_delta_to_active_view 在 Android 分支把所有 delta 事件 push 进来，
fn native_runtime() -> Result<&'static Arc<NativeRuntime>, String> {
    NATIVE_RUNTIME
        .get()
        .ok_or_else(|| "原生运行时尚未初始化（未调用 nativeInit）".to_string())
        .and_then(|entry| match entry {
            Ok(runtime) => Ok(runtime),
            Err(err) => Err(err.clone()),
        })
}

/// 初始化原生后端：自建 Tokio runtime + AppState + IdeContextRuntime。
fn init_native_runtime(app_root: std::path::PathBuf) -> Result<(), String> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .thread_stack_size(8 * 1024 * 1024)
        .build()
        .map_err(|err| format!("创建原生 Tokio 运行时失败: {err}"))?;
    // 自建 8MB 栈 Tokio runtime。chat.send 深层链（模型请求/工具/委托）内部使用
    // tokio::spawn 提交任务，这些调用点都在本 runtime 的 worker 上下文中执行，
    // 天然使用本 runtime 的 handle；不再需要 tauri::async_runtime::set 安装全局 handle。
    let state = AppState::new_with_root(app_root)?;
    let ide_context_runtime = IdeContextRuntime::new();
    // Android 原生模式：启动数据持久化 worker（配置/会话/记忆落盘）。
    // worker 内部用 tokio::spawn，需在 runtime context 内启动，故用 handle.spawn 包裹。
    let worker_state = state.clone();
    let worker_runtime = runtime.handle().clone();
    worker_runtime.spawn(async move {
        if let Err(err) = start_app_data_persist_worker(&worker_state) {
            runtime_log_error(format!("[启动] 应用数据持久化 worker 启动失败: {err}"));
        }
        if let Err(err) = start_conversation_persist_worker(&worker_state) {
            runtime_log_error(format!("[启动] 会话持久化 worker 启动失败: {err}"));
        }
    });
    // Android 原生模式：启动时自动拉起配置中 enabled 的远程 IM 渠道。
    // 等价桌面 Vue afterSafetyGateReady 调用的 remoteIm.services.start；
    // 幂等（reconcile 先停后按 enabled 启动），失败记录日志不阻塞初始化。
    let remote_im_state = state.clone();
    let remote_im_runtime = runtime.handle().clone();
    remote_im_runtime.spawn(async move {
        match start_remote_im_services_inner(&remote_im_state).await {
            Ok(value) => {
                let started = value.get("started").and_then(serde_json::Value::as_u64).unwrap_or(0);
                let failed = value.get("failed").and_then(serde_json::Value::as_u64).unwrap_or(0);
                runtime_log_info(format!(
                    "[远程IM] 启动完成: started={}, failed={}",
                    started, failed
                ));
            }
            Err(err) => {
                runtime_log_error(format!("[远程IM] 启动全部渠道失败: {err}"));
            }
        }
    });
    // Android 原生模式：按配置拉起 Web 访问服务（远程连接）。
    // 等价桌面 run_deferred_setup 的 start_web_access_server；配置关闭时服务自动跳过。
    let native_app = NativeAppHandle::noop();
    let start_state = state.clone();
    let start_ide_context_runtime = ide_context_runtime.clone();
    let handle = runtime.handle().clone();
    handle.spawn(async move {
        start_web_access_server(native_app, start_state, start_ide_context_runtime).await;
    });
    NATIVE_RUNTIME
        .set(Ok(Arc::new(NativeRuntime {
            runtime,
            state,
            ide_context_runtime,
        })))
        .map_err(|_| "原生运行时重复初始化".to_string())
}