use std::str::FromStr;

const MAIN_TRAY_ID: &str = "easy-call-tray";
const WINDOW_LAYOUTS_FILE_NAME: &str = "window_layouts.json";
const WINDOW_DIAGNOSTIC_LOG_FILE_NAME: &str = "window_diagnostics.log";
const FILE_READER_WINDOW_LABEL: &str = "file-reader";
const NEAR_FULLSCREEN_RESTORE_RATIO: f64 = 0.92;

static DETACHED_CHAT_WINDOWS: OnceLock<Mutex<std::collections::HashMap<String, String>>> =
    OnceLock::new();

static OFFSCREEN_LAYOUT_LOGGED_ONCE: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct PersistedWindowLayouts {
    #[serde(default)]
    windows: std::collections::HashMap<String, PersistedWindowLayout>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

fn append_window_diagnostic_log(app: &AppHandle, message: String) {
    runtime_log_info(message.clone());

    let state = app.state::<AppState>();
    let dir = app_layout_state_dir(&state.data_path);
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(WINDOW_DIAGNOSTIC_LOG_FILE_NAME);
    let line = format!("{} {}\n", now_iso(), message);
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = std::io::Write::write_all(&mut file, line.as_bytes());
    }
}

fn load_window_layouts(data_path: &PathBuf) -> PersistedWindowLayouts {
    let path = window_layouts_path(data_path);
    if !path.exists() {
        return PersistedWindowLayouts::default();
    }
    read_json_file::<PersistedWindowLayouts>(&path, "window layouts").unwrap_or_default()
}

fn save_window_layouts(data_path: &PathBuf, layouts: &PersistedWindowLayouts) -> Result<(), String> {
    write_json_file_atomic(
        &window_layouts_path(data_path),
        layouts,
        "window layouts",
    )
}

fn upsert_window_layout<F>(app: &AppHandle, label: &str, update: F) -> Result<(), String>
where
    F: FnOnce(&mut PersistedWindowLayout),
{
    let state = app.state::<AppState>();
    let mut layouts = load_window_layouts(&state.data_path);
    let entry = layouts.windows.entry(label.to_string()).or_default();
    update(entry);
    save_window_layouts(&state.data_path, &layouts)
}

fn default_window_size(label: &str) -> (u32, u32) {
    match label {
        "main" => (900_u32, 900_u32),
        "chat" => (900_u32, 900_u32),
        "archives" => (900_u32, 900_u32),
        "quick-setup" => (800_u32, 600_u32),
        FILE_READER_WINDOW_LABEL => (1040_u32, 760_u32),
        _ => (900_u32, 900_u32),
    }
}

