// ==================== Android Live Update 通知 ====================
// 参考 MAA-Meow TaskExecutionService 的进行中任务通知：任务运行时用一条
// ongoing 通知常驻展示，结束后移除或转为普通通知。
// P-AI 在 Android 上为"消息输出中"和"目标进行中"各维护一条 live 通知：
//  - 轮次开始 → ongoing 通知（常驻，提示正在回复）
//  - 轮次完成/失败 → 同 id 更新为终态（非 ongoing，可手动划掉），
//    完成后不再打扰，避免与既有完成/失败通知重复弹窗
//  - 目标创建/更新 → ongoing 通知展示目标摘要；目标结束 → 更新为终态
// 桌面端无此语义，函数为空实现。

#[cfg(target_os = "android")]
const CHAT_LIVE_UPDATE_NOTIFICATION_ID: i32 = 0x50414901;
#[cfg(target_os = "android")]
const GOAL_LIVE_UPDATE_NOTIFICATION_ID: i32 = 0x50414902;
#[cfg(target_os = "android")]
const LIVE_UPDATE_BODY_MAX_CHARS: usize = 180;

// 固定通知 id 只有一条，多会话并发输出时后到的会话会覆盖前一条。
// 记录当前 live 通知归属的会话：只有归属会话结束才更新终态，避免误改
// 仍在进行中的其他会话通知。
#[cfg(target_os = "android")]
static CHAT_LIVE_UPDATE_OWNER: std::sync::OnceLock<
    std::sync::Mutex<Option<String>>,
> = std::sync::OnceLock::new();
#[cfg(target_os = "android")]
static GOAL_LIVE_UPDATE_OWNER: std::sync::OnceLock<
    std::sync::Mutex<Option<String>>,
> = std::sync::OnceLock::new();

// 保活：后台任务（回复轮次 / 目标）活跃会话集合。任务启动时加入、结束时移除，
// 集合非空则启动前台服务提升进程优先级，空则停止。保活独立于通知权限，
// 通知权限被拒时任务仍在后台运行，进程仍需保活。
#[cfg(target_os = "android")]
static CHAT_KEEP_ALIVE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::OnceLock::new();
#[cfg(target_os = "android")]
static GOAL_KEEP_ALIVE: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashSet<String>>,
> = std::sync::OnceLock::new();
#[cfg(target_os = "android")]
static KEEP_ALIVE_ACTIVE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[cfg(target_os = "android")]
fn live_update_owner_take(owner: &'static std::sync::OnceLock<std::sync::Mutex<Option<String>>>) {
    if let Ok(mut guard) = owner.get_or_init(|| std::sync::Mutex::new(None)).lock() {
        *guard = None;
    }
}

#[cfg(target_os = "android")]
fn live_update_owner_matches(
    owner: &'static std::sync::OnceLock<std::sync::Mutex<Option<String>>>,
    conversation_id: &str,
) -> bool {
    match owner.get_or_init(|| std::sync::Mutex::new(None)).lock() {
        Ok(guard) => guard
            .as_deref()
            .map(|current| current.trim() == conversation_id.trim())
            .unwrap_or(false),
        Err(_) => false,
    }
}

#[cfg(target_os = "android")]
fn live_update_owner_set(
    owner: &'static std::sync::OnceLock<std::sync::Mutex<Option<String>>>,
    conversation_id: &str,
) {
    if let Ok(mut guard) = owner.get_or_init(|| std::sync::Mutex::new(None)).lock() {
        *guard = Some(conversation_id.trim().to_string());
    }
}

#[cfg(target_os = "android")]
fn live_update_app_handle(state: &AppState) -> Option<tauri::AppHandle> {
    match state.app_handle.lock() {
        Ok(guard) => guard.as_ref().cloned(),
        Err(_) => None,
    }
}

