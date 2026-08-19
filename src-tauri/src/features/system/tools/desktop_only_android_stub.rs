use super::*;
// Android 平台的桌面工具 stub：截图（xcap）与桌面操作（enigo）在移动端不可用，
// 保留与 xcap_screenshot.rs / operate_runner.rs 相同的对外签名。

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct XcapWindowInfo {
    pub(crate) id: u32,
    pub(crate) pid: u32,
    pub(crate) app_name: String,
    pub(crate) title: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) is_focused: bool,
    pub(crate) is_minimized: bool,
    pub(crate) is_maximized: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct XcapMonitorInfo {
    pub(crate) id: u32,
    pub(crate) name: String,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) rotation: f32,
    pub(crate) scale_factor: f32,
    pub(crate) frequency: f32,
    pub(crate) is_primary: bool,
    pub(crate) is_builtin: bool,
}

pub(crate) fn desktop_tool_unsupported_error(tool: &str) -> DesktopToolError {
    DesktopToolError::internal_error(format!("Android 平台不支持桌面工具：{tool}"))
}

pub(crate) async fn run_screenshot_tool(_input: ScreenshotRequest) -> DesktopToolResult<ScreenshotResponse> {
    Err(desktop_tool_unsupported_error("desktop_screenshot"))
}

pub(crate) fn run_capture_window_tool(
    _input: ScreenshotRequest,
    _window_id: Option<u32>,
) -> DesktopToolResult<ScreenshotResponse> {
    Err(desktop_tool_unsupported_error("capture_window"))
}

pub(crate) fn xcap_list_windows_infos() -> DesktopToolResult<Vec<XcapWindowInfo>> {
    Err(desktop_tool_unsupported_error("xcap_list_windows"))
}

pub(crate) fn xcap_list_monitors_infos() -> DesktopToolResult<Vec<XcapMonitorInfo>> {
    Err(desktop_tool_unsupported_error("xcap_list_monitors"))
}

pub(crate) async fn run_operate_tool(
    _input: OperateRequest,
    _screenshots_root: &std::path::Path,
    _include_base64: bool,
) -> DesktopToolResult<OperateResponse> {
    Err(desktop_tool_unsupported_error("operate"))
}

/// Android 无桌面（xcap）截图，operate 截图临时目录恒为空，直接返回未删除。
pub(crate) fn clear_operate_screenshots_temp(
    _data_path: &PathBuf,
    _conversation_id: &str,
) -> Result<(usize, usize), String> {
    Ok((0, 0))
}