fn minimum_window_size(label: &str) -> (u32, u32) {
    match label {
        "main" => (900_u32, 600_u32),
        "chat" => (520_u32, 520_u32),
        "archives" => (560_u32, 560_u32),
        "quick-setup" => (800_u32, 600_u32),
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

fn is_fixed_window_size(label: &str) -> bool {
    matches!(label, "quick-setup")
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

fn focus_file_reader_window(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(FILE_READER_WINDOW_LABEL)
        .ok_or_else(|| "文件阅读窗口不存在".to_string())?;
    let _ = window.unminimize();
    let _ = window.show();
    ensure_window_visible_after_show(app, FILE_READER_WINDOW_LABEL, "focus_file_reader_window");
    window
        .set_focus()
        .map_err(|err| format!("聚焦文件阅读窗口失败：{err}"))
}

fn emit_file_reader_open_path(app: &AppHandle, path: &str) -> Result<(), String> {
    app.emit_to(
        FILE_READER_WINDOW_LABEL,
        "file-reader-open-path",
        serde_json::json!({ "path": path }),
    )
    .map_err(|err| format!("投递文件阅读请求失败：{err}"))
}

fn open_file_reader_window(app: &AppHandle, path: String) -> Result<String, String> {
    let normalized_path = path.trim().to_string();
    if normalized_path.is_empty() {
        return Err("path 不能为空".to_string());
    }

    if app.get_webview_window(FILE_READER_WINDOW_LABEL).is_some() {
        focus_file_reader_window(app)?;
        emit_file_reader_open_path(app, &normalized_path)?;
        return Ok(FILE_READER_WINDOW_LABEL.to_string());
    }

    schedule_file_reader_window_creation(app, normalized_path)?;
    Ok(FILE_READER_WINDOW_LABEL.to_string())
}

fn show_file_reader_window(app: &AppHandle) -> Result<String, String> {
    if app.get_webview_window(FILE_READER_WINDOW_LABEL).is_some() {
        focus_file_reader_window(app)?;
        return Ok(FILE_READER_WINDOW_LABEL.to_string());
    }

    schedule_file_reader_window_creation(app, String::new())?;
    Ok(FILE_READER_WINDOW_LABEL.to_string())
}

fn schedule_file_reader_window_creation(app: &AppHandle, path: String) -> Result<(), String> {
    let app_handle = app.clone();
    std::thread::Builder::new()
        .name("file-reader-window-create".to_string())
        .spawn(move || {
            let started_at = std::time::Instant::now();
            runtime_log_info(format!("[文件阅读窗口] 开始创建窗口：window_label={}", FILE_READER_WINDOW_LABEL));
            if app_handle.get_webview_window(FILE_READER_WINDOW_LABEL).is_some() {
                let _ = focus_file_reader_window(&app_handle);
                let _ = emit_file_reader_open_path(&app_handle, &path);
                return;
            }

            let encoded_path = urlencoding::encode(&path);
            let url = format!("file-reader.html?path={encoded_path}");
            let window = match tauri::WebviewWindowBuilder::new(
                &app_handle,
                FILE_READER_WINDOW_LABEL,
                tauri::WebviewUrl::App(url.into()),
            )
            .title("PAI - 文件阅读")
            .inner_size(1040.0, 760.0)
            .min_inner_size(720.0, 520.0)
            .resizable(true)
            .decorations(false)
            .shadow(true)
            .visible(false)
            .build()
            {
                Ok(window) => window,
                Err(err) => {
                    runtime_log_error(format!(
                        "[文件阅读窗口] 创建失败：window_label={}，error={}",
                        FILE_READER_WINDOW_LABEL,
                        err
                    ));
                    return;
                }
            };

            if let Err(err) = apply_window_layout_before_show(&app_handle, FILE_READER_WINDOW_LABEL) {
                runtime_log_error(format!(
                    "[文件阅读窗口] 应用窗口布局失败：window_label={}，error={}",
                    FILE_READER_WINDOW_LABEL,
                    err
                ));
            }
            let _ = window.unminimize();
            let _ = window.show();
            ensure_window_visible_after_show(
                &app_handle,
                FILE_READER_WINDOW_LABEL,
                "show_file_reader_window",
            );
            let _ = window.set_focus();
            runtime_log_info(format!(
                "[文件阅读窗口] 窗口已显示：window_label={}，elapsed_ms={}",
                FILE_READER_WINDOW_LABEL,
                started_at.elapsed().as_millis()
            ));
        })
        .map(|_| ())
        .map_err(|err| format!("调度创建文件阅读窗口失败：{err}"))
}

fn monitor_logical_size(monitor: &tauri::Monitor) -> tauri::LogicalSize<f64> {
    monitor
        .size()
        .to_logical::<f64>(monitor.scale_factor().max(0.1))
}

fn default_window_size_for_monitor(label: &str, monitor: &tauri::Monitor) -> (u32, u32) {
    let fallback = default_window_size(label);
    if matches!(label, "quick-setup") {
        return fallback;
    }
    let logical = monitor_logical_size(monitor);
    let min_side = logical.width.min(logical.height);
    if !min_side.is_finite() || min_side <= 1.0 {
        return fallback;
    }
    let target = (min_side * 0.8).round().max(1.0) as u32;
    (target, target)
}

fn logical_to_physical_px(value: u32, scale_factor: f64) -> i32 {
    ((value as f64) * scale_factor.max(0.1)).round() as i32
}

fn preferred_window_monitor(window: &tauri::WebviewWindow) -> Option<tauri::Monitor> {
    if let Ok(Some(monitor)) = window.primary_monitor() {
        return Some(monitor);
    }
    if let Some(monitor) = window
        .available_monitors()
        .ok()
        .and_then(|mut monitors| monitors.drain(..).next())
    {
        return Some(monitor);
    }
    window.current_monitor().ok().flatten()
}

fn resolved_window_size_for_monitor(
    label: &str,
    monitor: &tauri::Monitor,
    width: Option<u32>,
    height: Option<u32>,
) -> (u32, u32) {
    let (default_width, default_height) = default_window_size_for_monitor(label, monitor);
    let (min_width, min_height) = minimum_window_size(label);
    let (restore_min_width, restore_min_height) = restore_window_minimum_size(label);
    let monitor_logical = monitor_logical_size(monitor);
    let max_width = monitor_logical.width.max(1.0).round() as u32;
    let max_height = monitor_logical.height.max(1.0).round() as u32;
    let target_width = if is_fixed_window_size(label) {
        default_width
    } else {
        width.unwrap_or(default_width)
    };
    let target_height = if is_fixed_window_size(label) {
        default_height
    } else {
        height.unwrap_or(default_height)
    };
    (
        target_width
            .max(restore_min_width.min(max_width))
            .max(min_width.min(max_width))
            .min(max_width),
        target_height
            .max(restore_min_height.min(max_height))
            .max(min_height.min(max_height))
            .min(max_height),
    )
}

fn window_size_is_near_fullscreen(width: u32, height: u32, monitor: &tauri::Monitor) -> bool {
    let monitor_logical = monitor_logical_size(monitor);
    if !monitor_logical.width.is_finite() || !monitor_logical.height.is_finite() {
        return false;
    }
    if monitor_logical.width <= 1.0 || monitor_logical.height <= 1.0 {
        return false;
    }
    let width_ratio = width as f64 / monitor_logical.width;
    let height_ratio = height as f64 / monitor_logical.height;
    width_ratio >= NEAR_FULLSCREEN_RESTORE_RATIO && height_ratio >= NEAR_FULLSCREEN_RESTORE_RATIO
}

fn saved_window_layout_is_near_fullscreen(
    app: &AppHandle,
    label: &str,
    monitor: &tauri::Monitor,
) -> bool {
    let state = app.state::<AppState>();
    let layouts = load_window_layouts(&state.data_path);
    let Some(saved) = layouts.windows.get(label) else {
        return false;
    };
    let (Some(width), Some(height)) = (saved.width, saved.height) else {
        return false;
    };
    window_size_is_near_fullscreen(width, height, monitor)
}

fn webview_window_inner_size_logical(
    window: &tauri::WebviewWindow,
) -> Result<(u32, u32), String> {
    let inner_size = window
        .inner_size()
        .map_err(|err| format!("Read window inner size failed: {err}"))?;
    let scale_factor = window
        .scale_factor()
        .map_err(|err| format!("Read window scale factor failed: {err}"))?;
    let inner_size_logical = inner_size.to_logical::<f64>(scale_factor.max(0.1));
    Ok((
        inner_size_logical.width.round().max(1.0) as u32,
        inner_size_logical.height.round().max(1.0) as u32,
    ))
}

fn current_window_size_is_near_fullscreen(
    window: &tauri::WebviewWindow,
    monitor: &tauri::Monitor,
) -> bool {
    webview_window_inner_size_logical(window)
        .map(|(width, height)| window_size_is_near_fullscreen(width, height, monitor))
        .unwrap_or(false)
}

fn window_rect_is_visible_on_any_monitor(
    monitors: &[tauri::Monitor],
    x: i32,
    y: i32,
    width: u32,
    height: u32,
) -> bool {
    let right = x.saturating_add(width as i32);
    let bottom = y.saturating_add(height as i32);
    monitors.iter().any(|monitor| {
        let monitor_x = monitor.position().x;
        let monitor_y = monitor.position().y;
        let monitor_right = monitor_x.saturating_add(monitor.size().width as i32);
        let monitor_bottom = monitor_y.saturating_add(monitor.size().height as i32);
        let visible_width = (right.min(monitor_right) - x.max(monitor_x)).max(0);
        let visible_height = (bottom.min(monitor_bottom) - y.max(monitor_y)).max(0);
        visible_width >= 80 && visible_height >= 80
    })
}

fn position_window_on_monitor(
    window: &tauri::WebviewWindow,
    label: &str,
    monitor: &tauri::Monitor,
    width: Option<u32>,
    height: Option<u32>,
) {
    let (resolved_width, resolved_height) =
        resolved_window_size_for_monitor(label, monitor, width, height);
    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        resolved_width as f64,
        resolved_height as f64,
    )));
    let margin = 24_i32;
    let resolved_width_physical = logical_to_physical_px(resolved_width, monitor.scale_factor());
    let x = monitor.position().x + monitor.size().width as i32 - resolved_width_physical - margin;
    let y = monitor.position().y + margin;
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
}

