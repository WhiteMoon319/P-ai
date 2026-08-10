use std::str::FromStr;

const MAIN_TRAY_ID: &str = "easy-call-tray";
const WINDOW_LAYOUTS_FILE_NAME: &str = "window_layouts.json";
const WINDOW_DIAGNOSTIC_LOG_FILE_NAME: &str = "window_diagnostics.log";
const FILE_READER_WINDOW_LABEL: &str = "file-reader";
const NEAR_FULLSCREEN_RESTORE_RATIO: f64 = 0.92;
const WINDOW_LAYOUT_SAVE_INTERVAL: std::time::Duration = std::time::Duration::from_secs(60);

static DETACHED_CHAT_WINDOWS: OnceLock<Mutex<std::collections::HashMap<String, String>>> =
    OnceLock::new();

static OFFSCREEN_LAYOUT_LOGGED_ONCE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

static CHAT_WINDOW_SIDE_EXPANSION: OnceLock<Mutex<ChatWindowSideExpansion>> = OnceLock::new();
static WINDOW_LAYOUT_STORE: OnceLock<Arc<Mutex<WindowLayoutStore>>> = OnceLock::new();
static WINDOW_LAYOUT_SAVE_SENDER: OnceLock<std::sync::mpsc::Sender<PersistedWindowLayouts>> =
    OnceLock::new();

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ChatWindowSideExpansion {
    left_physical: u32,
    right_physical: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PhysicalWindowRect {
    x: i32,
    y: i32,
    width: u32,
    height: u32,
}

#[derive(Debug)]
struct WindowLayoutStore {
    layouts: PersistedWindowLayouts,
}

fn chat_window_side_expansion() -> &'static Mutex<ChatWindowSideExpansion> {
    CHAT_WINDOW_SIDE_EXPANSION.get_or_init(|| Mutex::new(ChatWindowSideExpansion::default()))
}

fn read_chat_window_side_expansion() -> Result<ChatWindowSideExpansion, String> {
    chat_window_side_expansion()
        .lock()
        .map(|state| *state)
        .map_err(|err| format!("读取聊天窗口侧栏外扩状态失败：{err}"))
}

fn write_chat_window_side_expansion(
    update: impl FnOnce(&mut ChatWindowSideExpansion),
) -> Result<ChatWindowSideExpansion, String> {
    let mut state = chat_window_side_expansion()
        .lock()
        .map_err(|err| format!("更新聊天窗口侧栏外扩状态失败：{err}"))?;
    update(&mut state);
    Ok(*state)
}

fn calculate_chat_window_expand_target(
    window: PhysicalWindowRect,
    screen: PhysicalWindowRect,
    side: &str,
    requested_width: u32,
) -> Option<PhysicalWindowRect> {
    if requested_width == 0 {
        return None;
    }
    if side != "left" && side != "right" {
        return None;
    }
    if window.width.saturating_add(requested_width) > screen.width {
        return None;
    }
    Some(PhysicalWindowRect {
        x: if side == "left" {
            window.x.saturating_sub(requested_width as i32)
        } else {
            window.x
        },
        y: window.y,
        width: window.width.saturating_add(requested_width),
        height: window.height,
    })
}

fn calculate_chat_window_collapse_target(
    window: PhysicalWindowRect,
    side: &str,
    applied_width: u32,
) -> Option<PhysicalWindowRect> {
    if applied_width == 0 || window.width <= applied_width {
        return None;
    }
    Some(PhysicalWindowRect {
        x: if side == "left" {
            window.x.saturating_add(applied_width as i32)
        } else {
            window.x
        },
        y: window.y,
        width: window.width - applied_width,
        height: window.height,
    })
}

#[cfg(test)]
mod chat_window_side_expansion_tests {
    use super::*;

    fn rect(x: i32, width: u32) -> PhysicalWindowRect {
        PhysicalWindowRect {
            x,
            y: 40,
            width,
            height: 900,
        }
    }

    #[test]
    fn expands_left_when_full_width_fits() {
        let target = calculate_chat_window_expand_target(
            rect(500, 600),
            rect(0, 1920),
            "left",
            320,
        );
        assert_eq!(target, Some(rect(180, 920)));
    }

    #[test]
    fn keeps_current_layout_when_expanded_window_would_exceed_screen_width() {
        let target = calculate_chat_window_expand_target(
            rect(100, 1700),
            rect(0, 1920),
            "left",
            320,
        );
        assert_eq!(target, None);
    }

    #[test]
    fn allows_left_position_to_extend_past_screen_edge_when_total_width_fits() {
        let target = calculate_chat_window_expand_target(
            rect(100, 600),
            rect(0, 1920),
            "left",
            320,
        );
        assert_eq!(target, Some(rect(-220, 920)));
    }

