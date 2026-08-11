// ========== Android 原生桥（JNI） ==========
// 彻底拔掉 Tauri 运行时后，Kotlin 通过 System.loadLibrary 加载本 .so，
// 直接调用 Java_com_whitemoon319_pai_* 方法完成初始化与 JSON-RPC 调用。
//
// 设计：
// - 全局 NativeRuntime 单例：自建 Tokio runtime + AppState + IdeContextRuntime
// - nativeInit(appRoot)：用应用数据目录初始化后端（等价原 tauri setup 的 AppState::new_with_root）
// - nativeCall(requestJson)：同步执行 JSON-RPC（block_on dispatch），返回响应 JSON
// - 事件下发（流式 token/通知）暂走事件队列，后续轮次补齐

use jni::objects::{JClass, JString};
use jni::sys::{jstring};
use jni::JNIEnv;

/// 全局原生运行时：Tokio runtime + 业务状态 + IDE 上下文运行时。
struct NativeRuntime {
    runtime: tokio::runtime::Runtime,
    state: AppState,
    ide_context_runtime: IdeContextRuntime,
}

static NATIVE_RUNTIME: OnceLock<Result<Arc<NativeRuntime>, String>> = OnceLock::new();

/// 原生流式事件队列：Kotlin 通过 pollEvents 轮询弹出。
/// dispatch_assistant_delta_to_active_view 在 Android 分支把所有 delta 事件 push 进来，
/// AppViewModel/前端轮询 Java_com_whitemoon319_pai_native_PaiNative_pollEvents 取出。
static NATIVE_DELTA_QUEUE: OnceLock<std::sync::Mutex<Vec<serde_json::Value>>> = OnceLock::new();

pub(crate) fn native_delta_queue() -> &'static std::sync::Mutex<Vec<serde_json::Value>> {
    NATIVE_DELTA_QUEUE.get_or_init(|| std::sync::Mutex::new(Vec::new()))
}

/// 把一条流式事件追加进原生事件队列（Android 分支专用）。
pub(crate) fn push_native_delta_event(event: serde_json::Value) {
    if let Ok(mut guard) = native_delta_queue().lock() {
        guard.push(event);
        // 队列只作短暂缓冲，Kotlin 高频轮询清空，不会无限增长。
        if guard.len() > 4096 {
            let len = guard.len();
            let overflow = guard.split_off(len - 2048);
            *guard = overflow;
        }
    }
}

/// 弹出并清空当前事件队列，返回 JSON 数组字符串。
fn drain_native_delta_events() -> String {
    let mut events = Vec::new();
    if let Ok(mut guard) = native_delta_queue().lock() {
        std::mem::swap(&mut events, &mut guard);
    }
    match serde_json::to_string(&events) {
        Ok(json) => json,
        Err(_) => "[]".to_string(),
    }
}

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

/// 提取 Android 应用数据目录（Kotlin 传入的 filesDir 等）。
unsafe fn app_root_from_env(env: &mut JNIEnv, input: JString) -> Result<std::path::PathBuf, String> {
    let raw: String = env
        .get_string(&input)
        .map_err(|err| format!("读取 appRoot 失败: {err}"))?
        .into();
    let root = std::path::PathBuf::from(raw);
    if root.as_os_str().is_empty() {
        return Err("appRoot 为空".to_string());
    }
    Ok(root)
}

// ========== JNI 导出 ==========