fn restore_window_to_default_drag_size(
    window: &tauri::WebviewWindow,
    label: &str,
    monitor: &tauri::Monitor,
) -> Result<(), String> {
    let outer_position = window
        .outer_position()
        .map_err(|err| format!("Read window outer position failed: {err}"))?;
    let outer_size = window
        .outer_size()
        .map_err(|err| format!("Read window outer size failed: {err}"))?;
    let cursor_position = window
        .cursor_position()
        .map_err(|err| format!("Read cursor position failed: {err}"))?;
    let (resolved_width, resolved_height) =
        resolved_window_size_for_monitor(label, monitor, None, None);
    let resolved_width_physical = logical_to_physical_px(resolved_width, monitor.scale_factor());
    let cursor_offset_x = (cursor_position.x - outer_position.x as f64)
        .clamp(0.0, outer_size.width.max(1) as f64);
    let cursor_anchor_ratio = if outer_size.width > 0 {
        (cursor_offset_x / outer_size.width as f64).clamp(0.15, 0.85)
    } else {
        0.5
    };
    let cursor_offset_y = (cursor_position.y - outer_position.y as f64).clamp(12.0, 48.0);
    let monitor_left = monitor.position().x;
    let monitor_top = monitor.position().y;
    let monitor_right = monitor_left.saturating_add(monitor.size().width as i32);
    let max_x = monitor_right.saturating_sub(resolved_width_physical);
    let target_x =
        (cursor_position.x.round() as i32) - (resolved_width_physical as f64 * cursor_anchor_ratio).round() as i32;
    let clamped_x = target_x.clamp(monitor_left, max_x.max(monitor_left));
    let target_y = (cursor_position.y.round() as i32) - cursor_offset_y.round() as i32;
    let clamped_y = target_y.max(monitor_top);

    let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
        resolved_width as f64,
        resolved_height as f64,
    )));
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        clamped_x, clamped_y,
    )));
    Ok(())
}