    #[test]
    fn expands_right_without_moving_the_left_edge() {
        let target = calculate_chat_window_expand_target(
            rect(500, 600),
            rect(0, 1920),
            "right",
            320,
        );
        assert_eq!(target, Some(rect(500, 920)));
    }

    #[test]
    fn collapses_left_back_to_the_base_rect() {
        let target = calculate_chat_window_collapse_target(rect(180, 920), "left", 320);
        assert_eq!(target, Some(rect(500, 600)));
    }

    #[test]
    fn chat_window_default_size_matches_tauri_config() {
        assert_eq!(default_window_size("chat"), (618, 1000));
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PersistedWindowLayouts {
    #[serde(default)]
    windows: std::collections::HashMap<String, PersistedWindowLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct PersistedWindowLayout {
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    x: Option<i32>,
    #[serde(default)]
    y: Option<i32>,
    #[serde(default)]
    maximized: bool,
}

fn window_layouts_path(data_path: &PathBuf) -> PathBuf {
    app_layout_state_dir(data_path).join(WINDOW_LAYOUTS_FILE_NAME)
}


fn read_window_layouts(data_path: &PathBuf) -> Result<PersistedWindowLayouts, String> {
    let path = window_layouts_path(data_path);
    if !path.exists() {
        return Ok(PersistedWindowLayouts::default());
    }
    read_json_file::<PersistedWindowLayouts>(&path, "window layouts")
}

fn save_window_layouts(data_path: &PathBuf, layouts: &PersistedWindowLayouts) -> Result<(), String> {
    write_json_file_atomic(
        &window_layouts_path(data_path),
        layouts,
        "window layouts",
    )
}

fn window_layout_store() -> Result<Arc<Mutex<WindowLayoutStore>>, String> {
    WINDOW_LAYOUT_STORE
        .get()
        .cloned()
        .ok_or_else(|| "窗口布局内存缓存尚未初始化".to_string())
}

fn window_layouts_snapshot() -> Result<PersistedWindowLayouts, String> {
    let store = window_layout_store()?;
    store
        .lock()
        .map(|state| state.layouts.clone())
        .map_err(|err| format!("读取窗口布局内存缓存失败：{err}"))
}

fn enqueue_window_layout_save(layouts: PersistedWindowLayouts) {
    let Some(sender) = WINDOW_LAYOUT_SAVE_SENDER.get() else {
        runtime_log_warn("[窗口布局] 保存队列尚未初始化，跳过异步写盘".to_string());
        return;
    };
    if let Err(err) = sender.send(layouts) {
        runtime_log_warn(format!("[窗口布局] 写入异步保存队列失败：{err}"));
    }
}

fn run_window_layout_save_worker(
    data_path: PathBuf,
    receiver: std::sync::mpsc::Receiver<PersistedWindowLayouts>,
) {
    let mut pending = match receiver.recv() {
        Ok(layouts) => layouts,
        Err(_) => return,
    };
    let mut next_save_at = std::time::Instant::now() + WINDOW_LAYOUT_SAVE_INTERVAL;
    loop {
        let wait = next_save_at.saturating_duration_since(std::time::Instant::now());
        match receiver.recv_timeout(wait) {
            Ok(layouts) => pending = layouts,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                if let Err(err) = save_window_layouts(&data_path, &pending) {
                    runtime_log_warn(format!(
                        "[窗口布局] 异步写盘失败，将在下一轮重试：error={err}"
                    ));
                    next_save_at = std::time::Instant::now() + WINDOW_LAYOUT_SAVE_INTERVAL;
                    continue;
                }
                next_save_at = std::time::Instant::now() + WINDOW_LAYOUT_SAVE_INTERVAL;
                match receiver.try_recv() {
                    Ok(layouts) => pending = layouts,
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        pending = match receiver.recv() {
                            Ok(layouts) => layouts,
                            Err(_) => return,
                        };
                        next_save_at = std::time::Instant::now() + WINDOW_LAYOUT_SAVE_INTERVAL;
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
                }
            }
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
        }
    }
}


fn upsert_window_layout<F>(label: &str, update: F) -> Result<(), String>
where
    F: FnOnce(&mut PersistedWindowLayout),
{
    let store = window_layout_store()?;
    let snapshot = {
        let mut state = store
            .lock()
            .map_err(|err| format!("更新窗口布局内存缓存失败：{err}"))?;
        let entry = state.layouts.windows.entry(label.to_string()).or_default();
        let previous = entry.clone();
        update(entry);
        if *entry == previous {
            return Ok(());
        }
        state.layouts.clone()
    };
    enqueue_window_layout_save(snapshot);
    Ok(())
}

fn default_window_size(label: &str) -> (u32, u32) {
    match label {
        "main" => (900_u32, 900_u32),
        "chat" => (618_u32, 1000_u32),
        "archives" => (900_u32, 900_u32),
        FILE_READER_WINDOW_LABEL => (1040_u32, 760_u32),
        _ => (900_u32, 900_u32),
    }
}

fn minimum_window_size(label: &str) -> (u32, u32) {
    match label {
        "main" => (900_u32, 600_u32),
        "chat" => (520_u32, 520_u32),
        "archives" => (560_u32, 560_u32),
        FILE_READER_WINDOW_LABEL => (720_u32, 520_u32),
        _ => (520_u32, 520_u32),
    }
}

fn restore_window_minimum_size(label: &str) -> (u32, u32) {
    match label {
        "main" => (900_u32, 600_u32),
        _ => minimum_window_size(label),
    }
}

fn detached_chat_windows() -> &'static Mutex<std::collections::HashMap<String, String>> {
    DETACHED_CHAT_WINDOWS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn detached_chat_window_for_conversation(conversation_id: &str) -> Option<String> {
    let cid = conversation_id.trim();
    if cid.is_empty() {
        return None;
    }
    let guard = detached_chat_windows().lock().unwrap_or_else(|poison| {
        runtime_log_info(format!(
            "[独立聊天窗口] 会话到窗口映射锁已中毒，继续恢复读取：error={:?}",
            poison
        ));
        poison.into_inner()
    });
    guard.get(cid).cloned()
}

fn register_detached_chat_window(conversation_id: &str, label: &str) -> Result<(), String> {
    let cid = conversation_id.trim();
    let window_label = label.trim();
    if cid.is_empty() || window_label.is_empty() {
        return Err("conversationId 和 windowLabel 不能为空".to_string());
    }
    let mut guard = detached_chat_windows()
        .lock()
        .map_err(|err| format!("锁定独立聊天窗口映射失败：{err}"))?;
    guard.insert(cid.to_string(), window_label.to_string());
    Ok(())
}

fn unregister_detached_chat_window_by_label(label: &str) -> Option<String> {
    let window_label = label.trim();
    if window_label.is_empty() {
        return None;
    }
    let mut guard = detached_chat_windows().lock().ok()?;
    let conversation_id = guard
        .iter()
        .find_map(|(conversation_id, mapped_label)| {
            if mapped_label == window_label {
                Some(conversation_id.clone())
            } else {
                None
            }
        })?;
    guard.remove(&conversation_id);
    Some(conversation_id)
}








fn logical_to_physical_px(value: u32, scale_factor: f64) -> i32 {
    ((value as f64) * scale_factor.max(0.1)).round() as i32
}









#[cfg(not(target_os = "android"))]
#[tauri::command]










#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn sync_default_tray_icon(app: &tauri::AppHandle) -> Result<(), String> {
    let tray = app
        .tray_by_id(MAIN_TRAY_ID)
        .ok_or_else(|| "Tray icon not found".to_string())?;

    tray
        .set_icon(app.default_window_icon().cloned())
        .map_err(|err| format!("Set tray icon failed: {err}"))
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "android")))]
fn sync_default_tray_icon(_app: &NativeAppHandle) -> Result<(), String> {
    Ok(())
}