#[cfg(target_os = "android")]
fn live_update_keep_alive_changed(app: &tauri::AppHandle) {
    use tauri_plugin_notification::NotificationExt;

    let chat_active = match CHAT_KEEP_ALIVE
        .get_or_init(|| std::sync::Mutex::new(Default::default()))
        .lock()
    {
        Ok(guard) => !guard.is_empty(),
        Err(_) => false,
    };
    let goal_active = match GOAL_KEEP_ALIVE
        .get_or_init(|| std::sync::Mutex::new(Default::default()))
        .lock()
    {
        Ok(guard) => !guard.is_empty(),
        Err(_) => false,
    };
    let active = chat_active || goal_active;
    let previous = KEEP_ALIVE_ACTIVE.swap(active, std::sync::atomic::Ordering::SeqCst);
    if previous == active {
        return;
    }
    let notifications = app.notification();
    let result = if active {
        notifications.keep_alive_start()
    } else {
        notifications.keep_alive_stop()
    };
    if let Err(err) = result {
        runtime_log_warn(format!(
            "[Live更新] 保活命令失败，active={}，error={}",
            active, err
        ));
    }
}

#[cfg(target_os = "android")]
fn live_update_keep_alive_chat(state: &AppState, conversation_id: &str, active: bool) {
    let Some(app) = live_update_app_handle(state) else {
        return;
    };
    let mut guard = match CHAT_KEEP_ALIVE
        .get_or_init(|| std::sync::Mutex::new(Default::default()))
        .lock()
    {
        Ok(guard) => guard,
        Err(_) => return,
    };
    if active {
        guard.insert(conversation_id.trim().to_string());
    } else {
        guard.remove(conversation_id.trim());
    }
    drop(guard);
    live_update_keep_alive_changed(&app);
}

#[cfg(target_os = "android")]
fn live_update_keep_alive_goal(state: &AppState, conversation_id: &str, active: bool) {
    let Some(app) = live_update_app_handle(state) else {
        return;
    };
    let mut guard = match GOAL_KEEP_ALIVE
        .get_or_init(|| std::sync::Mutex::new(Default::default()))
        .lock()
    {
        Ok(guard) => guard,
        Err(_) => return,
    };
    if active {
        guard.insert(conversation_id.trim().to_string());
    } else {
        guard.remove(conversation_id.trim());
    }
    drop(guard);
    live_update_keep_alive_changed(&app);
}

#[cfg(target_os = "android")]
fn live_update_send(
    app: &tauri::AppHandle,
    id: i32,
    title: &str,
    body: &str,
    ongoing: bool,
    promoted: bool,
) {
    use tauri_plugin_notification::{NotificationExt, PermissionState};

    let normalized_title = title.trim();
    let normalized_body = body.trim();
    if normalized_title.is_empty() || normalized_body.is_empty() {
        return;
    }
    let notifications = app.notification();
    let permission = match notifications.permission_state() {
        Ok(permission) => permission,
        Err(err) => {
            runtime_log_warn(format!(
                "[Live更新] 跳过，任务=读取通知权限，notification_id={}，error={}",
                id, err
            ));
            return;
        }
    };
    let permission = match permission {
        PermissionState::Prompt | PermissionState::PromptWithRationale => {
            match notifications.request_permission() {
                Ok(permission) => permission,
                Err(err) => {
                    runtime_log_warn(format!(
                        "[Live更新] 跳过，任务=请求通知权限，notification_id={}，error={}",
                        id, err
                    ));
                    return;
                }
            }
        }
        state => state,
    };
    if permission == PermissionState::Denied {
        return;
    }
    let mut builder = notifications
        .builder()
        .id(id)
        .title(normalized_title)
        .body(normalized_body)
        .icon("ic_stat_pai");
    if ongoing {
        builder = builder.ongoing();
        if promoted {
            // 官方 live updates：标准样式（BigTextStyle + 进度）+ ongoing +
            // 请求系统提升（API 35+ 生效，低版本 no-op）。
            builder = builder
                .request_promoted_ongoing()
                .large_body(normalized_body)
                .progress(0, 0, true);
        }
    }
    if let Err(err) = builder.show() {
        runtime_log_warn(format!(
            "[Live更新] 失败，任务=发送通知，notification_id={}，error={}",
            id, err
        ));
    }
}