/// Java_com_whitemoon319_pai_native_PaiNative_init(String appRoot) -> String
/// 返回 "ok" 或错误信息（便于 Kotlin 侧直接弹错）。
#[no_mangle]
pub extern "system" fn Java_com_whitemoon319_pai_native_PaiNative_init(
    mut env: JNIEnv,
    _class: JClass,
    app_root: JString,
) -> jstring {
    let result = unsafe { app_root_from_env(&mut env, app_root) }
        .and_then(init_native_runtime)
        .map(|()| "ok".to_string())
        .unwrap_or_else(|err| format!("nativeInit 失败: {err}"));
    match env.new_string(result) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Java_com_whitemoon319_pai_native_PaiNative_call(String requestJson) -> String
/// 同步执行 JSON-RPC 并返回响应 JSON；失败时返回 JSON-RPC error 结构。
#[no_mangle]
pub extern "system" fn Java_com_whitemoon319_pai_native_PaiNative_call(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    let response = native_call_inner(&mut env, &request_json);
    match env.new_string(response) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Java_com_whitemoon319_pai_native_PaiNative_pollEvents() -> String
/// 拉取并清空待下发事件（流式 delta / 工具事件等），返回 JSON 数组。
#[no_mangle]
pub extern "system" fn Java_com_whitemoon319_pai_native_PaiNative_pollEvents(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let events = drain_native_delta_events();
    match env.new_string(events) {
        Ok(js) => js.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

fn native_call_inner(env: &mut JNIEnv, request_json: &JString) -> String {
    let request_text: String = match env.get_string(request_json) {
        Ok(s) => s.into(),
        Err(err) => {
            return ide_chat_jsonrpc_error(
                None,
                -32600,
                format!("读取请求失败: {err}"),
            )
            .to_string();
        }
    };

    let request: Value = match serde_json::from_str(&request_text) {
        Ok(value) => value,
        Err(err) => {
            return ide_chat_jsonrpc_error(None, -32700, format!("invalid json: {err}")).to_string();
        }
    };
    let id = request.get("id").cloned();
    let method = request
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let params = request.get("params").cloned().unwrap_or(Value::Null);

    if method.trim().is_empty() {
        return ide_chat_jsonrpc_error(id, -32600, "missing method").to_string();
    }

    let runtime = match native_runtime() {
        Ok(runtime) => runtime.clone(),
        Err(err) => {
            return ide_chat_jsonrpc_error(id, -32000, err).to_string();
        }
    };

    // 同步边界：把 dispatch 提交到原生 Tokio worker（8MB 栈）执行，JNI 线程只等待结果。
    // 直接 block_on 会在 Kotlin IO 线程（栈 ~1MB）驱动 chat.send 等深递归 future 导致栈溢出。
    let join = runtime
        .runtime
        .spawn(native_dispatch(runtime.clone(), method, params, id.clone()));
    let result = runtime.runtime.block_on(join).unwrap_or_else(|err| {
        Err(format!("原生 dispatch 任务异常: {err}"))
    });
    match result {
        Ok(value) => ide_chat_jsonrpc_success(id, value).to_string(),
        Err(err) => ide_chat_jsonrpc_error(id, -32000, err).to_string(),
    }
}

/// 原生精简 dispatch：复用现有 `*_ws_inner` / `*_for_web_settings` 等只依赖 `&AppState` 的实现。
async fn native_dispatch(
    runtime: Arc<NativeRuntime>,
    method: String,
    params: Value,
    _id: Option<Value>,
) -> Result<Value, String> {
    let state = &runtime.state;
    // Android 原生模式：无真实 AppHandle（emit/path 走 pollEvents 旁路），用 noop 占位；
    // 写配置类方法（save_config/save_agents/api_config 等）通过 web_settings 包装调用共享 inner。
    let native_app = NativeAppHandle::noop();
    let ide_context_runtime = &runtime.ide_context_runtime;
    // sidebar 语义在原生端固定一个 viewer id，避免 ws 侧 client_id 概念。
    let viewer_id = "android-native";
    // 原生桥单会话模式：resumeSubscription 登记到固定 client_id，流式事件后续走事件队列。
    let mut opened_conversation_id: Option<String> = None;

    // 需要 NativeAppHandle 的方法（写配置/事件推送等）本轮返回暂不支持，后续轮次迁移。
        // 尚未接入原生通道的方法（工作区初始化/迁移等，后续轮次迁移）。
    let app_dependent = [
        "frontend_ready_start_remote_im_services",
        "run_message_store_migration",
        "check_message_store_migration",
    ];
    if app_dependent.contains(&method.as_str()) {
        return Err(format!(
            "原生桥暂不支持需要原生事件通道的方法: {method}（后续轮次迁移）"
        ));
    }

    match method.as_str() {
        "bridge.ping" => Ok(serde_json::json!({
            "ok": true,
            "ts": chrono::Utc::now().to_rfc3339(),
        })),
        "webview.ping" => Ok(serde_json::json!(true)),
        "webview_pong" => Ok(serde_json::json!(true)),
        "conversation.list" => ide_chat_conversation_list(state, viewer_id),
        "conversation.setActive" => ide_chat_set_active_conversation_command(state, params),
        "conversation.resumeSubscription" => ide_chat_resume_sidebar_subscription(
            state,
            params,
            viewer_id,
            &mut opened_conversation_id,
        ),
        "conversation.create" => ide_chat_create_conversation(state, params)
            .await
            .and_then(|result| Ok(result)),
        "conversation.createOptions" => ide_chat_create_conversation_options(state),
        "conversation.blockPage" => ide_chat_conversation_block_page(state, params),
        "conversation.messageById" => ide_chat_conversation_message_by_id_command(state, params),
        "conversation.messagesBefore" => ide_chat_conversation_messages_before_command(state, params),
        "conversation.markRead" => ide_chat_mark_conversation_read(state, params),
        "conversation.setPreferredModel" => ide_chat_set_preferred_model_command(state, params),
        "model.list" => ide_chat_model_list(state, params),
        "get_prompt_preview" => ide_chat_get_prompt_preview_for_web_settings(state, params).await,
        "prompt.systemPreview" => ide_chat_get_system_prompt_preview_for_web_settings(state, params).await,
        "conversation.rewind" => ide_chat_rewind_conversation_command(state, params).await,
        "conversation.rewindPreview" => ide_chat_preview_rewind_conversation(state, params).await,
        "conversation.compact" => ide_chat_compact_conversation(state, params).await,
        "conversation.compactPreview" => ide_chat_compact_preview(state, params),
        "conversation.autoPush" => ide_chat_set_auto_push_command(state, params),
        "conversation.rename" => ide_chat_rename_conversation_command(state, params),
        "conversation.pin" => ide_chat_toggle_pin_command(state, params),
        "conversation.delete" => ide_chat_delete_conversation(state, params).await,
        "conversation.batchArchive" => ide_chat_batch_archive_conversations(state, params).await,
        "conversation.exportShare" => ide_chat_export_conversation_share_command(state, params),
        // ---- 归档会话管理（对齐 Vue ArchivesWindow）----
        "archives.list" => ide_chat_list_archives_command(state),
        "archives.blockPage" => ide_chat_archive_block_page_command(state, params),
        "archives.summary" => ide_chat_archive_summary_command(state, params),
        "archives.delete" => ide_chat_delete_archive_command(state, params),
        "archives.unarchive" => ide_chat_unarchive_command(state, params),
        "conversation.runtimeSnapshot" => ide_chat_conversation_runtime_snapshot(state, params),
        "conversation.fastRequestTurns" => ide_chat_conversation_fast_request_turns(state, params),
        "conversation.freshnessSnapshot" => ide_chat_conversation_freshness_snapshot(state, params).await,
        "chat.send" => ide_chat_send_message(state, params).await,
        "chat.stop" => ide_chat_stop_conversation(state, params),
        "load_config" => ide_chat_load_config_for_web_settings(state),
        "load_chat_settings" => ide_chat_load_chat_settings_for_web_settings(state),
        "save_config" => ide_chat_save_config_for_web_settings(state, &native_app, ide_context_runtime, params),
        "load_agents" => ide_chat_load_agents_for_web_settings(state),
        "save_agents" => ide_chat_save_agents_for_web_settings(state, &native_app, params),
        "save_chat_settings" => ide_chat_save_chat_settings_for_web_settings(state, &native_app, params),
        "patch_chat_settings" => ide_chat_patch_chat_settings_for_web_settings(state, &native_app, params),
        "save_conversation_api_settings" => ide_chat_save_conversation_api_settings_for_web_settings(state, &native_app, params),
        "patch_conversation_api_settings" => ide_chat_patch_conversation_api_settings_for_web_settings(state, &native_app, params),
        "set_ui_language" => ide_chat_set_ui_language_command(state, &native_app, params),
        "app.language.set" => ide_chat_set_ui_language_command(state, &native_app, params),
        "set_department_primary_api_config" => ide_chat_set_department_primary_api_command(state, &native_app, params),
        "department.primaryApi.set" => ide_chat_set_department_primary_api_command(state, &native_app, params),
        "set_github_update_method" => ide_chat_set_github_update_method_for_web_settings(state, &native_app, params),
        "set_skipped_github_update_version" => ide_chat_set_skipped_github_update_version_for_web_settings(state, &native_app, params),
        "convert_private_agent_to_main" => ide_chat_convert_private_agent_to_main_for_web_settings(state, &native_app, params),
        "set_agent_private_memory_enabled" => ide_chat_set_agent_private_memory_enabled_for_web_settings(state, params),
        "set_agent_memory_recall_mode" => ide_chat_set_agent_memory_recall_mode_for_web_settings(state, params),
        "check_github_update" => Ok(serde_json::json!({
            "currentVersion": env!("CARGO_PKG_VERSION"),
            "latestVersion": env!("CARGO_PKG_VERSION"),
            "hasUpdate": false,
            "releaseUrl": "",
            "updateSource": "apk",
            "accessMode": "none",
            "releaseNotes": "",
            "publishedAt": null,
            "runtimeKind": "android",
            "canForceUpdate": false,
        })),
        "test_text_connection" => ide_chat_serialize(test_text_connection_inner(
            ide_chat_parse_param_field::<ApiConfig>(params, "input")?, state).await?),
        "api_config.create" => ide_chat_serialize(api_config_create_inner(
            ide_chat_parse_param_field::<ApiConfig>(params, "input")?, &native_app, state, ide_context_runtime)?),
        "api_config.update" => ide_chat_serialize(api_config_update_inner(
            ide_chat_parse_param_field::<ApiConfig>(params, "input")?, &native_app, state, ide_context_runtime)?),
        "api_config.delete" => ide_chat_serialize(api_config_delete_inner(
            ide_chat_parse_param_field::<ApiConfigDeleteInput>(params, "input")?, &native_app, state, ide_context_runtime)?),
        "check_tools_status" => ide_chat_check_tools_status_for_web_settings(state, params),
        "stt_transcribe" => ide_chat_stt_transcribe_for_web_settings(state, params).await,
        "attachment.ingestLocalPath" => ide_chat_serialize(attachment_ingest_local_path_inner(
            ide_chat_parse_param_field::<AttachmentIngestLocalPathInput>(params, "input")?, state).await?),
        "get_web_access_info" => ide_chat_web_access_info_for_web_settings(&native_app, state, ide_context_runtime).await,
        "transport.accessInfo" => ide_chat_web_access_info_for_web_settings(&native_app, state, ide_context_runtime).await,
        "app.bootstrapSnapshot" => ide_chat_load_app_bootstrap_snapshot_for_web_settings(state),
        // ---- Vue 设置页对齐：存储/用量/日志 ----
        "get_storage_usage_overview" => ide_chat_get_storage_usage_overview_for_web_settings(state).await,
        "refresh_storage_usage_overview" => ide_chat_refresh_storage_usage_overview_for_web_settings(state).await,
        "cleanup_storage_legacy_items" => ide_chat_cleanup_storage_legacy_items_for_web_settings(state, params),
        "get_usage_overview" => ide_chat_get_usage_overview_for_web_settings(state).await,
        "list_recent_runtime_logs" => list_recent_runtime_logs().and_then(ide_chat_serialize),
        "list_runtime_logs_since" => {
            let since = params.get("sinceCreatedAt").and_then(Value::as_str).map(str::to_string);
            list_runtime_logs_since(since).and_then(ide_chat_serialize)
        }
        "clear_recent_runtime_logs" => clear_recent_runtime_logs().and_then(ide_chat_serialize),
        "append_runtime_log_probe" => append_runtime_log_probe(
            params.get("message").and_then(Value::as_str).map(str::to_string),
        ).and_then(ide_chat_serialize),
        // ---- LLM 轮次日志（诊断）----
        "list_recent_llm_round_logs" => ide_chat_list_recent_llm_round_logs_for_web_settings(state),
        "get_recent_llm_round_log_section" => ide_chat_get_recent_llm_round_log_section_for_web_settings(state, params),
        "clear_recent_llm_round_logs" => ide_chat_clear_recent_llm_round_logs_for_web_settings(state),
        // ---- Vue 设置页对齐：记忆 ----
        "list_memories" => ide_chat_list_memories_for_web_settings(state),
        "delete_memory" => ide_chat_delete_memory_for_web_settings(state, params),
        "search_memories_mixed" => ide_chat_search_memories_mixed_for_web_settings(state, params),
        "search_memories_recall" => ide_chat_search_memories_recall_command(state, params),
        "get_memory_provider_bindings" => ide_chat_get_memory_provider_bindings_for_web_settings(state),
        "get_memory_embedding_sync_progress" => ide_chat_get_memory_embedding_sync_progress_for_web_settings(state),
        "save_memory_embedding_binding" => ide_chat_save_memory_embedding_binding_for_web_settings(state, params),
        "save_memory_rerank_binding" => ide_chat_save_memory_rerank_binding_for_web_settings(state, params),
        "get_agent_private_memory_count" => ide_chat_get_agent_private_memory_count_for_web_settings(state, params),
        // ---- Vue 设置页对齐：MCP / 技能 ----
        "mcp_list_servers" => ide_chat_mcp_list_servers_for_web_settings(state),
        "mcp_validate_definition" => ide_chat_mcp_validate_definition_for_web_settings(params),
        "mcp_fix_definition" => ide_chat_mcp_fix_definition_for_web_settings(state, params).await,
        "mcp_save_server" => ide_chat_mcp_save_server_for_web_settings(state, params),
        "mcp_remove_server" => ide_chat_mcp_remove_server_for_web_settings(state, params).await,
        "mcp_list_server_tools" => ide_chat_mcp_list_server_tools_for_web_settings(state, params).await,
        "mcp_list_server_tools_cached" => ide_chat_mcp_list_server_tools_cached_for_web_settings(state, params),
        "mcp_deploy_server" => ide_chat_mcp_deploy_server_for_web_settings(state, params).await,
        "mcp_undeploy_server" => ide_chat_mcp_undeploy_server_for_web_settings(state, params).await,
        "mcp_list_skills" => ide_chat_mcp_list_skills_for_web_settings(state),
        // ---- Vue 设置页对齐：任务 ----
        "task_list_tasks" => ide_chat_task_list_tasks_for_web_settings(state),
        "task_get_task" => ide_chat_task_get_task_for_web_settings(state, params),
        "task_create_task" => ide_chat_task_create_task_for_web_settings(state, params),
        "task_update_task" => ide_chat_task_update_task_for_web_settings(state, params),
        "task_complete_task" => ide_chat_task_complete_task_for_web_settings(state, params),
        "task_delete_task" => ide_chat_task_delete_task_for_web_settings(state, params),
        "task_list_run_logs" => ide_chat_task_list_run_logs_for_web_settings(state, params),
        "task_optimize_draft" => ide_chat_task_optimize_draft_for_web_settings(state, params).await,
        // ---- Vue 设置页对齐：远程 IM ----
        "remote_im_list_channels" => ide_chat_remote_im_list_channels_for_web_settings(state),
        "remote_im_list_contacts" => ide_chat_remote_im_list_contacts_for_web_settings(state),
        "remote_im_get_channel_status" => ide_chat_remote_im_get_channel_status_for_web_settings(state, params).await,
        "remote_im_restart_channel" => ide_chat_remote_im_restart_channel_for_web_settings(state, params).await,
        "remote_im_get_channel_logs" => ide_chat_remote_im_get_channel_logs_for_web_settings(state, params).await,
        "remote_im_get_contact_logs" => ide_chat_remote_im_get_contact_logs_for_web_settings(state, params).await,
        "remote_im_update_contact_allow_send" => ide_chat_remote_im_update_contact_allow_send_for_web_settings(state, params),
        "remote_im_update_contact_allow_send_files" => ide_chat_remote_im_update_contact_allow_send_files_for_web_settings(state, params),
        "remote_im_update_contact_blocked_message_prefixes" => ide_chat_remote_im_update_contact_blocked_message_prefixes_for_web_settings(state, params),
        "remote_im_update_contact_activation" => ide_chat_remote_im_update_contact_activation_for_web_settings(state, params),
        "remote_im_update_contact_department_binding" => ide_chat_remote_im_update_contact_department_binding_for_web_settings(state, params),
        "remote_im_update_contact_processing_mode" => ide_chat_remote_im_update_contact_processing_mode_for_web_settings(state, params),
        "remote_im_update_contact_workspace" => ide_chat_remote_im_update_contact_workspace_for_web_settings(state, params),
        "remote_im_delete_contact" => ide_chat_remote_im_delete_contact_for_web_settings(state, params),
        "remote_im_weixin_oc_start_login" => ide_chat_remote_im_weixin_oc_start_login_for_web_settings(state, params).await,
        "remote_im_weixin_oc_get_login_status" => ide_chat_remote_im_weixin_oc_get_login_status_for_web_settings(state, params).await,
        "remote_im_weixin_oc_logout" => ide_chat_remote_im_weixin_oc_logout_for_web_settings(state, params).await,
        "remote_im_weixin_oc_sync_contacts" => ide_chat_remote_im_weixin_oc_sync_contacts_for_web_settings(state, params).await,
        "get_android_workspace_status" => ide_chat_serialize(get_android_workspace_status_ws_inner(state)?),
        "init_android_workspace" => ide_chat_serialize(init_android_workspace_ws_inner(state, Some(&native_app)).await?),
        "repair_android_workspace_runtime" => ide_chat_serialize(repair_android_workspace_runtime_ws_inner(state, Some(&native_app))?),
        "reset_android_workspace_runtime" => ide_chat_serialize(reset_android_workspace_runtime_ws_inner(
            state, Some(&native_app), &android_workspace_root(state))?),
        "reset_android_workspace_state" => ide_chat_serialize(reset_android_workspace_state_ws_inner(state, Some(&native_app))?),
        "import_android_workspace_rootfs_archive" => ide_chat_serialize(import_android_workspace_rootfs_archive_ws_inner(
            state,
            Some(&native_app),
            params.get("fileName").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("dataBase64").and_then(Value::as_str).unwrap_or_default().to_string(),
        ).await?),
        "android_workspace.list" => ide_chat_serialize(list_android_workspace_files_ws_inner(
            state,
            params.get("path").and_then(Value::as_str).map(str::to_string),
        )?),
        "android_workspace.readText" => ide_chat_serialize(read_android_workspace_text_ws_inner(
            state,
            params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
        )?),
        "android_workspace.writeText" => ide_chat_serialize(write_android_workspace_text_ws_inner(
            state,
            params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("text").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("overwrite").and_then(Value::as_bool),
        )?),
        "android_workspace.move" => ide_chat_serialize(move_android_workspace_file_ws_inner(
            state,
            params.get("source").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("target").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("overwrite").and_then(Value::as_bool),
        )?),
        "android_workspace.glob" => ide_chat_serialize(glob_android_workspace_files_ws_inner(
            state,
            params.get("pattern").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("path").and_then(Value::as_str).map(str::to_string),
        )?),
        "android_workspace.grep" => ide_chat_serialize(grep_android_workspace_files_ws_inner(
            state,
            params.get("query").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("path").and_then(Value::as_str).map(str::to_string),
            params.get("regex").and_then(Value::as_bool),
            params.get("ignoreCase").and_then(Value::as_bool),
            params.get("includeGlob").and_then(Value::as_str).map(str::to_string),
        )?),
        "android_workspace.delete" => ide_chat_serialize(delete_file_from_android_workspace_ws_inner(
            state,
            params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
        )?),
        "android_workspace.import" => ide_chat_serialize(import_file_to_android_workspace_ws_inner(
            state,
            params.get("fileName").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("mime").and_then(Value::as_str).map(str::to_string),
            params.get("dataBase64").and_then(Value::as_str).unwrap_or_default().to_string(),
            params.get("targetPath").and_then(Value::as_str).map(str::to_string),
        )?),
        "android_workspace.export" => ide_chat_serialize(export_file_from_android_workspace_ws_inner(
            state,
            params.get("path").and_then(Value::as_str).unwrap_or_default().to_string(),
        )?),
        "get_app_version" => Ok(serde_json::json!(env!("CARGO_PKG_VERSION").to_string())),
        "get_project_repository_url" => Ok(serde_json::json!(GITHUB_REPO_PAGE.to_string())),
        "list_terminal_shell_candidates" => ide_chat_list_terminal_shell_candidates_for_web_settings(state),
        "list_tool_catalog" => ide_chat_list_tool_catalog_for_web_settings(state).await,
        "list_department_permission_catalog" => ide_chat_list_department_permission_catalog_for_web_settings(state).await,
        _ => Err(format!("原生桥 method not found: {method}")),
    }
}