#[cfg(target_os = "android")]
fn toggle_window_maximize_with_default_restore(
    _app: &NativeAppHandle,
    _label: &str,
) -> Result<bool, String> {
    Ok(false)
}


#[cfg(target_os = "android")]
fn start_window_drag_with_default_restore(_app: &NativeAppHandle, _label: &str) -> Result<(), String> {
    Ok(())
}


fn normalize_hotkey_for_parser(raw: &str) -> String {
    let mut text = raw.trim().to_string();
    if text.is_empty() {
        return "Alt+Backquote".to_string();
    }
    text = text.replace('·', "`");
    text = text.replace('＋', "+");
    text
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn parse_hotkey(raw: &str) -> Result<Shortcut, String> {
    let normalized = normalize_hotkey_for_parser(raw);
    Shortcut::from_str(&normalized)
        .or_else(|_| Shortcut::from_str("Alt+Backquote"))
        .map_err(|err| format!("Parse hotkey failed: {err}"))
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn register_default_hotkey(app: &tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = read_config(&state.config_path).unwrap_or_default();
    register_hotkeys_from_config(app, &config)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "android")))]
fn register_default_hotkey(_app: &NativeAppHandle) -> Result<(), String> {
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn register_hotkey_from_config(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    register_hotkeys_from_config(app, config)
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "android")))]
fn register_hotkey_from_config(_app: &NativeAppHandle, _config: &AppConfig) -> Result<(), String> {
    Ok(())
}

