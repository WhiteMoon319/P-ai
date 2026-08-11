pub(crate) const ANDROID_WORKSPACE_STATUS_EVENT: &str = "easy-call:android-workspace-status-changed";
pub(crate) const ANDROID_WORKSPACE_NOT_READY_MESSAGE: &str = "Android 工作区未就绪，请先在设置的工具页初始化 PAI 助理空间。";
pub(crate) const ANDROID_WORKSPACE_ROOTFS_URL: &str = "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.3-base-arm64.tar.gz";
pub(crate) const ANDROID_WORKSPACE_ROOTFS_FILE_NAME: &str = "ubuntu-base-24.04.3-base-arm64.tar.gz";
pub(crate) const ANDROID_WORKSPACE_ROOTFS_SHA256: &str = "7b2dced6dd56ad5e4a813fa25c8de307b655fdabc6ea9213175a92c48dabb048";
pub(crate) const ANDROID_WORKSPACE_ROOTFS_VERSION: &str = "ubuntu-base-24.04.3-arm64";
pub(crate) const ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH: u64 = 29_865_086;
pub(crate) const ANDROID_WORKSPACE_ROOTFS_CONNECT_TIMEOUT_SECS: u64 = 30;
pub(crate) const ANDROID_WORKSPACE_ROOTFS_CHUNK_TIMEOUT_SECS: u64 = 60;
pub(crate) const ANDROID_WORKSPACE_ROOTFS_MARKER_FILE: &str = ".pai-rootfs-installed";
pub(crate) const ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES: u64 = 64 * 1024 * 1024;

// types 与 rootfs_paths 已迁至 crates/pai-android-platform（阶段 5）。
pub(crate) use pai_android_platform::android_workspace::types::*;
pub(crate) use pai_android_platform::android_workspace::rootfs_paths::*;

#[cfg(target_os = "android")]
pub(crate) mod android_workspace_rootfs_installer {
    include!("android_workspace/rootfs_installer.rs");
}
#[cfg(target_os = "android")]
use android_workspace_rootfs_installer::*;

pub(crate) fn android_workspace_root(state: &AppState) -> PathBuf {
    state.llm_workspace_path.clone()
}

pub(crate) fn android_workspace_state_path(state: &AppState) -> PathBuf {
    let root = android_workspace_root(state);
    android_workspace_runtime_base(&root).join(ANDROID_WORKSPACE_STATE_FILE)
}

// android_workspace_status_paths / android_workspace_runtime_root / shell_workspace_display_path
// 已迁至 crates/pai-android-platform（阶段 5），通过顶部 bridge 生效。

pub(crate) fn android_workspace_required_dirs(root: &std::path::Path) -> [PathBuf; 4] {
    [
        root.join("imports"),
        root.join("exports"),
        root.join("tmp"),
        root.join(".pai"),
    ]
}

#[cfg(target_os = "android")]
pub(crate) fn android_workspace_proot_temp_root(state: &AppState) -> PathBuf {
    let root = android_workspace_root(state);
    android_workspace_runtime_base(&root).join("tmp").join("proot")
}

pub(crate) fn android_workspace_runtime_ready(root: &std::path::Path) -> bool {
    let runtime_root = android_workspace_runtime_root(root);
    if !runtime_root.join(ANDROID_WORKSPACE_ROOTFS_MARKER_FILE).is_file() {
        return false;
    }
    // proot 入口是 /bin/sh（Ubuntu 中 /bin -> /usr/bin symlink，实际 execve /usr/bin/sh），
    // 入口不可用时即使 dash 存在也会在 exec 阶段直接失败。
    // is_file() 跟随 symlink，usr/bin/sh -> dash 且 dash 存在时判定通过。
    let dash = runtime_root.join("usr").join("bin").join("dash");
    let usr_sh = runtime_root.join("usr").join("bin").join("sh");
    dash.is_file() && usr_sh.is_file()
}

pub(crate) fn android_workspace_layout_ready(root: &std::path::Path) -> bool {
    root.is_dir()
        && android_workspace_required_dirs(root).iter().all(|path| path.is_dir())
        && android_workspace_runtime_ready(root)
}

pub(crate) fn read_android_workspace_status_file(state: &AppState) -> Option<AndroidWorkspaceStatus> {
    let path = android_workspace_state_path(state);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<AndroidWorkspaceStatus>(&raw).ok()
}

