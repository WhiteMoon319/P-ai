// Android 平台的桌面工具 stub：截图（xcap）与桌面操作（enigo）在移动端不可用，
// 保留与 xcap_screenshot.rs / operate_runner.rs 相同的对外签名。

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct XcapWindowInfo {
    id: u32,
    pid: u32,
    app_name: String,
    title: String,
    x: i32,
    y: i32,
    z: i32,
    width: u32,
    height: u32,
    is_focused: bool,
    is_minimized: bool,
    is_maximized: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct XcapMonitorInfo {
    id: u32,
    name: String,
    x: i32,
    y: i32,
    width: u32,
    height: u32,
    rotation: f32,
    scale_factor: f32,
    frequency: f32,
    is_primary: bool,
    is_builtin: bool,
}

/// 控件树元素类型（仅用于类型签名对齐，Android 上桌面操作始终返回错误）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiElementInfo {
    pub window_id: u32,
    pub window_title: String,
    pub control_type: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub focused: bool,
}

fn desktop_tool_unsupported_error(tool: &str) -> DesktopToolError {
    DesktopToolError::internal_error(format!("Android 平台不支持桌面工具：{tool}"))
}

async fn run_screenshot_tool(_input: ScreenshotRequest) -> DesktopToolResult<ScreenshotResponse> {
    Err(desktop_tool_unsupported_error("desktop_screenshot"))
}

fn run_capture_window_tool(
    _input: ScreenshotRequest,
    _window_id: Option<u32>,
) -> DesktopToolResult<ScreenshotResponse> {
    Err(desktop_tool_unsupported_error("capture_window"))
}

fn xcap_list_windows_infos() -> DesktopToolResult<Vec<XcapWindowInfo>> {
    Err(desktop_tool_unsupported_error("xcap_list_windows"))
}

fn xcap_list_monitors_infos() -> DesktopToolResult<Vec<XcapMonitorInfo>> {
    Err(desktop_tool_unsupported_error("xcap_list_monitors"))
}

async fn run_operate_tool(
    _input: OperateRequest,
    _screenshots_root: &std::path::Path,
    _include_base64: bool,
) -> DesktopToolResult<OperateResponse> {
    Err(desktop_tool_unsupported_error("operate"))
}

/// Android 无桌面（xcap）截图，operate 截图临时目录恒为空，直接返回未删除。
fn clear_operate_screenshots_temp(
    _data_path: &PathBuf,
    _conversation_id: &str,
) -> Result<(usize, usize), String> {
    Ok((0, 0))
}

// ==================== windows 工具 stub（Android 始终返回错误） ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsRequest {
    script: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsResponse {
    ok: bool,
    executed_count: usize,
    steps: Vec<WindowsStepResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WindowsStepResult {
    kind: WindowsStepKind,
    summary: String,
    ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum WindowsStepKind {
    ListWindows,
    ActivateWindow,
}

fn run_windows_tool(_input: WindowsRequest) -> DesktopToolResult<WindowsResponse> {
    Err(desktop_tool_unsupported_error("windows"))
}