fn log_offscreen_layout_reset(
    app: &AppHandle,
    label: &str,
    reason: &str,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    monitor_count: usize,
) {
    if !OFFSCREEN_LAYOUT_LOGGED_ONCE.swap(true, std::sync::atomic::Ordering::Relaxed) {
        append_window_diagnostic_log(
            app,
            format!(
                "[窗口] 检测到离屏窗口布局，已重置到可见区域：label={}，reason={}，x={}，y={}，width={}，height={}，monitor_count={}",
                label.trim(),
                reason,
                x,
                y,
                width,
                height,
                monitor_count
            ),
        );
    }
}

fn ensure_window_visible_after_show(app: &AppHandle, label: &str, reason: &str) {
    let Some(window) = app.get_webview_window(label) else {
        return;
    };
    let Ok(monitors) = window.available_monitors() else {
        return;
    };
    if monitors.is_empty() {
        return;
    }
    let Ok(position) = window.outer_position() else {
        return;
    };
    let Ok(size) = window.outer_size() else {
        return;
    };
    if window_rect_is_visible_on_any_monitor(
        &monitors,
        position.x,
        position.y,
        size.width,
        size.height,
    ) {
        return;
    }
    let Some(monitor) = preferred_window_monitor(&window) else {
        return;
    };
    log_offscreen_layout_reset(
        app,
        label,
        reason,
        position.x,
        position.y,
        size.width,
        size.height,
        monitors.len(),
    );
    position_window_on_monitor(&window, label, &monitor, None, None);
}

fn apply_window_layout_before_show(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' not found"))?;
    let (min_width, min_height) = minimum_window_size(label);
    let _ = window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
        min_width as f64,
        min_height as f64,
    ))));
    let state = app.state::<AppState>();
    let layouts = load_window_layouts(&state.data_path);
    let saved = layouts.windows.get(label);
    let fallback_monitor = preferred_window_monitor(&window);

    if matches!(label, "quick-setup") {
        if let Some(monitor) = fallback_monitor.as_ref() {
            position_window_on_monitor(&window, label, monitor, None, None);
        } else {
            let (width, height) = default_window_size(label);
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                width as f64,
                height as f64,
            )));
        }
        return Ok(());
    }

    if let Some(saved) = saved {
        if let Some(monitor) = fallback_monitor.as_ref() {
            let preferred_width = saved.width;
            let preferred_height = saved.height;
            let (resolved_width, resolved_height) =
                resolved_window_size_for_monitor(label, monitor, preferred_width, preferred_height);
            let resolved_width_physical =
                logical_to_physical_px(resolved_width, monitor.scale_factor()) as u32;
            let resolved_height_physical =
                logical_to_physical_px(resolved_height, monitor.scale_factor()) as u32;
            let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                resolved_width as f64,
                resolved_height as f64,
            )));
            if let (Some(x), Some(y)) = (saved.x, saved.y) {
                let monitors = window.available_monitors().unwrap_or_default();
                if !monitors.is_empty()
                    && window_rect_is_visible_on_any_monitor(
                        &monitors,
                        x,
                        y,
                        resolved_width_physical,
                        resolved_height_physical,
                    )
                {
                    let _ = window.set_position(Position::Physical(PhysicalPosition::new(x, y)));
                } else {
                    let reason = if monitors.is_empty() {
                        "monitor_list_empty"
                    } else {
                        "saved_position_offscreen"
                    };
                    log_offscreen_layout_reset(
                        app,
                        label,
                        reason,
                        x,
                        y,
                        resolved_width_physical,
                        resolved_height_physical,
                        monitors.len(),
                    );
                    position_window_on_monitor(
                        &window,
                        label,
                        monitor,
                        Some(resolved_width),
                        Some(resolved_height),
                    );
                }
            } else {
                position_window_on_monitor(
                    &window,
                    label,
                    monitor,
                    Some(resolved_width),
                    Some(resolved_height),
                );
            }
        } else {
            if let (Some(width), Some(height)) = (saved.width, saved.height) {
                let (restore_min_width, restore_min_height) = restore_window_minimum_size(label);
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                    width.max(restore_min_width) as f64,
                    height.max(restore_min_height) as f64,
                )));
            } else {
                let (width, height) = default_window_size(label);
                let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(
                    width as f64,
                    height as f64,
                )));
            }
        }
        if saved.maximized {
            let _ = window.maximize();
        }
        return Ok(());
    }

    if let Some(monitor) = fallback_monitor.as_ref() {
        position_window_on_monitor(&window, label, monitor, None, None);
    }
    Ok(())
}