pub(crate) fn write_android_workspace_status_file(
    state: &AppState,
    status: &AndroidWorkspaceStatus,
) -> Result<(), String> {
    let path = android_workspace_state_path(state);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android 工作区状态目录失败 ({}): {err}", parent.display()))?;
    }
    let body = serde_json::to_vec_pretty(status)
        .map_err(|err| format!("序列化 Android 工作区状态失败: {err}"))?;
    fs::write(&path, body)
        .map_err(|err| format!("写入 Android 工作区状态失败 ({}): {err}", path.display()))
}

pub(crate) fn normalize_android_workspace_status(state: &AppState) -> AndroidWorkspaceStatus {
    let root = android_workspace_root(state);
    let (llm_workspace_root, runtime_root) = android_workspace_status_paths(&root);
    let Some(mut status) = read_android_workspace_status_file(state) else {
        if android_workspace_runtime_ready(&root) {
            let status = android_workspace_ready_status(&root);
            let _ = write_android_workspace_status_file(state, &status);
            return status;
        }
        return AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
    };
    let mut changed = false;
    if status.root_path != llm_workspace_root {
        status.root_path = llm_workspace_root.clone();
        changed = true;
    }
    if status.llm_workspace_root != llm_workspace_root {
        status.llm_workspace_root = llm_workspace_root;
        changed = true;
    }
    if status.runtime_root != runtime_root {
        status.runtime_root = runtime_root;
        changed = true;
    }
    if status.version != ANDROID_WORKSPACE_STATUS_VERSION {
        status.version = ANDROID_WORKSPACE_STATUS_VERSION;
        changed = true;
    }
    if status.download_total_bytes != Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH) {
        status.download_total_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
        changed = true;
    }
    if android_workspace_runtime_ready(&root) {
        if status.runtime_version.as_deref() != Some(ANDROID_WORKSPACE_ROOTFS_VERSION) {
            status.runtime_version = Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string());
            changed = true;
        }
        if status.download_bytes != Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH) {
            status.download_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
            changed = true;
        }
        if status.download_stage.is_some() {
            status.download_stage = None;
            changed = true;
        }
        if matches!(status.state, AndroidWorkspaceStateKind::Downloading) {
            status.state = AndroidWorkspaceStateKind::Ready;
            status.initialized_at = status.initialized_at.or_else(|| Some(now_iso()));
            status.updated_at = Some(now_iso());
            changed = true;
        }
    }
    if matches!(status.state, AndroidWorkspaceStateKind::Ready) && !android_workspace_layout_ready(&root) {
        status.state = AndroidWorkspaceStateKind::NotDownloaded;
        status.last_error = Some("Android 工作区目录缺失或未完整初始化。".to_string());
        status.updated_at = Some(now_iso());
        changed = true;
    }
    if changed {
        let _ = write_android_workspace_status_file(state, &status);
    }
    status
}

pub(crate) fn emit_android_workspace_status(app: Option<&NativeAppHandle>, status: &AndroidWorkspaceStatus) {
    if let Some(app) = app {
        let _ = app.emit(ANDROID_WORKSPACE_STATUS_EVENT, status);
    }
}

pub(crate) fn android_workspace_set_status(
    state: &AppState,
    app: Option<&NativeAppHandle>,
    mut status: AndroidWorkspaceStatus,
) -> Result<AndroidWorkspaceStatus, String> {
    let root = android_workspace_root(state);
    let (llm_workspace_root, runtime_root) = android_workspace_status_paths(&root);
    status.root_path = llm_workspace_root.clone();
    status.llm_workspace_root = llm_workspace_root;
    status.runtime_root = runtime_root;
    status.version = ANDROID_WORKSPACE_STATUS_VERSION;
    status.updated_at = Some(now_iso());
    write_android_workspace_status_file(state, &status)?;
    emit_android_workspace_status(app, &status);
    Ok(status)
}

pub(crate) fn ensure_android_workspace_layout(root: &std::path::Path) -> Result<(), String> {
    fs::create_dir_all(root)
        .map_err(|err| format!("创建 Android 工作区根目录失败 ({}): {err}", root.display()))?;
    for dir in android_workspace_required_dirs(root) {
        fs::create_dir_all(&dir)
            .map_err(|err| format!("创建 Android 工作区目录失败 ({}): {err}", dir.display()))?;
    }
    fs::create_dir_all(android_workspace_runtime_root(root))
        .map_err(|err| format!("创建 Android Linux 运行环境目录失败: {err}"))?;
    ensure_workspace_mcp_layout_at_root(root)?;
    ensure_workspace_skills_layout_at_root(root)?;
    ensure_workspace_private_organization_layout_at_root(root)?;
    Ok(())
}