#[cfg(target_os = "android")]
fn live_update_todo_step_text(
    state: &AppState,
    conversation_id: &str,
    ui_language: &str,
) -> Option<String> {
    let meta = conversation_service_v2()
        .get_conversation_meta(state, conversation_id)
        .ok()?;
    let todos = &meta.current_todos;
    if todos.is_empty() {
        return None;
    }
    let active_index = todos
        .iter()
        .position(|item| item.status.trim() == "in_progress")
        .or_else(|| todos.iter().position(|item| item.status.trim() == "pending"))?;
    let content = native_notification_text_excerpt(
        &todos[active_index].content,
        LIVE_UPDATE_BODY_MAX_CHARS,
    );
    if content.trim().is_empty() {
        return None;
    }
    let step = active_index + 1;
    let total = todos.len();
    Some(match ui_language.trim() {
        "en-US" => format!("Step {step}/{total}: {content}"),
        _ => format!("第 {step}/{total} 步：{content}"),
    })
}

#[cfg(target_os = "android")]
fn live_update_chat_meta_title(
    state: &AppState,
    conversation_id: &str,
    ui_language: &str,
    failed: bool,
) -> Option<String> {
    let meta = match conversation_service_v2().get_conversation_meta(state, conversation_id) {
        Ok(meta) => meta,
        Err(err) => {
            runtime_log_warn(format!(
                "[Live更新] 跳过，任务=读取会话通知上下文，conversation_id={}，error={}",
                conversation_id, err
            ));
            return None;
        }
    };
    if !conversation_meta_is_local_normal_chat_for_notification(&meta) {
        return None;
    }
    Some(notification_title_for_conversation_meta(
        state,
        &meta,
        ui_language,
        failed,
    ))
}

#[cfg(target_os = "android")]
fn live_update_chat_started(state: &AppState, conversation_id: &str) {
    let Some(app) = live_update_app_handle(state) else {
        return;
    };
    live_update_keep_alive_chat(state, conversation_id, true);
    let settings = local_chat_notification_settings(state, conversation_id);
    let Some(title) =
        live_update_chat_meta_title(state, conversation_id, settings.ui_language, false)
    else {
        return;
    };
    let body = live_update_todo_step_text(state, conversation_id, settings.ui_language)
        .unwrap_or_else(|| {
            local_chat_notification_text(
                settings.ui_language,
                "正在回复…",
                "正在回覆…",
                "Replying…",
            )
        });
    live_update_owner_set(&CHAT_LIVE_UPDATE_OWNER, conversation_id);
    live_update_send(&app, CHAT_LIVE_UPDATE_NOTIFICATION_ID, &title, &body, true, true);
}

#[cfg(target_os = "android")]
fn live_update_chat_finished(
    state: &AppState,
    conversation_id: &str,
    failed: bool,
    text: &str,
) {
    // 保活集合与通知归属解耦：每个 started 必然对应一个 finished，无条件移除。
    live_update_keep_alive_chat(state, conversation_id, false);
    // 当前 live 通知可能已被其他会话的输出覆盖，只有归属会话结束才更新终态。
    if !live_update_owner_matches(&CHAT_LIVE_UPDATE_OWNER, conversation_id) {
        return;
    }
    live_update_owner_take(&CHAT_LIVE_UPDATE_OWNER);
    let Some(app) = live_update_app_handle(state) else {
        return;
    };
    let settings = local_chat_notification_settings(state, conversation_id);
    let Some(title) =
        live_update_chat_meta_title(state, conversation_id, settings.ui_language, failed)
    else {
        return;
    };
    let excerpt = native_notification_text_excerpt(text, LIVE_UPDATE_BODY_MAX_CHARS);
    let body = if failed {
        let fallback = local_chat_notification_text(
            settings.ui_language,
            "本轮调度失败。",
            "本輪調度失敗。",
            "This round failed.",
        );
        if excerpt.trim().is_empty() {
            fallback
        } else {
            excerpt
        }
    } else {
        let fallback = local_chat_notification_text(
            settings.ui_language,
            "本轮回复完成。",
            "本輪回覆完成。",
            "Finished this reply.",
        );
        if excerpt.trim().is_empty() {
            fallback
        } else {
            excerpt
        }
    };
    // 终态通知非 ongoing，用户可手动划掉；不重复弹完成/失败通知。
    live_update_send(&app, CHAT_LIVE_UPDATE_NOTIFICATION_ID, &title, &body, false, false);
}