fn persist_window_layout_snapshot_with_reason(
    app: &AppHandle,
    label: &str,
    _reason: &str,
) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' not found"))?;
    let maximized = window
        .is_maximized()
        .map_err(|err| format!("Read window maximized state failed: {err}"))?;
    let size_and_position = if maximized {
        None
    } else {
        let (width, height) = webview_window_inner_size_logical(&window)?;
        let outer_pos = window
            .outer_position()
            .map_err(|err| format!("Read window outer position failed: {err}"))?;
        Some((width, height, outer_pos.x, outer_pos.y))
    };

    upsert_window_layout(app, label, |entry| {
        if let Some((width, height, x, y)) = size_and_position {
            entry.width = Some(width);
            entry.height = Some(height);
            entry.x = Some(x);
            entry.y = Some(y);
        }
        entry.maximized = maximized;
    })
}

fn attach_window_layout_persistence(app: &AppHandle) {
    for label in ["main", "chat", "archives"] {
        let Some(window) = app.get_webview_window(label) else {
            continue;
        };
        let app_handle = app.clone();
        let label = label.to_string();
        let _ = window.on_window_event(move |event| match event {
            tauri::WindowEvent::Resized(_) => {
                if let Err(err) =
                    persist_window_layout_snapshot_with_reason(&app_handle, &label, "resized")
                {
                    runtime_log_error(format!(
                        "[窗口] 持久化窗口布局失败: label={}, error={}",
                        label.trim(),
                        err
                    ));
                }
            }
            tauri::WindowEvent::Moved(_) => {
                if let Err(err) =
                    persist_window_layout_snapshot_with_reason(&app_handle, &label, "moved")
                {
                    runtime_log_error(format!(
                        "[窗口] 持久化窗口布局失败: label={}, error={}",
                        label.trim(),
                        err
                    ));
                }
            }
            tauri::WindowEvent::CloseRequested { .. } => {
                if let Err(err) = persist_window_layout_snapshot_with_reason(
                    &app_handle,
                    &label,
                    "close_requested",
                ) {
                    runtime_log_error(format!(
                        "[窗口] 持久化窗口布局失败: label={}, error={}",
                        label.trim(),
                        err
                    ));
                }
            }
            tauri::WindowEvent::Destroyed => {
                if let Err(err) =
                    persist_window_layout_snapshot_with_reason(&app_handle, &label, "destroyed")
                {
                    runtime_log_error(format!(
                        "[窗口] 持久化窗口布局失败: label={}, error={}",
                        label.trim(),
                        err
                    ));
                }
            }
            _ => {}
        });
    }
}

fn sync_default_tray_icon(app: &AppHandle) -> Result<(), String> {
    let tray = app
        .tray_by_id(MAIN_TRAY_ID)
        .ok_or_else(|| "Tray icon not found".to_string())?;

    tray
        .set_icon(app.default_window_icon().cloned())
        .map_err(|err| format!("Set tray icon failed: {err}"))
}

fn show_window(app: &AppHandle, label: &str) -> Result<(), String> {
    apply_window_layout_before_show(app, label)?;
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' not found"))?;

    let _ = window.unminimize();
    let _ = window.show();
    ensure_window_visible_after_show(app, label, "show_window");
    let _ = window.set_focus();
    Ok(())
}