pub(crate) fn android_workspace_ready_status(root: &std::path::Path) -> AndroidWorkspaceStatus {
    let now = now_iso();
    let (llm_workspace_root, runtime_root) = android_workspace_status_paths(root);
    AndroidWorkspaceStatus {
        state: AndroidWorkspaceStateKind::Ready,
        root_path: llm_workspace_root.clone(),
        llm_workspace_root,
        runtime_root,
        initialized_at: Some(now.clone()),
        updated_at: Some(now),
        last_error: None,
        version: ANDROID_WORKSPACE_STATUS_VERSION,
        runtime_version: Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string()),
        download_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_stage: None,
    }
}

pub(crate) fn is_android_workspace_ready(state: &AppState) -> bool {
    #[cfg(target_os = "android")]
    {
        let status = normalize_android_workspace_status(state);
        matches!(status.state, AndroidWorkspaceStateKind::Ready)
            && android_workspace_layout_ready(&android_workspace_root(state))
    }
}

pub(crate) fn android_workspace_gate_error_for_tool(tool_name: &str, is_mcp_tool: bool) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        if android_workspace_tool_requires_linux_runtime(tool_name, is_mcp_tool) {
            return Some(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
    }
    None
}

pub(crate) mod android_workspace_manager {
    include!("android_workspace/manager.rs");
}
use android_workspace_manager::*;

#[cfg(target_os = "android")]
pub(crate) mod android_workspace_file_system {
    include!("android_workspace/file_system.rs");
}
#[cfg(target_os = "android")]
use android_workspace_file_system::*;