#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn register_hotkeys_from_config(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), String> {
    let summon_shortcut = parse_hotkey(&config.hotkey)?;
    let manager = app.global_shortcut();
    manager
        .unregister_all()
        .map_err(|err| format!("Unregister hotkeys failed: {err}"))?;
    manager
        .register(summon_shortcut)
        .map_err(|err| format!("Register summon hotkey failed: {err}"))
}

fn default_hotkey_label() -> String {
    "Alt+·".to_string()
}

fn normalize_hotkey_label(value: &str) -> String {
    let raw = value.trim();
    if raw.is_empty() {
        return default_hotkey_label();
    }
    let normalized = raw.replace('＋', "+").replace('`', "·");
    let upper = normalized.to_uppercase();
    if upper.contains("BACKQUOTE") {
        return normalized
            .replace("Backquote", "·")
            .replace("BACKQUOTE", "·")
            .replace("backquote", "·");
    }
    normalized
}

fn ensure_hotkey_config_normalized(config: &mut AppConfig) {
    config.hotkey = normalize_hotkey_label(&config.hotkey);
    if config.hotkey.trim().is_empty() {
        config.hotkey = default_hotkey_label();
    }
}




// ==================== 运行日志窗口 ====================

const RUNTIME_LOGS_WINDOW_LABEL: &str = "runtime-logs";


#[cfg(any(target_os = "windows", target_os = "macos", target_os = "linux"))]
fn build_tray(app: &tauri::AppHandle) -> Result<(), String> {
    let config = MenuItem::with_id(app, "config", "配置", true, None::<&str>)
        .map_err(|err| format!("Create tray menu item failed: {err}"))?;
    let chat = MenuItem::with_id(app, "chat", "对话", true, None::<&str>)
        .map_err(|err| format!("Create tray menu item failed: {err}"))?;
    let file_reader = MenuItem::with_id(app, "file-reader", "文件浏览器", true, None::<&str>)
        .map_err(|err| format!("Create tray menu item failed: {err}"))?;
    let archives = MenuItem::with_id(app, "archives", "归档", true, None::<&str>)
        .map_err(|err| format!("Create tray menu item failed: {err}"))?;
    let runtime_logs = MenuItem::with_id(app, "runtime-logs", "运行日志", true, None::<&str>)
        .map_err(|err| format!("Create tray menu item failed: {err}"))?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)
        .map_err(|err| format!("Create tray menu item failed: {err}"))?;

    let menu = Menu::with_items(app, &[&config, &chat, &file_reader, &archives, &runtime_logs, &quit])
        .map_err(|err| format!("Create tray menu failed: {err}"))?;

    let mut tray = TrayIconBuilder::with_id(MAIN_TRAY_ID).menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }

    tray.tooltip("P-ai")
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                dispatch_tray_action(tray.app_handle(), "left_click", "chat");
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            if id == "config" {
                dispatch_tray_action(app, "menu", "config");
            } else if id == "chat" {
                dispatch_tray_action(app, "menu", "chat");
            } else if id == "file-reader" {
                dispatch_tray_action(app, "menu", "file-reader");
            } else if id == "archives" {
                dispatch_tray_action(app, "menu", "archives");
            } else if id == "runtime-logs" {
                dispatch_tray_action(app, "menu", "runtime-logs");
            } else if id == "quit" {
                runtime_log_info(format!("[托盘] 收到动作：source=menu，action=quit"));
                graceful_exit_app(app, 0);
            }
        })
        .build(app)
        .map_err(|err| format!("Build tray failed: {err}"))?;

    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux", target_os = "android")))]
fn build_tray(_app: &NativeAppHandle) -> Result<(), String> {
    Ok(())
}


// ==================== WebView 心跳崩溃恢复 ====================

const WEBVIEW_HEARTBEAT_INTERVAL_MS: u64 = 5000;
const WEBVIEW_HEARTBEAT_MAX_MISS: u32 = 3;
const WEBVIEW_MONITORED_LABELS: &[&str] = &["main", "chat"];

static WEBVIEW_PONG_TIMESTAMPS: OnceLock<Mutex<std::collections::HashMap<String, std::time::Instant>>> =
    OnceLock::new();

fn webview_pong_timestamps() -> &'static Mutex<std::collections::HashMap<String, std::time::Instant>> {
    WEBVIEW_PONG_TIMESTAMPS.get_or_init(|| Mutex::new(std::collections::HashMap::new()))
}

fn webview_record_pong(label: &str) {
    if let Ok(mut map) = webview_pong_timestamps().lock() {
        map.insert(label.to_string(), std::time::Instant::now());
    }
}

fn webview_window_url_for_label(label: &str) -> &'static str {
    match label {
        "main" => "index.html",
        "chat" => "chat.html",
        "archives" => "archives.html",
        _ => "index.html",
    }
}


