// ==================== 设备控制（Shizuku/root 提权）命令 ====================
//
// 执行域口径：Android 上存在两个独立执行域——
//   - Linux 域：现有 terminal（proot rootfs），跑文件/代码/构建命令；
//   - Android 域：本模块（device_control.*），跑 pm/cmd/input/toybox 等系统命令，
//     经 Shizuku 首选 / root 兜底提权。
// 命令白名单：只允许显式枚举的 DeviceCommand，禁止自由字符串 shell。
// 危险操作（冻结/卸载/删除文件/安装）必须带 confirm=true 才执行。

/// Android 系统命令首词白名单：命中即应路由到 Android 域（执行域路由依据）。
/// 前端/agent 侧用 `sys:` 前缀显式覆盖歧义命令到 Android 域。
pub(crate) const ANDROID_SYSTEM_COMMAND_PREFIXES: &[&str] = &[
    "pm", "cmd", "input", "am", "dumpsys", "settings", "service", "getprop",
    "screencap", "toybox", "wm", "netd", "appops", "content",
];

/// 执行域路由：命令首词命中 Android 系统命令白名单，或带 `sys:` 前缀显式
/// 覆盖（歧义命令如 ls/rm/cat/cp 用 `sys:` 强制进 Android 域），返回需在
/// Android 域提权执行的命令全文；否则返回 None 落 Linux 域。
///
/// 与计划四「执行环境域」口径一致：未命中白名单一律落 Linux 域，
/// 不允许静默进入提权环境。
pub(crate) fn route_to_android_domain(command: &str) -> Option<String> {
    let trimmed = command.trim();
    if let Some(rest) = trimmed.strip_prefix("sys:") {
        let rest = rest.trim_start();
        if rest.is_empty() {
            return None;
        }
        return Some(rest.to_string());
    }
    let first_word = trimmed.split_whitespace().next()?;
    if ANDROID_SYSTEM_COMMAND_PREFIXES.contains(&first_word) {
        return Some(trimmed.to_string());
    }
    None
}

/// 路由放行前的元字符防护：白名单首词命令若含 shell 元字符，拒绝进提权
/// 环境（防 `pm disable-user --user 0 x; rm -rf /` 之类拼接绕过白名单意图，
/// 计划 5.4「禁止拼接用户自由输入」），返回拒绝原因；无元字符返回 None。
pub(crate) fn android_domain_route_rejection(command: &str) -> Option<&'static str> {
    const METACHARS: &[char] = &[
        ';', '|', '&', '$', '`', '\n', '>', '<', '(', ')', '{', '}', '\\', '\'', '"',
    ];
    if command.contains(METACHARS) {
        return Some(
            "命令含 shell 元字符，已拒绝路由到 Android 提权环境；请拆分命令或改用 device_control.* 工具。",
        );
    }
    None
}