/// ws 端调用版。
pub(crate) fn list_android_workspace_files_ws_inner(
    state: &AppState,
    path: Option<String>,
) -> Result<AndroidWorkspaceFileListResult, String> {
    #[cfg(target_os = "android")]
    {
        let raw_path = path.unwrap_or_default();
        let target = android_workspace_resolve_file_manager_existing_path(state, &raw_path, true)?;
        let metadata = fs::metadata(&target)
            .map_err(|err| format!("读取 Android 工作区文件夹失败 ({}): {err}", target.display()))?;
        if !metadata.is_dir() {
            return Err("文件管理路径必须是目录。".to_string());
        }
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let current_path = android_workspace_relative_display(&root, &target);
        let parent_path = if current_path.is_empty() {
            None
        } else {
            target.parent().map(|parent| android_workspace_relative_display(&root, parent))
        };
        let mut entries = Vec::new();
        let dir_entries = fs::read_dir(&target)
            .map_err(|err| format!("读取 Android 工作区文件列表失败 ({}): {err}", target.display()))?;
        for entry in dir_entries.flatten() {
            if let Some(item) = android_workspace_file_entry(&root, entry.path()) {
                entries.push(item);
            }
        }
        entries.sort_by(|left, right| {
            let left_dir = left.kind == "directory";
            let right_dir = right.kind == "directory";
            right_dir
                .cmp(&left_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        entries.truncate(ANDROID_WORKSPACE_MAX_LIST_ENTRIES);
        Ok(AndroidWorkspaceFileListResult {
            current_path,
            parent_path,
            entries,
        })
    }
}


/// ws 端调用版。
pub(crate) fn read_android_workspace_text_ws_inner(
    state: &AppState,
    path: String,
) -> Result<AndroidWorkspaceTextResult, String> {
    #[cfg(target_os = "android")]
    {
        let target = android_workspace_resolve_file_manager_existing_path(state, &path, false)?;
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let metadata = fs::metadata(&target)
            .map_err(|err| format!("读取 Android 工作区文本文件失败 ({}): {err}", target.display()))?;
        if !metadata.is_file() {
            return Err("只能读取文本文件，不能读取目录。".to_string());
        }
        if metadata.len() > ANDROID_WORKSPACE_TEXT_READ_MAX_BYTES {
            return Err(format!(
                "文件过大：{} bytes，当前 Android 工作区文本读取上限为 {} bytes。",
                metadata.len(),
                ANDROID_WORKSPACE_TEXT_READ_MAX_BYTES
            ));
        }
        let text = fs::read_to_string(&target)
            .map_err(|err| format!("读取 Android 工作区文本文件失败 ({}): {err}", target.display()))?;
        Ok(AndroidWorkspaceTextResult {
            path: android_workspace_relative_display(&root, &target),
            bytes: text.len(),
            text,
        })
    }
}


/// ws 端调用版。
pub(crate) fn write_android_workspace_text_ws_inner(
    state: &AppState,
    path: String,
    text: String,
    overwrite: Option<bool>,
) -> Result<AndroidWorkspaceWriteResult, String> {
    #[cfg(target_os = "android")]
    {
        let target = android_workspace_resolve_file_manager_target_path(state, &path, false)?;
        let bytes = text.as_bytes();
        if bytes.len() > ANDROID_WORKSPACE_TEXT_WRITE_MAX_BYTES {
            return Err(format!(
                "写入内容过大：{} bytes，当前 Android 工作区文本写入上限为 {} bytes。",
                bytes.len(),
                ANDROID_WORKSPACE_TEXT_WRITE_MAX_BYTES
            ));
        }
        let allow_overwrite = overwrite.unwrap_or(false);
        if target.exists() {
            if !allow_overwrite {
                return Err(format!("文件已存在：{}", target.display()));
            }
            let metadata = fs::symlink_metadata(&target)
                .map_err(|err| format!("读取 Android 工作区目标文件失败 ({}): {err}", target.display()))?;
            if metadata.file_type().is_symlink() || !metadata.is_file() {
                return Err("写入目标必须是普通文件。".to_string());
            }
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建 Android 工作区写入目录失败 ({}): {err}", parent.display()))?;
        }
        fs::write(&target, text)
            .map_err(|err| format!("写入 Android 工作区文本文件失败 ({}): {err}", target.display()))?;
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let entry = android_workspace_file_entry(&root, target)
            .ok_or_else(|| "写入后无法读取 Android 工作区文件元数据。".to_string())?;
        Ok(AndroidWorkspaceWriteResult { entry })
    }
}


/// ws 端调用版。
pub(crate) fn move_android_workspace_file_ws_inner(
    state: &AppState,
    source: String,
    target: String,
    overwrite: Option<bool>,
) -> Result<AndroidWorkspaceMoveResult, String> {
    #[cfg(target_os = "android")]
    {
        let source_path = android_workspace_resolve_file_manager_existing_path(state, &source, false)?;
        let target_path = android_workspace_resolve_file_manager_target_path(state, &target, false)?;
        let source_metadata = fs::symlink_metadata(&source_path)
            .map_err(|err| format!("读取 Android 工作区移动源失败 ({}): {err}", source_path.display()))?;
        if source_metadata.file_type().is_symlink() {
            return Err("不允许移动符号链接。".to_string());
        }
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let source_display = android_workspace_relative_display(&root, &source_path);
        if target_path.exists() {
            if !overwrite.unwrap_or(false) {
                return Err(format!("目标已存在：{}", target_path.display()));
            }
            let target_metadata = fs::symlink_metadata(&target_path)
                .map_err(|err| format!("读取 Android 工作区移动目标失败 ({}): {err}", target_path.display()))?;
            if target_metadata.file_type().is_symlink() {
                return Err("不允许覆盖符号链接。".to_string());
            }
            if target_metadata.is_dir() {
                fs::remove_dir_all(&target_path)
                    .map_err(|err| format!("删除 Android 工作区移动目标目录失败 ({}): {err}", target_path.display()))?;
            } else {
                fs::remove_file(&target_path)
                    .map_err(|err| format!("删除 Android 工作区移动目标文件失败 ({}): {err}", target_path.display()))?;
            }
        }
        if let Some(parent) = target_path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建 Android 工作区移动目标目录失败 ({}): {err}", parent.display()))?;
        }
        fs::rename(&source_path, &target_path)
            .map_err(|err| format!("移动 Android 工作区文件失败 ({} -> {}): {err}", source_path.display(), target_path.display()))?;
        let entry = android_workspace_file_entry(&root, target_path)
            .ok_or_else(|| "移动后无法读取 Android 工作区文件元数据。".to_string())?;
        Ok(AndroidWorkspaceMoveResult {
            source_path: source_display,
            entry,
        })
    }
}


/// ws 端调用版。
pub(crate) fn glob_android_workspace_files_ws_inner(
    state: &AppState,
    pattern: String,
    path: Option<String>,
) -> Result<AndroidWorkspaceGlobResult, String> {
    #[cfg(target_os = "android")]
    {
        let matcher = android_workspace_glob_to_regex(&pattern)?;
        let start = android_workspace_resolve_file_manager_existing_path(state, path.as_deref().unwrap_or_default(), true)?;
        let metadata = fs::metadata(&start)
            .map_err(|err| format!("读取 Android 工作区 glob 起点失败 ({}): {err}", start.display()))?;
        if !metadata.is_dir() {
            return Err("glob 起点必须是目录。".to_string());
        }
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let mut entries = Vec::new();
        for entry in walkdir::WalkDir::new(&start).follow_links(false).into_iter().filter_map(Result::ok) {
            if entries.len() >= ANDROID_WORKSPACE_MAX_LIST_ENTRIES {
                break;
            }
            let path = entry.path().to_path_buf();
            let Some(file_entry) = android_workspace_file_entry(&root, path) else {
                continue;
            };
            if matcher.is_match(&file_entry.path) {
                entries.push(file_entry);
            }
        }
        Ok(AndroidWorkspaceGlobResult { entries })
    }
}


/// ws 端调用版。
pub(crate) fn grep_android_workspace_files_ws_inner(
    state: &AppState,
    query: String,
    path: Option<String>,
    regex: Option<bool>,
    ignore_case: Option<bool>,
    include_glob: Option<String>,
) -> Result<AndroidWorkspaceGrepResult, String> {
    #[cfg(target_os = "android")]
    {
        if query.trim().is_empty() {
            return Err("grep query 不能为空".to_string());
        }
        let pattern = if regex.unwrap_or(false) { query.clone() } else { ::regex::escape(&query) };
        let matcher = ::regex::RegexBuilder::new(&pattern)
            .case_insensitive(ignore_case.unwrap_or(true))
            .build()
            .map_err(|err| format!("grep pattern 无效：{err}"))?;
        let include_matcher = include_glob
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(android_workspace_glob_to_regex)
            .transpose()?;
        let start = android_workspace_resolve_file_manager_existing_path(state, path.as_deref().unwrap_or_default(), true)?;
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let mut files = Vec::<(PathBuf, String)>::new();
        let metadata = fs::metadata(&start)
            .map_err(|err| format!("读取 Android 工作区 grep 起点失败 ({}): {err}", start.display()))?;
        if metadata.is_file() {
            let Some(file_entry) = android_workspace_file_entry(&root, start.clone()) else {
                return Err("grep 起点不允许访问系统目录。".to_string());
            };
            files.push((start, file_entry.path));
        } else if metadata.is_dir() {
            for entry in walkdir::WalkDir::new(&start).follow_links(false).into_iter().filter_map(Result::ok) {
                if files.len() >= ANDROID_WORKSPACE_MAX_LIST_ENTRIES {
                    break;
                }
                if !entry.file_type().is_file() {
                    continue;
                }
                let path = entry.path().to_path_buf();
                let Some(file_entry) = android_workspace_file_entry(&root, path.clone()) else {
                    continue;
                };
                if file_entry.kind == "file" {
                    files.push((path, file_entry.path));
                }
            }
        } else {
            return Err("grep 起点必须是文件或目录。".to_string());
        }
        let mut matches = Vec::new();
        for (file, relative_path) in files {
            if matches.len() >= ANDROID_WORKSPACE_MAX_SEARCH_RESULTS {
                break;
            }
            let metadata = match fs::symlink_metadata(&file) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > ANDROID_WORKSPACE_TEXT_READ_MAX_BYTES {
                continue;
            }
            if let Some(include_matcher) = &include_matcher {
                if !include_matcher.is_match(&relative_path) {
                    continue;
                }
            }
            let Ok(handle) = fs::File::open(&file) else {
                continue;
            };
            let reader = std::io::BufReader::new(handle);
            for (index, line) in std::io::BufRead::lines(reader).enumerate() {
                if matches.len() >= ANDROID_WORKSPACE_MAX_SEARCH_RESULTS {
                    break;
                }
                let Ok(line) = line else {
                    continue;
                };
                if matcher.is_match(&line) {
                    matches.push(AndroidWorkspaceSearchMatch {
                        path: relative_path.clone(),
                        line: index + 1,
                        text: line,
                    });
                }
            }
        }
        Ok(AndroidWorkspaceGrepResult { matches })
    }
}


/// ws 端调用版：接受 &AppState（dispatch 无法注入 State）。
pub(crate) fn get_android_workspace_status_ws_inner(state: &AppState) -> Result<AndroidWorkspaceStatus, String> {
    #[cfg(target_os = "android")]
    {
        Ok(normalize_android_workspace_status(state))
    }
}




#[cfg(target_os = "android")]
pub(crate) fn android_workspace_remove_path_if_exists(path: &std::path::Path, context: &str) -> Result<(), String> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(format!("读取 Android 沙盒重置目标失败 ({context}, {}): {err}", path.display())),
    };
    if metadata.is_dir() && !metadata.file_type().is_symlink() {
        fs::remove_dir_all(path)
            .map_err(|err| format!("删除 Android 沙盒目录失败 ({context}, {}): {err}", path.display()))
    } else {
        fs::remove_file(path)
            .map_err(|err| format!("删除 Android 沙盒文件失败 ({context}, {}): {err}", path.display()))
    }
}