fn toggle_window_maximize_with_default_restore(
    app: &AppHandle,
    label: &str,
) -> Result<bool, String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' not found"))?;
    if is_fixed_window_size(label) {
        return Ok(false);
    }
    let was_maximized = window
        .is_maximized()
        .map_err(|err| format!("Read window maximized state failed: {err}"))?;
    if !was_maximized {
        window
            .maximize()
            .map_err(|err| format!("Maximize window failed: {err}"))?;
        let maximized = window
            .is_maximized()
            .map_err(|err| format!("Read window maximized state failed: {err}"))?;
        return Ok(maximized);
    }

    let restore_monitor = preferred_window_monitor(&window);
    let saved_layout_near_fullscreen = restore_monitor
        .as_ref()
        .map(|monitor| saved_window_layout_is_near_fullscreen(app, label, monitor))
        .unwrap_or(false);
    window
        .unmaximize()
        .map_err(|err| format!("Restore window failed: {err}"))?;
    let restored_near_fullscreen = restore_monitor
        .as_ref()
        .map(|monitor| current_window_size_is_near_fullscreen(&window, monitor))
        .unwrap_or(false);
    if saved_layout_near_fullscreen || restored_near_fullscreen {
        if let Some(monitor) = restore_monitor.as_ref() {
            position_window_on_monitor(&window, label, monitor, None, None);
            let _ = persist_window_layout_snapshot_with_reason(
                app,
                label,
                "restore_near_fullscreen_to_default",
            );
        }
    }
    let maximized = window
        .is_maximized()
        .map_err(|err| format!("Read window maximized state failed: {err}"))?;
    Ok(maximized)
}

fn start_window_drag_with_default_restore(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' not found"))?;
    if is_fixed_window_size(label) {
        return window
            .start_dragging()
            .map_err(|err| format!("Start dragging window failed: {err}"));
    }

    let was_maximized = window
        .is_maximized()
        .map_err(|err| format!("Read window maximized state failed: {err}"))?;
    let restore_monitor = preferred_window_monitor(&window);
    let should_restore_default_size = if was_maximized {
        true
    } else {
        restore_monitor
            .as_ref()
            .map(|monitor| current_window_size_is_near_fullscreen(&window, monitor))
            .unwrap_or(false)
    };

    if should_restore_default_size {
        if was_maximized {
            window
                .unmaximize()
                .map_err(|err| format!("Restore window failed: {err}"))?;
        }
        if let Some(monitor) = restore_monitor.as_ref() {
            restore_window_to_default_drag_size(&window, label, monitor)?;
            let _ =
                persist_window_layout_snapshot_with_reason(app, label, "drag_restore_to_default");
        }
    }

    window
        .start_dragging()
        .map_err(|err| format!("Start dragging window failed: {err}"))
}

fn toggle_window(app: &AppHandle, label: &str) -> Result<(), String> {
    let window = app
        .get_webview_window(label)
        .ok_or_else(|| format!("Window '{label}' not found"))?;
    let visible = window
        .is_visible()
        .map_err(|err| format!("Check window visibility failed: {err}"))?;
    let focused = window
        .is_focused()
        .map_err(|err| format!("Check window focus failed: {err}"))?;
    if visible && focused {
        window
            .hide()
            .map_err(|err| format!("Hide window failed: {err}"))?;
        return Ok(());
    }
    show_window(app, label)
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

fn parse_hotkey(raw: &str) -> Result<Shortcut, String> {
    let normalized = normalize_hotkey_for_parser(raw);
    Shortcut::from_str(&normalized)
        .or_else(|_| Shortcut::from_str("Alt+Backquote"))
        .map_err(|err| format!("Parse hotkey failed: {err}"))
}

fn register_default_hotkey(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let config = read_config(&state.config_path).unwrap_or_default();
    register_hotkeys_from_config(app, &config)
}

fn register_hotkey_from_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
    register_hotkeys_from_config(app, config)
}