#[cfg(target_os = "android")]
fn live_update_goal_changed(
    state: &AppState,
    conversation_id: &str,
    goal: Option<&ConversationGoalState>,
) {
    let Some(app) = live_update_app_handle(state) else {
        return;
    };
    let settings = local_chat_notification_settings(state, conversation_id);
    let Some(title) =
        live_update_chat_meta_title(state, conversation_id, settings.ui_language, false)
    else {
        return;
    };
    let Some(goal) = goal else {
        // 保活集合与通知归属解耦：目标结束/删除无条件移除。
        live_update_keep_alive_goal(state, conversation_id, false);
        // 目标被删除/清除：只有归属会话结束才更新终态。
        if !live_update_owner_matches(&GOAL_LIVE_UPDATE_OWNER, conversation_id) {
            return;
        }
        live_update_owner_take(&GOAL_LIVE_UPDATE_OWNER);
        let body = local_chat_notification_text(
            settings.ui_language,
            "目标已结束。",
            "目標已結束。",
            "Goal finished.",
        );
        live_update_send(&app, GOAL_LIVE_UPDATE_NOTIFICATION_ID, &title, &body, false, false);
        return;
    };
    if goal.status.trim() != "active" {
        // 保活集合与通知归属解耦：目标结束/删除无条件移除。
        live_update_keep_alive_goal(state, conversation_id, false);
        if !live_update_owner_matches(&GOAL_LIVE_UPDATE_OWNER, conversation_id) {
            return;
        }
        live_update_owner_take(&GOAL_LIVE_UPDATE_OWNER);
        let body = local_chat_notification_text(
            settings.ui_language,
            "目标已结束。",
            "目標已結束。",
            "Goal finished.",
        );
        live_update_send(&app, GOAL_LIVE_UPDATE_NOTIFICATION_ID, &title, &body, false, false);
        return;
    }
    // 目标进行中：记录归属会话并发送 ongoing 常驻通知。
    live_update_keep_alive_goal(state, conversation_id, true);
    live_update_owner_set(&GOAL_LIVE_UPDATE_OWNER, conversation_id);
    let objective = native_notification_text_excerpt(
        &goal.objective,
        LIVE_UPDATE_BODY_MAX_CHARS,
    );
    let body = live_update_todo_step_text(state, conversation_id, settings.ui_language)
        .unwrap_or_else(|| {
            if objective.trim().is_empty() {
                local_chat_notification_text(
                    settings.ui_language,
                    "目标进行中…",
                    "目標進行中…",
                    "Goal in progress…",
                )
            } else {
                objective
            }
        });
    live_update_send(&app, GOAL_LIVE_UPDATE_NOTIFICATION_ID, &title, &body, true, true);
}

#[cfg(not(target_os = "android"))]
fn live_update_chat_started(_state: &AppState, _conversation_id: &str) {}

#[cfg(not(target_os = "android"))]
fn live_update_chat_finished(
    _state: &AppState,
    _conversation_id: &str,
    _failed: bool,
    _text: &str,
) {
}

#[cfg(not(target_os = "android"))]
fn live_update_goal_changed(
    _state: &AppState,
    _conversation_id: &str,
    _goal: Option<&ConversationGoalState>,
) {
}