/// ws 端调用版：重置 Linux 沙盒运行时（保留用户工作区与 Skill 数据）。
pub(crate) fn reset_android_workspace_runtime_ws_inner(
    state: &AppState,
    app: Option<&NativeAppHandle>,
    root: &std::path::Path,
) -> Result<AndroidWorkspaceStatus, String> {
    #[cfg(target_os = "android")]
    {
        for (path, context) in [
            (android_workspace_runtime_root(root), "Linux rootfs"),
            (android_workspace_rootfs_staging_root(root), "rootfs staging"),
            (android_workspace_proot_temp_root(state), "proot 临时目录"),
            (root.join("runtime"), "旧版 llm-workspace 内 runtime"),
            (root.join("tmp").join("proot"), "旧版 proot 临时目录"),
            (root.join("tmp").join(ANDROID_WORKSPACE_ROOTFS_FILE_NAME), "旧版 rootfs 下载缓存"),
            (android_workspace_rootfs_archive_path(root), "rootfs 下载缓存"),
        ] {
            android_workspace_remove_path_if_exists(&path, context)?;
        }
        runtime_log_info("[Android 工作区] 重置 Linux 沙盒完成，用户工作区与 Skill 数据保留".to_string());
        let status = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, root);
        android_workspace_set_status(state, app, status)
    }
}

