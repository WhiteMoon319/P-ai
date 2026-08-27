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

const DEVICE_CONTROL_STATUS_EVENT: &str = "easy-call:device-control-status-changed";

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
    /// input tap <x> <y>：点击
    Tap { x: u32, y: u32 },
    /// input swipe <x1> <y1> <x2> <y2> [ms]：滑动
    Swipe { x1: u32, y1: u32, x2: u32, y2: u32, duration_ms: Option<u32> },
    /// input keyevent <keycode>：按键
    KeyEvent { keycode: u32 },
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
            DeviceCommand::Tap { x, y } => format!("input tap {x} {y}"),
            DeviceCommand::Swipe { x1, y1, x2, y2, duration_ms } => match duration_ms {
                Some(ms) => format!("input swipe {x1} {y1} {x2} {y2} {ms}"),
                None => format!("input swipe {x1} {y1} {x2} {y2}"),
            },
            DeviceCommand::KeyEvent { keycode } => format!("input keyevent {keycode}"),
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

/// Android 域提权执行入口（Shizuku 首选 / root 兜底，均走 Kotlin 插件）。
#[cfg(target_os = "android")]
fn device_control_execute_privileged(
    app: &tauri::AppHandle,
    command: &DeviceCommand,
    timeout_ms: u64,
) -> Result<DeviceControlExecResult, String> {
    use tauri_plugin_device_control::DeviceControlExt;

    // 危险操作的 confirm 校验已在各命令入口完成；此处仅执行。
    let rendered = command.render();
    let result = app
        .device_control()
        .execute_command(tauri_plugin_device_control::ExecuteCommandRequest {
            command: rendered,
            timeout_ms,
        })
        .map_err(|err| format!("设备控制执行失败: {err}"))?;
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
    third_party_only: Option<bool>,
) -> Result<String, String> {
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
    package: String,
    confirm: Option<bool>,
) -> Result<(), String> {
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
async fn device_control_unfreeze(app: tauri::AppHandle, package: String) -> Result<(), String> {
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
    package: String,
    confirm: Option<bool>,
) -> Result<(), String> {
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

/// 点击屏幕（x/y 为屏幕像素坐标）。
#[tauri::command]
async fn device_control_tap(app: tauri::AppHandle, x: u32, y: u32) -> Result<(), String> {
    let cmd = DeviceCommand::Tap { x, y };
    let result = device_control_execute_privileged(&app, &cmd, 15_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "点击失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 滑动屏幕（可选时长 ms）。
#[tauri::command]
async fn device_control_swipe(
    app: tauri::AppHandle,
    x1: u32,
    y1: u32,
    x2: u32,
    y2: u32,
    duration_ms: Option<u32>,
) -> Result<(), String> {
    let cmd = DeviceCommand::Swipe {
        x1,
        y1,
        x2,
        y2,
        duration_ms,
    };
    let result = device_control_execute_privileged(&app, &cmd, 15_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "滑动失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
    }
    Ok(())
}

/// 按键（keycode 参考 Android KeyEvent 常量，如 4=返回、3=主页）。
#[tauri::command]
async fn device_control_key_event(
    app: tauri::AppHandle,
    keycode: u32,
) -> Result<(), String> {
    let cmd = DeviceCommand::KeyEvent { keycode };
    let result = device_control_execute_privileged(&app, &cmd, 15_000)?;
    if result.exit_code != 0 {
        return Err(format!(
            "按键失败（exit={}）：{}",
            result.exit_code,
            result.stderr.trim()
        ));
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
    state.llm_workspace_path.clone()
}

/// 路径校验：只允许落在 Android 工作区沙盒内（llm_workspace_path），
/// 拒绝绝对系统路径、`..`、符号链接逃逸。
fn device_control_validate_path(state: &AppState, raw: &str) -> Result<String, String> {
    let root = device_control_root(state)
        .canonicalize()
        .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
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
    fn render_tap_swipe_key_event() {
        assert_eq!(DeviceCommand::Tap { x: 540, y: 1200 }.render(), "input tap 540 1200");
        assert_eq!(
            DeviceCommand::Swipe { x1: 540, y1: 2000, x2: 540, y2: 400, duration_ms: Some(300) }.render(),
            "input swipe 540 2000 540 400 300"
        );
        assert_eq!(
            DeviceCommand::Swipe { x1: 540, y1: 2000, x2: 540, y2: 400, duration_ms: None }.render(),
            "input swipe 540 2000 540 400"
        );
        assert_eq!(DeviceCommand::KeyEvent { keycode: 4 }.render(), "input keyevent 4");
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
}