fn register_hotkeys_from_config(app: &AppHandle, config: &AppConfig) -> Result<(), String> {
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

fn show_chat_entry_window(app: &AppHandle) -> Result<(), String> {
    let target = match state_read_config_cached(app.state::<AppState>().inner()) {
        Ok(mut config) => {
            normalize_app_config(&mut config);
            startup_window_label_for_config(&config)
        }
        Err(err) => {
            runtime_log_error(format!("[托盘] 读取对话入口配置失败: {err}"));
            "quick-setup"
        }
    };
    show_window(app, target)
}

fn run_tray_action(app: &AppHandle, action: &str) -> Result<(), String> {
    match action {
        "config" => show_window(app, "main"),
        "chat" => show_chat_entry_window(app),
        "file-reader" => {
            show_file_reader_window(app)?;
            Ok(())
        }
        "archives" => show_window(app, "archives"),
        "runtime-logs" => show_runtime_logs_window(app),
        other => Err(format!("未知托盘动作：{other}")),
    }
}

fn dispatch_tray_action(app: &AppHandle, source: &'static str, action: &'static str) {
    let app_handle = app.clone();
    let thread_name = format!("tray-action-{action}");
    if let Err(err) = std::thread::Builder::new()
        .name(thread_name)
        .spawn(move || {
            runtime_log_info(format!("[托盘] 收到动作：source={}，action={}", source, action));
            match run_tray_action(&app_handle, action) {
                Ok(()) => {
                    runtime_log_info(format!("[托盘] 动作完成：source={}，action={}", source, action));
                }
                Err(err) => {
                    runtime_log_error(format!(
                        "[托盘] 动作失败：source={}，action={}，error={}",
                        source, action, err
                    ));
                }
            }
        })
    {
        runtime_log_error(format!(
            "[托盘] 调度动作失败：source={}，action={}，error={}",
            source, action, err
        ));
    }
}

// ==================== 运行日志窗口 ====================

const RUNTIME_LOGS_WINDOW_LABEL: &str = "runtime-logs";

fn show_runtime_logs_window(app: &AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(RUNTIME_LOGS_WINDOW_LABEL) {
        append_window_diagnostic_log(
            app,
            format!(
                "[运行日志窗口] 已存在，开始聚焦：window_label={}",
                RUNTIME_LOGS_WINDOW_LABEL
            ),
        );
        if let Err(err) = window.unminimize() {
            append_window_diagnostic_log(
                app,
                format!(
                    "[运行日志窗口] 取消最小化失败：window_label={}，error={}",
                    RUNTIME_LOGS_WINDOW_LABEL, err
                ),
            );
        }
        if let Err(err) = window.show() {
            append_window_diagnostic_log(
                app,
                format!(
                    "[运行日志窗口] 显示失败：window_label={}，error={}",
                    RUNTIME_LOGS_WINDOW_LABEL, err
                ),
            );
        }
        ensure_window_visible_after_show(app, RUNTIME_LOGS_WINDOW_LABEL, "focus_runtime_logs_window");
        if let Err(err) = window.set_focus() {
            append_window_diagnostic_log(
                app,
                format!(
                    "[运行日志窗口] 聚焦失败：window_label={}，error={}",
                    RUNTIME_LOGS_WINDOW_LABEL, err
                ),
            );
        }
        return Ok(());
    }
    let app_handle = app.clone();
    std::thread::Builder::new()
        .name("runtime-logs-window-create".to_string())
        .spawn(move || {
            let started_at = std::time::Instant::now();
            append_window_diagnostic_log(
                &app_handle,
                format!(
                    "[运行日志窗口] 开始创建窗口：window_label={}",
                    RUNTIME_LOGS_WINDOW_LABEL
                ),
            );
            if app_handle.get_webview_window(RUNTIME_LOGS_WINDOW_LABEL).is_some() {
                append_window_diagnostic_log(
                    &app_handle,
                    format!(
                        "[运行日志窗口] 创建前发现窗口已存在，转为聚焦：window_label={}",
                        RUNTIME_LOGS_WINDOW_LABEL
                    ),
                );
                return;
            }
            let window = match tauri::WebviewWindowBuilder::new(
                &app_handle,
                RUNTIME_LOGS_WINDOW_LABEL,
                tauri::WebviewUrl::App("runtime-logs.html".into()),
            )
            .title("PAI - 运行日志")
            .inner_size(900.0, 600.0)
            .min_inner_size(600.0, 400.0)
            .resizable(true)
            .decorations(false)
            .shadow(true)
            .visible(false)
            .build()
            {
                Ok(w) => w,
                Err(err) => {
                    append_window_diagnostic_log(
                        &app_handle,
                        format!(
                            "[运行日志窗口] 创建失败：window_label={}，error={}",
                            RUNTIME_LOGS_WINDOW_LABEL, err
                        ),
                    );
                    return;
                }
            };
            if let Err(err) = apply_window_layout_before_show(&app_handle, RUNTIME_LOGS_WINDOW_LABEL) {
                append_window_diagnostic_log(
                    &app_handle,
                    format!(
                        "[运行日志窗口] 应用窗口布局失败：window_label={}，error={}",
                        RUNTIME_LOGS_WINDOW_LABEL, err
                    ),
                );
            }
            if let Err(err) = window.unminimize() {
                append_window_diagnostic_log(
                    &app_handle,
                    format!(
                        "[运行日志窗口] 取消最小化失败：window_label={}，error={}",
                        RUNTIME_LOGS_WINDOW_LABEL, err
                    ),
                );
            }
            if let Err(err) = window.show() {
                append_window_diagnostic_log(
                    &app_handle,
                    format!(
                        "[运行日志窗口] 显示失败：window_label={}，error={}",
                        RUNTIME_LOGS_WINDOW_LABEL, err
                    ),
                );
            }
            ensure_window_visible_after_show(
                &app_handle,
                RUNTIME_LOGS_WINDOW_LABEL,
                "show_runtime_logs_window",
            );
            if let Err(err) = window.set_focus() {
                append_window_diagnostic_log(
                    &app_handle,
                    format!(
                        "[运行日志窗口] 聚焦失败：window_label={}，error={}",
                        RUNTIME_LOGS_WINDOW_LABEL, err
                    ),
                );
            }
            append_window_diagnostic_log(
                &app_handle,
                format!(
                    "[运行日志窗口] 窗口已显示：window_label={}，elapsed_ms={}",
                    RUNTIME_LOGS_WINDOW_LABEL,
                    started_at.elapsed().as_millis()
                ),
            );
            let cloned = window.clone();
            let _ = window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = cloned.hide();
                }
            });
        })
        .map_err(|err| format!("调度创建运行日志窗口失败：{err}"))?;
    Ok(())
}