/// 非 Android 环境（Windows 开发机）返回的占位就绪状态，供 ws 调试链路走通。
pub(crate) fn android_workspace_ws_fake_ready(root: &std::path::Path) -> AndroidWorkspaceStatus {
    let (llm_workspace_root, runtime_root) = android_workspace_status_paths(root);
    AndroidWorkspaceStatus {
        state: AndroidWorkspaceStateKind::Ready,
        root_path: llm_workspace_root.clone(),
        llm_workspace_root,
        runtime_root,
        initialized_at: None,
        updated_at: Some(now_iso()),
        last_error: None,
        version: ANDROID_WORKSPACE_STATUS_VERSION,
        runtime_version: Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string()),
        download_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_stage: None,
    }
}


/// ws 端调用版。
pub(crate) fn import_file_to_android_workspace_ws_inner(
    state: &AppState,
    file_name: String,
    mime: Option<String>,
    data_base64: String,
    target_path: Option<String>,
) -> Result<AndroidWorkspaceImportResult, String> {
    #[cfg(target_os = "android")]
    {
        let _ = mime;
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let mut target = android_workspace_resolve_import_target_path(&root, &file_name, target_path.as_deref())?;
        android_workspace_manager::android_workspace_ensure_paths_within_sandbox(state, &[target.clone()])?;
        android_workspace_ensure_user_file_manager_path(&root, &target, false)?;
        if target.exists() {
            target = android_workspace_unique_sibling_path(&target);
            android_workspace_manager::android_workspace_ensure_paths_within_sandbox(state, &[target.clone()])?;
            android_workspace_ensure_user_file_manager_path(&root, &target, false)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建 Android 工作区导入目录失败 ({}): {err}", parent.display()))?;
        }
        let safe_name = android_workspace_sanitize_file_name(&file_name);
        // decode 前预检：按 base64 长度估算原始字节数，超限直接拒绝，避免先分配完整内存。
        let estimated_bytes = data_base64
            .trim()
            .len()
            .saturating_mul(3)
            .saturating_div(4);
        if estimated_bytes as u64 > ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES {
            return Err(format!(
                "导入文件过大：约 {} bytes，当前 Android 文件管理器单文件上限为 {} bytes。",
                estimated_bytes, ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES
            ));
        }
        let bytes = B64
            .decode(data_base64.trim())
            .map_err(|err| format!("解析导入文件失败: {err}"))?;
        if bytes.len() as u64 > ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES {
            return Err(format!(
                "导入文件过大：{} bytes，当前 Android 文件管理器单文件上限为 {} bytes。",
                bytes.len(),
                ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES
            ));
        }
        fs::write(&target, &bytes)
            .map_err(|err| format!("写入 Android 工作区导入文件失败 ({}): {err}", target.display()))?;
        let status = normalize_android_workspace_status(state);
        Ok(AndroidWorkspaceImportResult {
            status,
            imported_path: android_workspace_relative_display(&root, &target),
            file_name: target
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or(&safe_name)
                .to_string(),
            bytes: bytes.len(),
        })
    }
}