/// Android 域危险命令识别（终端路由二次确认用）：卸载/冻结/安装/删除文件。
/// 命中后需用户确认才放行进提权环境（计划 5.4 危险操作确认口径）。
pub(crate) fn android_domain_command_is_dangerous(command: &str) -> bool {
    let mut words = command.split_whitespace();
    match words.next() {
        Some("pm") => matches!(
            words.next(),
            Some("uninstall") | Some("disable-user") | Some("install") | Some("enable")
        ),
        Some("rm") => true,
        _ => false,
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceControlStatusResult {
    shizuku_available: bool,
    shizuku_granted: bool,
    root_available: bool,
    privilege_state: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeviceControlExecResult {
    exit_code: i32,
    stdout: String,
    stderr: String,
}

/// 命令白名单（Rust 侧唯一允许执行的 Android 系统命令集）。
#[derive(Debug, Clone)]
enum DeviceCommand {
    /// pm list packages [-3]：列出全部/第三方应用
    ListPackages { third_party_only: bool },
    /// pm disable-user --user 0 <pkg>：冻结应用（危险）
    Freeze { package: String },
    /// pm enable <pkg>：解冻应用
    Unfreeze { package: String },
    /// pm uninstall --user 0 <pkg>：卸载应用（危险，保留数据）
    Uninstall { package: String },
    /// pm install -r <apk>：安装应用（危险）
    Install { apk_path: String },
    /// rm -f <path>：删除白名单目录内文件（危险）
    DeleteFile { path: String },
    /// screencap -p <path>：截屏（供 agent 视觉反馈）
    Screenshot { path: String },
}

impl DeviceCommand {
    fn is_dangerous(&self) -> bool {
        matches!(
            self,
            DeviceCommand::Freeze { .. }
                | DeviceCommand::Uninstall { .. }
                | DeviceCommand::Install { .. }
                | DeviceCommand::DeleteFile { .. }
        )
    }

    /// 渲染为受控命令字符串（参数已在构造时校验）。
    fn render(&self) -> String {
        match self {
            DeviceCommand::ListPackages { third_party_only } => {
                if *third_party_only {
                    "pm list packages -3".to_string()
                } else {
                    "pm list packages".to_string()
                }
            }
            DeviceCommand::Freeze { package } => format!("pm disable-user --user 0 {package}"),
            DeviceCommand::Unfreeze { package } => format!("pm enable {package}"),
            DeviceCommand::Uninstall { package } => format!("pm uninstall --user 0 {package}"),
            DeviceCommand::Install { apk_path } => format!("pm install -r {apk_path}"),
            DeviceCommand::DeleteFile { path } => format!("rm -f {path}"),
            DeviceCommand::Screenshot { path } => format!("screencap -p {path}"),
        }
    }
}

/// 包名校验：仅允许 `[a-zA-Z0-9._]`，拒绝路径分隔、引号与 shell 元字符。
fn validate_package(package: &str) -> Result<String, String> {
    let trimmed = package.trim();
    if trimmed.is_empty() {
        return Err("包名不能为空。".to_string());
    }
    if !trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '_')
    {
        return Err(format!("包名含非法字符: {package}"));
    }
    Ok(trimmed.to_string())
}

/// 危险操作确认：confirm 必须为 true，否则拒绝。
fn require_confirm(confirm: bool, action: &str) -> Result<(), String> {
    if confirm {
        Ok(())
    } else {
        Err(format!("危险操作「{action}」需要 confirm=true 二次确认后才会执行。"))
    }
}

/// 设备控制能力开关校验（计划 5.4：默认全部关闭）。
/// `capability` 为分项能力名（freeze/uninstall/install/delete_file/touch/screenshot）；
/// None 表示仅校验总开关（终端路由路径）。
fn device_control_check_enabled(state: &AppState, capability: Option<&str>) -> Result<(), String> {
    let config = state_read_config_cached(state).map_err(|err| format!("读取配置失败: {err}"))?;
    let dc = config.device_control;
    // Android 调试：经 eprintln! 输出到 logcat（RustStdoutStderr），确认实际读到的开关值
    eprintln!(
        "[device-control] check_enabled cap={:?} enabled={} freeze={} uninstall={} install={} delete={} touch={} screenshot={}",
        capability, dc.enabled, dc.allow_freeze, dc.allow_uninstall, dc.allow_install,
        dc.allow_delete_file, dc.allow_touch, dc.allow_screenshot
    );
    if !dc.enabled {
        return Err("设备控制未开启：请先在「设置 → 工具 → 设备控制」启用总开关。".to_string());
    }
    let allowed = match capability {
        Some("freeze") => dc.allow_freeze,
        Some("uninstall") => dc.allow_uninstall,
        Some("install") => dc.allow_install,
        Some("delete_file") => dc.allow_delete_file,
        Some("touch") => dc.allow_touch,
        Some("screenshot") => dc.allow_screenshot,
        _ => true,
    };
    if !allowed {
        return Err(format!("设备控制能力「{}」未开启：请在设置-工具-设备控制中启用对应开关。", capability.unwrap_or("")));
    }
    Ok(())
}

/// Android 域提权执行入口（Shizuku 首选 / root 兜底，均走 Kotlin 插件）。
#[cfg(target_os = "android")]
fn device_control_execute_privileged(
    app: &tauri::AppHandle,
    command: &DeviceCommand,
    timeout_ms: u64,
) -> Result<DeviceControlExecResult, String> {
    use tauri_plugin_device_control::DeviceControlExt;

    // 审计日志（计划 5.4）：每次提权命令执行落 logs/ 后端日志
    let rendered = command.render();
    runtime_log_info(format!("[设备控制-审计] 提权执行: {rendered}"));
    let result = app
        .device_control()
        .execute_command(tauri_plugin_device_control::ExecuteCommandRequest {
            command: rendered,
            timeout_ms,
        })
        .map_err(|err| {
            runtime_log_warn(format!("[设备控制-审计] 提权执行失败: {err}"));
            format!("设备控制执行失败: {err}")
        })?;
    runtime_log_info(format!(
        "[设备控制-审计] 完成 exit={} {}",
        result.exit_code,
        if result.exit_code == 0 { "OK" } else { "FAIL" }
    ));
    Ok(DeviceControlExecResult {
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

#[cfg(not(target_os = "android"))]
fn device_control_execute_privileged(
    _app: &tauri::AppHandle,
    _command: &DeviceCommand,
    _timeout_ms: u64,
) -> Result<DeviceControlExecResult, String> {
    Err("设备控制仅在 Android 端可用。".to_string())
}

/// Android 域提权 shell 执行（供 terminal 执行域路由调用）。
/// 仅应经 [route_to_android_domain] 放行的白名单首词命令进入，避免静默提权。
#[cfg(target_os = "android")]
pub(crate) async fn device_control_execute_shell_command(
    app: &tauri::AppHandle,
    command: &str,
    timeout_ms: u64,
) -> Result<DeviceControlExecResult, String> {
    use tauri_plugin_device_control::DeviceControlExt;
    // 审计日志（计划 5.4）：终端路由到 Android 域的提权执行同样落日志
    runtime_log_info(format!("[设备控制-审计] 终端路由提权执行: {command}"));
    let result = app
        .device_control()
        .execute_command(tauri_plugin_device_control::ExecuteCommandRequest {
            command: command.to_string(),
            timeout_ms,
        })
        .map_err(|err| {
            runtime_log_warn(format!("[设备控制-审计] 路由提权执行失败: {err}"));
            format!("Android 域命令执行失败: {err}")
        })?;
    runtime_log_info(format!(
        "[设备控制-审计] 路由完成 exit={} {}",
        result.exit_code,
        if result.exit_code == 0 { "OK" } else { "FAIL" }
    ));
    Ok(DeviceControlExecResult {
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
    })
}

#[cfg(not(target_os = "android"))]
pub(crate) async fn device_control_execute_shell_command(
    _app: &tauri::AppHandle,
    _command: &str,
    _timeout_ms: u64,
) -> Result<DeviceControlExecResult, String> {
    Err("设备控制仅在 Android 端可用。".to_string())
}

/// 注入式触控辅助（供内置工具与 command 共用；desktop stub 返回错误）。
#[cfg(target_os = "android")]
pub(crate) fn device_control_inject_touch(
    app: &tauri::AppHandle,
    action: tauri_plugin_device_control::TouchAction,
) -> Result<(), String> {
    use tauri_plugin_device_control::DeviceControlExt;
    app.device_control()
        .inject_touch(action)
        .map_err(|err| format!("触控注入失败: {err}"))
}

#[cfg(not(target_os = "android"))]
pub(crate) fn device_control_inject_touch(
    _app: &tauri::AppHandle,
    _action: tauri_plugin_device_control::TouchAction,
) -> Result<(), String> {
    Err("设备控制仅在 Android 端可用。".to_string())
}

/// 查询提权状态（Shizuku 可用/已授权、root 可用）。
#[tauri::command]
async fn device_control_status(app: tauri::AppHandle) -> Result<DeviceControlStatusResult, String> {
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_device_control::DeviceControlExt;
        let status = app
            .device_control()
            .status()
            .map_err(|err| format!("读取设备提权状态失败: {err}"))?;
        return Ok(DeviceControlStatusResult {
            shizuku_available: status.shizuku_available,
            shizuku_granted: status.shizuku_granted,
            root_available: status.root_available,
            privilege_state: status.privilege_state,
        });
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Ok(DeviceControlStatusResult {
            shizuku_available: false,
            shizuku_granted: false,
            root_available: false,
            privilege_state: "disabled".to_string(),
        })
    }
}

/// 触发 Shizuku 授权弹窗。
#[tauri::command]
async fn device_control_request_privilege(app: tauri::AppHandle) -> Result<(), String> {
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_device_control::DeviceControlExt;
        app.device_control()
            .request_privilege()
            .map_err(|err| format!("请求设备提权失败: {err}"))?;
        return Ok(());
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        Err("设备控制仅在 Android 端可用。".to_string())
    }
}

/// 列出已安装应用（第三方或全部）。
#[tauri::command]
async fn device_control_list_packages(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    third_party_only: Option<bool>,
) -> Result<String, String> {
    device_control_check_enabled(&state, None)?;
    let cmd = DeviceCommand::ListPackages {
        third_party_only: third_party_only.unwrap_or(true),
    };
    let result = device_control_execute_privileged(&app, &cmd, 30_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "列出应用失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(result.stdout.trim().to_string())
}

/// 冻结应用（危险，需 confirm）。
#[tauri::command]
async fn device_control_freeze(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    package: String,
    confirm: Option<bool>,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("freeze"))?;
    require_confirm(confirm.unwrap_or(false), "冻结应用")?;
    let package = validate_package(&package)?;
    let cmd = DeviceCommand::Freeze { package };
    let result = device_control_execute_privileged(&app, &cmd, 30_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "冻结应用失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 解冻应用。
#[tauri::command]
async fn device_control_unfreeze(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    package: String,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("freeze"))?;
    let package = validate_package(&package)?;
    let cmd = DeviceCommand::Unfreeze { package };
    let result = device_control_execute_privileged(&app, &cmd, 30_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "解冻应用失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 卸载应用（危险，需 confirm；保留数据 --user 0）。
#[tauri::command]
async fn device_control_uninstall(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    package: String,
    confirm: Option<bool>,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("uninstall"))?;
    require_confirm(confirm.unwrap_or(false), "卸载应用")?;
    let package = validate_package(&package)?;
    let cmd = DeviceCommand::Uninstall { package };
    let result = device_control_execute_privileged(&app, &cmd, 60_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "卸载应用失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 安装应用（危险，需 confirm）。
#[tauri::command]
async fn device_control_install(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    apk_path: String,
    confirm: Option<bool>,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("install"))?;
    require_confirm(confirm.unwrap_or(false), "安装应用")?;
    let path = device_control_validate_path(&state, &apk_path)?;
    let cmd = DeviceCommand::Install { apk_path: path };
    let result = device_control_execute_privileged(&app, &cmd, 120_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "安装应用失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 删除白名单目录内文件（危险，需 confirm）。
#[tauri::command]
async fn device_control_delete_file(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
    confirm: Option<bool>,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("delete_file"))?;
    require_confirm(confirm.unwrap_or(false), "删除文件")?;
    let path = device_control_validate_path(&state, &path)?;
    let cmd = DeviceCommand::DeleteFile { path };
    let result = device_control_execute_privileged(&app, &cmd, 30_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "删除文件失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 点击屏幕（注入式，x/y 为屏幕像素坐标）。
#[tauri::command]
async fn device_control_tap(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    x: u32,
    y: u32,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("touch"))?;
    runtime_log_info(format!("[设备控制-审计] 注入点击 ({x}, {y})"));
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_device_control::DeviceControlExt;
        app.device_control()
            .inject_touch(tauri_plugin_device_control::TouchAction::Tap { x, y })
            .map_err(|err| format!("点击失败: {err}"))?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (&app, &state, x, y);
        return Err("设备控制仅在 Android 端可用。".to_string());
    }
    Ok(())
}

/// 滑动屏幕（注入式，可选时长 ms）。
#[tauri::command]
async fn device_control_swipe(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    duration_ms: Option<u32>,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("touch"))?;
    runtime_log_info(format!(
        "[设备控制-审计] 注入滑动 ({x1},{y1})->({x2},{y2}) {:?}ms",
        duration_ms
    ));
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_device_control::DeviceControlExt;
        app.device_control()
            .inject_touch(tauri_plugin_device_control::TouchAction::Swipe {
                x1,
                y1,
                x2,
                y2,
                duration_ms,
            })
            .map_err(|err| format!("滑动失败: {err}"))?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (&app, &state, x1, y1, x2, y2, duration_ms);
        return Err("设备控制仅在 Android 端可用。".to_string());
    }
    Ok(())
}

/// 按键（注入式，keycode 参考 Android KeyEvent 常量，如 4=返回、3=主页）。
#[tauri::command]
async fn device_control_key_event(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    keycode: u32,
) -> Result<(), String> {
    device_control_check_enabled(&state, Some("touch"))?;
    runtime_log_info(format!("[设备控制-审计] 注入按键 keycode={keycode}"));
    #[cfg(target_os = "android")]
    {
        use tauri_plugin_device_control::DeviceControlExt;
        app.device_control()
            .inject_touch(tauri_plugin_device_control::TouchAction::Key { keycode })
            .map_err(|err| format!("按键失败: {err}"))?;
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = (&app, &state, keycode);
        return Err("设备控制仅在 Android 端可用。".to_string());
    }
    Ok(())
}

/// 截屏到白名单目录（供 agent 视觉反馈）。
#[tauri::command]
async fn device_control_screenshot(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    file_name: Option<String>,
) -> Result<String, String> {
    device_control_check_enabled(&state, Some("screenshot"))?;
    let root = device_control_root(&state);
    let screenshots_dir = root.join("screenshots");
    std::fs::create_dir_all(&screenshots_dir)
        .map_err(|err| format!("创建截屏目录失败: {err}"))?;
    let name = file_name
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| format!("screenshot_{}.png", chrono::Utc::now().timestamp()));
    let safe_name = name
        .replace(['/', '\\', ' ', ':'], "_")
        .chars()
        .take(80)
        .collect::<String>();
    let target = screenshots_dir.join(safe_name);
    let cmd = DeviceCommand::Screenshot {
        path: target.to_string_lossy().to_string(),
    };
    let result = device_control_execute_privileged(&app, &cmd, 30_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "截屏失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(target.to_string_lossy().to_string())
}

// ---- 路径防护 ----

fn device_control_root(state: &AppState) -> PathBuf {
    // S3 权限域：文件类命令（install/deleteFile/screenshot）由 shell 身份（Shizuku
    // ADB 激活）执行，不能落在应用私有目录（/data/user/0/<pkg>/... 700，shell 无权限）。
    // 改用 `/sdcard/Android/data/<pkg>/files/device_control`——应用（自己）与 shell
    //（adb 可访问全部 sdcard）都可读写，构成双向可用中转区。
    let pkg = state
        .llm_workspace_path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .nth(3)
        .unwrap_or("ai.easycall.app");
    PathBuf::from(format!("/sdcard/Android/data/{pkg}/files/device_control"))
}

/// 路径校验：只允许落在设备控制 sdcard 中转区（device_control_root），
/// 拒绝绝对系统路径、`..`、符号链接逃逸。
fn device_control_validate_path(state: &AppState, raw: &str) -> Result<String, String> {
    let root = device_control_root(state);
    std::fs::create_dir_all(&root)
        .map_err(|err| format!("创建设备控制中转目录失败: {err}"))?;
    let root = root
        .canonicalize()
        .map_err(|err| format!("解析设备控制中转目录失败: {err}"))?;
    device_control_validate_path_inner(&root, raw)
}

/// 路径校验纯逻辑（与 AppState 解耦，便于单测）。
fn device_control_validate_path_inner(root: &std::path::Path, raw: &str) -> Result<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("路径不能为空。".to_string());
    }
    if trimmed.contains("..") {
        return Err("路径不能包含 ..（路径逃逸防护）。".to_string());
    }
    let candidate = std::path::PathBuf::from(trimmed);
    let candidate = if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    };
    let canonical = candidate
        .canonicalize()
        .map_err(|err| format!("路径不存在或不可访问: {err}"))?;
    if !canonical.starts_with(root) {
        return Err(format!("路径不在 Android 工作区沙盒内: {trimmed}"));
    }
    Ok(canonical.to_string_lossy().to_string())
}

#[cfg(test)]
mod device_control_tests {
    use super::*;

    fn tmp_root() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("dc-test-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::create_dir_all(dir.join("screenshots")).unwrap();
        dir
    }

    #[test]
    fn render_list_packages_third_party() {
        let cmd = DeviceCommand::ListPackages { third_party_only: true };
        assert_eq!(cmd.render(), "pm list packages -3");
        assert!(!cmd.is_dangerous());
    }

    #[test]
    fn render_list_packages_all() {
        let cmd = DeviceCommand::ListPackages { third_party_only: false };
        assert_eq!(cmd.render(), "pm list packages");
    }

    #[test]
    fn render_freeze_uses_user_zero_and_package() {
        let cmd = DeviceCommand::Freeze { package: "com.example.app".to_string() };
        assert_eq!(cmd.render(), "pm disable-user --user 0 com.example.app");
        assert!(cmd.is_dangerous());
    }

    #[test]
    fn render_unfreeze_enable() {
        let cmd = DeviceCommand::Unfreeze { package: "com.example.app".to_string() };
        assert_eq!(cmd.render(), "pm enable com.example.app");
        assert!(!cmd.is_dangerous());
    }

    #[test]
    fn render_uninstall_user_zero() {
        let cmd = DeviceCommand::Uninstall { package: "com.example.app".to_string() };
        assert_eq!(cmd.render(), "pm uninstall --user 0 com.example.app");
        assert!(cmd.is_dangerous());
    }

    #[test]
    fn render_install_reinstall() {
        let cmd = DeviceCommand::Install { apk_path: "/data/user/0/ai.easycall.app/x.apk".to_string() };
        assert_eq!(cmd.render(), "pm install -r /data/user/0/ai.easycall.app/x.apk");
        assert!(cmd.is_dangerous());
    }

    #[test]
    fn render_delete_file_rm_force() {
        let cmd = DeviceCommand::DeleteFile { path: "/data/user/0/ai.easycall.app/llm-workspace/tmp/x.bin".to_string() };
        assert_eq!(cmd.render(), "rm -f /data/user/0/ai.easycall.app/llm-workspace/tmp/x.bin");
        assert!(cmd.is_dangerous());
    }

    #[test]
    fn touch_action_serializes_semantic_events() {
        // 注入式触控：TouchAction 序列化为结构化动作 JSON（serde tag="action"），不再是 input 命令文本
        let tap = serde_json::to_value(tauri_plugin_device_control::TouchAction::Tap { x: 540, y: 1200 }).unwrap();
        assert_eq!(tap["action"], "tap");
        assert_eq!(tap["x"], 540);
        assert_eq!(tap["y"], 1200);
        assert!(tap.get("x1").is_none(), "tap 不应携带 swipe 字段");

        let swipe = serde_json::to_value(tauri_plugin_device_control::TouchAction::Swipe {
            x1: 540,
            y1: 2000,
            x2: 540,
            y2: 400,
            duration_ms: Some(300),
        })
        .unwrap();
        assert_eq!(swipe["action"], "swipe");
        assert_eq!(swipe["x1"], 540);
        assert_eq!(swipe["durationMs"], 300);

        let swipe_no_duration = serde_json::to_value(tauri_plugin_device_control::TouchAction::Swipe {
            x1: 540,
            y1: 2000,
            x2: 540,
            y2: 400,
            duration_ms: None,
        })
        .unwrap();
        assert_eq!(swipe_no_duration["action"], "swipe");
        assert_eq!(swipe_no_duration["durationMs"], serde_json::Value::Null);

        let key = serde_json::to_value(tauri_plugin_device_control::TouchAction::Key { keycode: 4 }).unwrap();
        assert_eq!(key["action"], "key");
        assert_eq!(key["keycode"], 4);

        // 反序列化也能还原（Plugin 侧经过 serde tag 枚举透传）
        let back: tauri_plugin_device_control::TouchAction =
            serde_json::from_value(tap).unwrap();
        assert!(matches!(back, tauri_plugin_device_control::TouchAction::Tap { x: 540, y: 1200 }));
    }

    #[test]
    fn render_screenshot() {
        let cmd = DeviceCommand::Screenshot { path: "/data/user/0/ai.easycall.app/llm-workspace/screenshots/a.png".to_string() };
        assert_eq!(cmd.render(), "screencap -p /data/user/0/ai.easycall.app/llm-workspace/screenshots/a.png");
    }

    #[test]
    fn dangerous_commands_require_confirm() {
        assert!(require_confirm(false, "冻结应用").is_err());
        assert!(require_confirm(true, "冻结应用").is_ok());
        assert!(require_confirm(false, "卸载应用").is_err());
        assert!(require_confirm(false, "删除文件").is_err());
        assert!(require_confirm(false, "安装应用").is_err());
    }

    #[test]
    fn validate_package_accepts_valid_names() {
        assert_eq!(validate_package("com.example.app").unwrap(), "com.example.app");
        assert_eq!(validate_package("  com.example.app  ").unwrap(), "com.example.app");
        assert_eq!(validate_package("com.example.app_2").unwrap(), "com.example.app_2");
    }

    #[test]
    fn validate_package_rejects_invalid_chars() {
        assert!(validate_package("").is_err());
        assert!(validate_package("   ").is_err());
        assert!(validate_package("com/example").is_err());
        assert!(validate_package("com;rm -rf").is_err());
        assert!(validate_package("com.example$(id)").is_err());
    }

    #[test]
    fn validate_package_rejects_shell_metacharacters() {
        for bad in ["com.a|b", "com.a&b", "com.a`b", "com.a;b", "com.a\\b"] {
            assert!(validate_package(bad).is_err(), "should reject: {bad}");
        }
    }

    #[test]
    fn path_validation_accepts_sandbox_relative() {
        let root = tmp_root();
        let ok = device_control_validate_path_inner(&root, "sub").unwrap();
        assert!(ok.starts_with(root.to_string_lossy().as_ref()));
        assert!(ok.ends_with("sub"));
    }

    #[test]
    fn path_validation_accepts_sandbox_absolute() {
        let root = tmp_root();
        let target = root.join("sub");
        let ok = device_control_validate_path_inner(&root, &target.to_string_lossy()).unwrap();
        assert!(ok.ends_with("sub"));
    }

    #[test]
    fn path_validation_rejects_traversal() {
        let root = tmp_root();
        assert!(device_control_validate_path_inner(&root, "../../etc/passwd").is_err());
        assert!(device_control_validate_path_inner(&root, "sub/../../etc").is_err());
        assert!(device_control_validate_path_inner(&root, "").is_err());
    }

    #[test]
    fn path_validation_rejects_outside_absolute() {
        let root = tmp_root();
        let outside = std::env::temp_dir().join("outside-dc-test");
        std::fs::create_dir_all(&outside).unwrap();
        assert!(device_control_validate_path_inner(&root, &outside.to_string_lossy()).is_err());
    }

    #[test]
    fn path_validation_rejects_nonexistent() {
        let root = tmp_root();
        assert!(device_control_validate_path_inner(&root, "no-such-dir").is_err());
    }

    #[test]
    fn android_command_prefixes_include_device_tools() {
        for prefix in ["pm", "cmd", "input", "am", "dumpsys", "settings", "service", "getprop", "screencap", "toybox"] {
            assert!(
                ANDROID_SYSTEM_COMMAND_PREFIXES.contains(&prefix),
                "missing prefix: {prefix}"
            );
        }
    }

    #[test]
    fn route_to_android_domain_routes_whitelisted_prefixes() {
        assert_eq!(route_to_android_domain("pm list packages").as_deref(), Some("pm list packages"));
        assert_eq!(route_to_android_domain("dumpsys window").as_deref(), Some("dumpsys window"));
        assert_eq!(route_to_android_domain("toybox").as_deref(), Some("toybox"));
        assert_eq!(route_to_android_domain("  input tap 540 1200").as_deref(), Some("input tap 540 1200"));
    }

    #[test]
    fn route_to_android_domain_sys_prefix_forces_android_domain() {
        assert_eq!(
            route_to_android_domain("sys:rm -f /data/local/tmp/a").as_deref(),
            Some("rm -f /data/local/tmp/a")
        );
        assert_eq!(
            route_to_android_domain("sys: pm clear com.example.app").as_deref(),
            Some("pm clear com.example.app")
        );
    }

    #[test]
    fn route_to_android_domain_leaves_others_to_linux_domain() {
        // 歧义命令默认落 Linux 域，不静默提权
        assert_eq!(route_to_android_domain("ls -la"), None);
        assert_eq!(route_to_android_domain("rm -f x"), None);
        assert_eq!(route_to_android_domain("cat /etc/hosts"), None);
        assert_eq!(route_to_android_domain(""), None);
        assert_eq!(route_to_android_domain("   "), None);
        assert_eq!(route_to_android_domain("sys:"), None);
        assert_eq!(route_to_android_domain("sys:   "), None);
    }

    #[test]
    fn route_rejects_shell_metachars_to_block_injection() {
        // 白名单首词命令拼接 shell 元字符（`;`/`|`/`&&`/`$()`/反引号等）必须被拒绝，
        // 防止 `pm disable-user --user 0 x; rm -rf /` 绕过白名单意图进入提权环境。
        assert_eq!(
            android_domain_route_rejection("pm disable-user --user 0 x; rm -rf /").unwrap_or(""),
            "命令含 shell 元字符，已拒绝路由到 Android 提权环境；请拆分命令或改用 device_control.* 工具。"
        );
        assert!(android_domain_route_rejection("pm list packages").is_none(), "普通 pm 命令不应被拒");
        assert!(android_domain_route_rejection("dumpsys window").is_none());
        assert!(android_domain_route_rejection("sys:rm -f /data/local/tmp/a").is_none(), "sys: 前缀带无元字符命令可放行");
        assert_eq!(android_domain_route_rejection("pm list | grep x").unwrap_or(""), "命令含 shell 元字符，已拒绝路由到 Android 提权环境；请拆分命令或改用 device_control.* 工具。");
        assert_eq!(android_domain_route_rejection("pm disable --user 0 x && rm -rf /").unwrap_or(""), "命令含 shell 元字符，已拒绝路由到 Android 提权环境；请拆分命令或改用 device_control.* 工具。");
        assert_eq!(android_domain_route_rejection("am broadcast -a x $(id)").unwrap_or(""), "命令含 shell 元字符，已拒绝路由到 Android 提权环境；请拆分命令或改用 device_control.* 工具。");
        assert_eq!(android_domain_route_rejection("cmd package uninstall `whoami`").unwrap_or(""), "命令含 shell 元字符，已拒绝路由到 Android 提权环境；请拆分命令或改用 device_control.* 工具。");
    }

    #[test]
    fn dangerous_command_detection_identifies_high_risk_operations() {
        assert!(android_domain_command_is_dangerous("pm uninstall com.example.app"));
        assert!(android_domain_command_is_dangerous("pm disable-user --user 0 com.example.app"));
        assert!(android_domain_command_is_dangerous("pm install -r /sdcard/a.apk"));
        assert!(android_domain_command_is_dangerous("rm -f /sdcard/Android/data/ai.easycall.app/files/device_control/x.bin"));
        assert!(!android_domain_command_is_dangerous("pm list packages"));
        assert!(!android_domain_command_is_dangerous("dumpsys window"));
        assert!(!android_domain_command_is_dangerous("input tap 540 1200"));
        assert!(!android_domain_command_is_dangerous("getprop ro.product.model"));
    }
}