fn build_tray(app: &AppHandle) -> Result<(), String> {
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

fn hide_on_close(app: &AppHandle) {
    for label in ["main", "chat", "archives", "quick-setup"] {
        if let Some(window) = app.get_webview_window(label) {
            let cloned = window.clone();
            let _ = window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = cloned.hide();
                }
            });
        }
    }
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
        "quick-setup" => "quick-setup.html",
        _ => "index.html",
    }
}

fn rebuild_crashed_window(app: &AppHandle, label: &str) {
    runtime_log_error(format!("[WebView心跳] 窗口崩溃恢复开始: label={label}"));
    // 尝试关闭旧窗口
    if let Some(window) = app.get_webview_window(label) {
        let _ = window.destroy();
    }
    // 等待旧窗口销毁
    std::thread::sleep(std::time::Duration::from_millis(200));

    let url = webview_window_url_for_label(label);
    let (default_w, default_h) = default_window_size(label);
    let (min_w, min_h) = minimum_window_size(label);
    let resizable = !is_fixed_window_size(label);

    let result = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App(url.into()),
    )
    .title(format!("PAI - {label}"))
    .inner_size(default_w as f64, default_h as f64)
    .min_inner_size(min_w as f64, min_h as f64)
    .resizable(resizable)
    .decorations(false)
    .shadow(true)
    .visible(false)
    .build();

    match result {
        Ok(window) => {
            // 重新注册 hide_on_close
            let cloned = window.clone();
            let _ = window.on_window_event(move |event| {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = cloned.hide();
                }
            });
            // 恢复布局并显示
            let _ = apply_window_layout_before_show(app, label);
            let _ = window.show();
            ensure_window_visible_after_show(app, label, "rebuild_crashed_window");
            let _ = window.set_focus();
            // 重置 pong 时间戳
            webview_record_pong(label);
            runtime_log_error(format!("[WebView心跳] 窗口崩溃恢复完成: label={label}"));
        }
        Err(err) => {
            runtime_log_error(format!("[WebView心跳] 窗口重建失败: label={label}, error={err}"));
        }
    }
}

fn start_webview_heartbeat_monitor(app: &AppHandle) {
    // 初始化所有监控窗口的 pong 时间戳
    for label in WEBVIEW_MONITORED_LABELS {
        webview_record_pong(label);
    }

    let app_handle = app.clone();
    std::thread::Builder::new()
        .name("webview-heartbeat".to_string())
        .spawn(move || {
            loop {
                std::thread::sleep(std::time::Duration::from_millis(WEBVIEW_HEARTBEAT_INTERVAL_MS));

                for label in WEBVIEW_MONITORED_LABELS {
                    // 只监控可见窗口
                    let is_visible = app_handle
                        .get_webview_window(label)
                        .and_then(|w| w.is_visible().ok())
                        .unwrap_or(false);
                    if !is_visible {
                        // 不可见窗口重置时间戳，不检测
                        webview_record_pong(label);
                        continue;
                    }

                    // 发送 ping
                    let _ = app_handle.emit_to(label, "easy-call:webview-ping", ());

                    // 检查上次 pong 时间
                    let missed = {
                        let map = webview_pong_timestamps().lock().ok();
                        map.and_then(|m| m.get(*label).copied())
                            .map(|last| last.elapsed().as_millis() as u64)
                            .unwrap_or(0)
                    };
                    let threshold = WEBVIEW_HEARTBEAT_INTERVAL_MS * (WEBVIEW_HEARTBEAT_MAX_MISS as u64 + 1);
                    if missed > threshold {
                        runtime_log_debug(format!(
                            "[WebView心跳] 检测到窗口无响应: label={}, missed_ms={}, threshold_ms={}",
                            label, missed, threshold
                        ));
                        rebuild_crashed_window(&app_handle, label);
                    }
                }
            }
        })
        .ok();
}