/// 通过 Android `content://` URI 直接把文件流式导入沙盒工作区。
///
/// WebView 只传 URI 字符串，字节流由 Kotlin 侧 ContentResolver 写入沙盒
/// 目标路径（绝对路径，已在 Rust 侧完成沙盒与用户可见性校验）。


/// ws 端调用版。
pub(crate) fn export_file_from_android_workspace_ws_inner(
    state: &AppState,
    path: String,
) -> Result<AndroidWorkspaceExportResult, String> {
    #[cfg(target_os = "android")]
    {
        let target = android_workspace_resolve_file_manager_existing_path(state, &path, false)?;
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let metadata = fs::metadata(&target)
            .map_err(|err| format!("读取 Android 工作区导出文件失败 ({}): {err}", target.display()))?;
        if !metadata.is_file() {
            return Err("只能导出文件，不能导出目录。".to_string());
        }
        if metadata.len() > ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES {
            return Err(format!(
                "导出文件过大：{} bytes，当前 Android 文件管理器单文件上限为 {} bytes。",
                metadata.len(),
                ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES
            ));
        }
        let bytes = fs::read(&target)
            .map_err(|err| format!("读取 Android 工作区导出文件失败 ({}): {err}", target.display()))?;
        let file_name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("workspace-export")
            .to_string();
        Ok(AndroidWorkspaceExportResult {
            path: android_workspace_relative_display(&root, &target),
            file_name,
            mime: android_workspace_mime_from_path(&target),
            data_base64: B64.encode(&bytes),
            bytes: bytes.len(),
        })
    }
}

/// 通过 Android 系统分享面板导出沙盒工作区文件。
///
/// WebView 的 `navigator.share` 在 wry Android 中不可用，改由 Kotlin 原生
/// ACTION_SEND + FileProvider 唤起系统分享。只传文件绝对路径，绕开 base64，
/// 因此也消除了旧导出链路的 64MB 上限。


/// ws 端调用版。
pub(crate) fn delete_file_from_android_workspace_ws_inner(
    state: &AppState,
    path: String,
) -> Result<AndroidWorkspaceDeleteResult, String> {
    #[cfg(target_os = "android")]
    {
        let target = android_workspace_resolve_file_manager_existing_path(state, &path, false)?;
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let metadata = fs::symlink_metadata(&target)
            .map_err(|err| format!("读取 Android 工作区文件失败 ({}): {err}", target.display()))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("只能删除文件，不能删除目录或链接。".to_string());
        }
        let deleted_path = android_workspace_relative_display(&root, &target);
        fs::remove_file(&target)
            .map_err(|err| format!("删除 Android 工作区文件失败 ({}): {err}", target.display()))?;
        Ok(AndroidWorkspaceDeleteResult { deleted_path })
    }
}

pub(crate) async fn init_android_workspace_ws_inner(
    state: &AppState,
    app: Option<&NativeAppHandle>,
) -> Result<AndroidWorkspaceStatus, String> {

        let root = android_workspace_root(&state);
        let mut downloading = normalize_android_workspace_status(state);
        downloading.state = AndroidWorkspaceStateKind::Downloading;
        downloading.last_error = None;
        downloading.runtime_version = None;
        downloading.download_bytes = Some(0);
        downloading.download_total_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
        downloading.download_stage = Some("preparing".to_string());
        android_workspace_set_status(state, app, downloading)?;
        match ensure_android_workspace_layout(&root) {
            Ok(()) => {
                if let Err(err) = ensure_android_workspace_rootfs(state, app.unwrap_or(&NativeAppHandle::noop()), &root).await {
                    let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                    failed.last_error = Some(err.clone());
                    let _ = android_workspace_set_status(state, app, failed);
                    return Err(err);
                }
                android_workspace_set_status(state, app, android_workspace_ready_status(&root))
            }
            Err(err) => {
                let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                failed.last_error = Some(err.clone());
                let _ = android_workspace_set_status(state, app, failed);
                Err(err)
            }
        }
}

pub(crate) async fn import_android_workspace_rootfs_archive_ws_inner(
    state: &AppState,
    app: Option<&NativeAppHandle>,
    file_name: String,
    data_base64: String,
) -> Result<AndroidWorkspaceStatus, String> {

        let root = android_workspace_root(&state);
        let safe_name = android_workspace_sanitize_file_name(&file_name);
        if !safe_name.ends_with(".tar.gz") && safe_name != ANDROID_WORKSPACE_ROOTFS_FILE_NAME {
            runtime_log_warn(format!(
                "[Android 工作区] 导入 Linux 运行环境压缩包文件名不匹配，继续按内容校验，file_name={}",
                safe_name
            ));
        }
        let mut importing = normalize_android_workspace_status(state);
        importing.state = AndroidWorkspaceStateKind::Downloading;
        importing.last_error = None;
        importing.runtime_version = None;
        importing.download_bytes = Some(0);
        importing.download_total_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
        importing.download_stage = Some("importing".to_string());
        android_workspace_set_status(state, app, importing)?;
        match ensure_android_workspace_layout(&root) {
            Ok(()) => {
                // decode 前预检：rootfs 压缩包约 29.8MB，允许导入上限与普通文件一致
                // （64MiB），超限直接拒绝，避免先分配完整内存再报错。
                let estimated_bytes = data_base64
                    .trim()
                    .len()
                    .saturating_mul(3)
                    .saturating_div(4);
                if estimated_bytes as u64 > ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES {
                    let error = format!(
                        "导入 Linux 运行环境压缩包过大：约 {} bytes，上限为 {} bytes。",
                        estimated_bytes, ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES
                    );
                    let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                    failed.last_error = Some(error.clone());
                    let _ = android_workspace_set_status(state, app, failed);
                    return Err(error);
                }
                let bytes = match B64.decode(data_base64.trim()) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        let error = format!("解析 Android Linux 运行环境压缩包失败: {err}");
                        let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                        failed.last_error = Some(error.clone());
                        let _ = android_workspace_set_status(state, app, failed);
                        return Err(error);
                    }
                };
                android_workspace_update_download_progress(state, app.unwrap_or(&NativeAppHandle::noop()), &root, bytes.len() as u64, "importing")?;
                if let Err(err) = import_android_workspace_rootfs_from_archive(state, app.unwrap_or(&NativeAppHandle::noop()), &root, bytes).await {
                    let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                    failed.last_error = Some(err.clone());
                    let _ = android_workspace_set_status(state, app, failed);
                    return Err(err);
                }
                android_workspace_set_status(state, app, android_workspace_ready_status(&root))
            }
            Err(err) => {
                let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                failed.last_error = Some(err.clone());
                let _ = android_workspace_set_status(state, app, failed);
                Err(err)
            }
        }
}

pub(crate) fn reset_android_workspace_state_ws_inner(
    state: &AppState,
    app: Option<&NativeAppHandle>,
) -> Result<AndroidWorkspaceStatus, String> {

        let status = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &android_workspace_root(&state));
        android_workspace_set_status(state, app, status)
}

pub(crate) fn repair_android_workspace_runtime_ws_inner(
    state: &AppState,
    app: Option<&NativeAppHandle>,
) -> Result<AndroidWorkspaceStatus, String> {

        let root = android_workspace_root(&state);
        let runtime_root = android_workspace_runtime_root(&root);
        let result = (|| {
            ensure_android_workspace_layout(&root)?;
            if !runtime_root.join("usr").join("bin").join("dash").is_file() {
                return Err("Android Linux 运行环境缺少 usr/bin/dash，请重置沙盒后重新初始化。".to_string());
            }
            let temp_dir = android_workspace_proot_temp_root(&state);
            let temp_dir = if let Ok(canonical) = temp_dir.canonicalize() { canonical } else { temp_dir };
            fs::create_dir_all(&temp_dir)
                .map_err(|err| format!("创建 Android proot 临时目录失败 ({}): {err}", temp_dir.display()))?;
            let (native_dir, _, _) = features_system_sandbox::android_rootfs_runner::android_proot_binary_paths()?;
            let _ = features_system_sandbox::android_rootfs_runner::android_proot_ensure_libs_dir(&native_dir, &temp_dir)?;
            features_system_sandbox::android_rootfs_patcher::android_proot_ensure_host_pai_layout(&root)?;
            features_system_sandbox::android_rootfs_patcher::android_proot_patch_rootfs(&runtime_root)?;
            write_android_workspace_rootfs_marker(&runtime_root)?;
            if !android_workspace_runtime_ready(&root) {
                return Err("Android Linux 运行环境修复后仍未通过就绪检查。".to_string());
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                runtime_log_info("[Android 工作区] 修复 Linux 沙盒完成".to_string());
                android_workspace_set_status(state, app, android_workspace_ready_status(&root))
            }
            Err(err) => {
                runtime_log_error(format!("[Android 工作区] 修复 Linux 沙盒失败 err={err:?}"));
                let mut status = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                status.last_error = Some(err.clone());
                let _ = android_workspace_set_status(state, app, status);
                Err(err)
            }
        }
}
