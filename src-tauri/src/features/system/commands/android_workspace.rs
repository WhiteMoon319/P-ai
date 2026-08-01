const ANDROID_WORKSPACE_STATUS_EVENT: &str = "easy-call:android-workspace-status-changed";
const ANDROID_WORKSPACE_STATE_FILE: &str = "android_workspace_state.json";
const ANDROID_WORKSPACE_NOT_READY_MESSAGE: &str = "Android 工作区未就绪，请先在设置的工具页初始化 PAI 助理空间。";
const ANDROID_WORKSPACE_ROOTFS_URL: &str = "https://cdimage.ubuntu.com/ubuntu-base/releases/24.04/release/ubuntu-base-24.04.3-base-arm64.tar.gz";
const ANDROID_WORKSPACE_ROOTFS_FILE_NAME: &str = "ubuntu-base-24.04.3-base-arm64.tar.gz";
const ANDROID_WORKSPACE_ROOTFS_SHA256: &str = "7b2dced6dd56ad5e4a813fa25c8de307b655fdabc6ea9213175a92c48dabb048";
const ANDROID_WORKSPACE_ROOTFS_VERSION: &str = "ubuntu-base-24.04.3-arm64";
const ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH: u64 = 29_865_086;
const ANDROID_WORKSPACE_ROOTFS_CONNECT_TIMEOUT_SECS: u64 = 30;
const ANDROID_WORKSPACE_ROOTFS_CHUNK_TIMEOUT_SECS: u64 = 60;
const ANDROID_WORKSPACE_ROOTFS_MARKER_FILE: &str = ".pai-rootfs-installed";
const ANDROID_WORKSPACE_FILE_TRANSFER_MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum AndroidWorkspaceStateKind {
    NotDownloaded,
    Downloading,
    Ready,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AndroidWorkspaceStatus {
    state: AndroidWorkspaceStateKind,
    root_path: String,
    initialized_at: Option<String>,
    updated_at: Option<String>,
    last_error: Option<String>,
    version: u32,
    #[serde(default)]
    runtime_version: Option<String>,
    #[serde(default)]
    download_bytes: Option<u64>,
    #[serde(default)]
    download_total_bytes: Option<u64>,
    #[serde(default)]
    download_stage: Option<String>,
}

impl AndroidWorkspaceStatus {
    fn new(state: AndroidWorkspaceStateKind, root: &std::path::Path) -> Self {
        Self {
            state,
            root_path: shell_workspace_display_path(root),
            initialized_at: None,
            updated_at: Some(now_iso()),
            last_error: None,
            version: 1,
            runtime_version: None,
            download_bytes: None,
            download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_stage: None,
        }
    }
}

fn android_workspace_root(state: &AppState) -> PathBuf {
    state.llm_workspace_path.clone()
}

fn android_workspace_state_path(state: &AppState) -> PathBuf {
    state
        .config_path
        .parent()
        .map(|path| path.join(ANDROID_WORKSPACE_STATE_FILE))
        .unwrap_or_else(|| state.data_path.with_file_name(ANDROID_WORKSPACE_STATE_FILE))
}

fn android_workspace_required_dirs(root: &std::path::Path) -> [PathBuf; 4] {
    [
        root.join("imports"),
        root.join("exports"),
        root.join("tmp"),
        root.join(".pai"),
    ]
}

fn android_workspace_runtime_root(root: &std::path::Path) -> PathBuf {
    root.join("runtime").join("linux")
}

fn android_workspace_runtime_ready(root: &std::path::Path) -> bool {
    let runtime_root = android_workspace_runtime_root(root);
    runtime_root.join(ANDROID_WORKSPACE_ROOTFS_MARKER_FILE).is_file()
        && runtime_root.join("usr").join("bin").join("dash").is_file()
}

fn android_workspace_layout_ready(root: &std::path::Path) -> bool {
    root.is_dir()
        && android_workspace_required_dirs(root).iter().all(|path| path.is_dir())
        && android_workspace_runtime_ready(root)
}

fn read_android_workspace_status_file(state: &AppState) -> Option<AndroidWorkspaceStatus> {
    let path = android_workspace_state_path(state);
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<AndroidWorkspaceStatus>(&raw).ok()
}

fn write_android_workspace_status_file(
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

fn normalize_android_workspace_status(state: &AppState) -> AndroidWorkspaceStatus {
    let root = android_workspace_root(state);
    let Some(mut status) = read_android_workspace_status_file(state) else {
        return AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
    };
    status.root_path = shell_workspace_display_path(&root);
    status.version = if status.version == 0 { 1 } else { status.version };
    status.download_total_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
    if android_workspace_runtime_ready(&root) {
        status.runtime_version = Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string());
        status.download_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
        status.download_stage = None;
        if matches!(status.state, AndroidWorkspaceStateKind::Downloading) {
            status.state = AndroidWorkspaceStateKind::Ready;
            status.initialized_at = status.initialized_at.or_else(|| Some(now_iso()));
            status.updated_at = Some(now_iso());
            let _ = write_android_workspace_status_file(state, &status);
        }
    }
    if matches!(status.state, AndroidWorkspaceStateKind::Ready) && !android_workspace_layout_ready(&root) {
        status.state = AndroidWorkspaceStateKind::NotDownloaded;
        status.last_error = Some("Android 工作区目录缺失或未完整初始化。".to_string());
        status.updated_at = Some(now_iso());
        let _ = write_android_workspace_status_file(state, &status);
    }
    status
}

fn emit_android_workspace_status(app: Option<&AppHandle>, status: &AndroidWorkspaceStatus) {
    if let Some(app) = app {
        let _ = app.emit(ANDROID_WORKSPACE_STATUS_EVENT, status);
    }
}

fn android_workspace_set_status(
    state: &AppState,
    app: Option<&AppHandle>,
    mut status: AndroidWorkspaceStatus,
) -> Result<AndroidWorkspaceStatus, String> {
    status.updated_at = Some(now_iso());
    write_android_workspace_status_file(state, &status)?;
    emit_android_workspace_status(app, &status);
    Ok(status)
}

fn ensure_android_workspace_layout(root: &std::path::Path) -> Result<(), String> {
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

fn android_workspace_rootfs_archive_path(root: &std::path::Path) -> PathBuf {
    root.join("tmp").join(ANDROID_WORKSPACE_ROOTFS_FILE_NAME)
}

fn android_workspace_rootfs_staging_root(root: &std::path::Path) -> PathBuf {
    root.join("tmp").join("rootfs-staging")
}

#[cfg(target_os = "android")]
fn android_workspace_apply_static_webpki_roots(
    builder: reqwest::ClientBuilder,
) -> Result<reqwest::ClientBuilder, String> {
    let mut roots = Vec::with_capacity(webpki_root_certs::TLS_SERVER_ROOT_CERTS.len());
    for der in webpki_root_certs::TLS_SERVER_ROOT_CERTS.iter() {
        roots.push(
            reqwest::tls::Certificate::from_der(der.as_ref())
                .map_err(|err| format!("加载 Android 静态 TLS 根证书失败: {err}"))?,
        );
    }
    Ok(builder.tls_certs_only(roots))
}

fn android_workspace_update_download_progress(
    state: &AppState,
    app: &AppHandle,
    root: &std::path::Path,
    bytes: u64,
    stage: &str,
) -> Result<(), String> {
    let status = AndroidWorkspaceStatus {
        state: AndroidWorkspaceStateKind::Downloading,
        root_path: shell_workspace_display_path(root),
        initialized_at: None,
        updated_at: Some(now_iso()),
        last_error: None,
        version: 1,
        runtime_version: None,
        download_bytes: Some(bytes),
        download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_stage: Some(stage.to_string()),
    };
    android_workspace_set_status(state, Some(app), status).map(|_| ())
}

async fn download_android_workspace_rootfs(
    state: &AppState,
    app: &AppHandle,
    root: &std::path::Path,
) -> Result<PathBuf, String> {
    let archive_path = android_workspace_rootfs_archive_path(root);
    if archive_path.exists() {
        fs::remove_file(&archive_path)
            .map_err(|err| format!("清理 Android Linux 下载缓存失败 ({}): {err}", archive_path.display()))?;
    }
    android_workspace_update_download_progress(state, app, root, 0, "connecting")?;
    eprintln!("[Android 工作区] 开始下载 Linux 运行环境 url={} expected_bytes={}", ANDROID_WORKSPACE_ROOTFS_URL, ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
    let mut client_builder = reqwest::Client::builder()
        .user_agent(app_http_user_agent())
        .timeout(std::time::Duration::from_secs(600))
        .connect_timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10));
    #[cfg(target_os = "android")]
    {
        client_builder = android_workspace_apply_static_webpki_roots(client_builder)?;
    }
    // reqwest 的 connect_timeout 不覆盖 Android 端部分 DNS/握手等待，额外包一层避免阶段卡死。
    let response = tokio::time::timeout(
        std::time::Duration::from_secs(ANDROID_WORKSPACE_ROOTFS_CONNECT_TIMEOUT_SECS),
        client_builder
            .build()
            .map_err(|err| format!("创建 Android Linux 下载客户端失败: {err}"))?
            .get(ANDROID_WORKSPACE_ROOTFS_URL)
            .send(),
    )
    .await
    .map_err(|_| {
        runtime_log_warn(format!(
            "[Android 工作区] 下载阶段超时，任务=连接下载源，url={}，timeout_secs={}",
            ANDROID_WORKSPACE_ROOTFS_URL,
            ANDROID_WORKSPACE_ROOTFS_CONNECT_TIMEOUT_SECS
        ));
        format!(
            "连接下载源超时（{} 秒）：{}",
            ANDROID_WORKSPACE_ROOTFS_CONNECT_TIMEOUT_SECS,
            ANDROID_WORKSPACE_ROOTFS_URL
        )
    })?
    .map_err(|err| format!("下载 Android Linux 运行环境失败: {err}"))?;

    if !response.status().is_success() {
        return Err(format!("下载 Android Linux 运行环境失败，HTTP 状态：{}", response.status()));
    }
    android_workspace_update_download_progress(state, app, root, 0, "downloading")?;
    eprintln!("[Android 工作区] Linux 运行环境响应已建立 status={} expected_bytes={}", response.status(), ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android Linux 下载目录失败 ({}): {err}", parent.display()))?;
    }
    let mut file = tokio::fs::File::create(&archive_path)
        .await
        .map_err(|err| format!("创建 Android Linux 下载文件失败 ({}): {err}", archive_path.display()))?;
    let mut stream = response.bytes_stream();
    let mut downloaded = 0u64;
    let mut next_emit = 0u64;
    while let Some(chunk) = tokio::time::timeout(
        std::time::Duration::from_secs(ANDROID_WORKSPACE_ROOTFS_CHUNK_TIMEOUT_SECS),
        stream.next(),
    )
    .await
    .map_err(|_| {
        runtime_log_warn(format!(
            "[Android 工作区] 下载阶段超时，任务=读取下载数据，url={}，timeout_secs={}",
            ANDROID_WORKSPACE_ROOTFS_URL,
            ANDROID_WORKSPACE_ROOTFS_CHUNK_TIMEOUT_SECS
        ));
        format!(
            "读取下载数据超时（{} 秒）：{}",
            ANDROID_WORKSPACE_ROOTFS_CHUNK_TIMEOUT_SECS,
            ANDROID_WORKSPACE_ROOTFS_URL
        )
    })? {

        let chunk = chunk.map_err(|err| format!("读取 Android Linux 下载数据失败: {err}"))?;
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk)
            .await
            .map_err(|err| format!("写入 Android Linux 下载文件失败: {err}"))?;
        downloaded = downloaded.saturating_add(chunk.len() as u64);
        if downloaded >= next_emit {
            android_workspace_update_download_progress(state, app, root, downloaded, "downloading")?;
            next_emit = downloaded.saturating_add(512 * 1024);
        }
    }
    tokio::io::AsyncWriteExt::flush(&mut file)
        .await
        .map_err(|err| format!("刷新 Android Linux 下载文件失败: {err}"))?;
    android_workspace_update_download_progress(state, app, root, downloaded, "downloaded")?;
    if downloaded != ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH {
        return Err(format!(
            "Android Linux 运行环境大小不匹配：expected={} actual={}",
            ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH, downloaded
        ));
    }
    Ok(archive_path)
}

fn verify_android_workspace_rootfs_archive(path: &std::path::Path) -> Result<(), String> {
    let bytes = fs::read(path)
        .map_err(|err| format!("读取 Android Linux 运行环境包失败 ({}): {err}", path.display()))?;
    let actual = {
        let mut hasher = <sha2::Sha256 as sha2::Digest>::new();
        sha2::Digest::update(&mut hasher, &bytes);
        bytes_to_lower_hex(sha2::Digest::finalize(hasher))
    };
    if actual != ANDROID_WORKSPACE_ROOTFS_SHA256 {
        return Err(format!(
            "Android Linux 运行环境校验失败：expected={} actual={}",
            ANDROID_WORKSPACE_ROOTFS_SHA256, actual
        ));
    }
    Ok(())
}

fn android_workspace_rootfs_resolve_entry_path(
    root: &std::path::Path,
    path: &std::path::Path,
) -> Result<PathBuf, String> {
    let mut target = root.to_path_buf();
    let mut has_component = false;
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => {
                has_component = true;
                target.push(part);
            }
            std::path::Component::CurDir => {}
            std::path::Component::Prefix(_) | std::path::Component::RootDir | std::path::Component::ParentDir => {
                return Err(format!("Android Linux 运行环境包包含非法路径：{}", path.display()));
            }
        }
    }
    if !has_component {
        return Err("Android Linux 运行环境包包含空路径".to_string());
    }
    Ok(target)
}

fn android_workspace_rootfs_normalize_path(path: &std::path::Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn android_workspace_rootfs_resolve_symlink_target(
    root: &std::path::Path,
    link_path: &std::path::Path,
    link_target: &std::path::Path,
) -> Option<PathBuf> {
    let root = android_workspace_rootfs_normalize_path(root);
    let link_path = android_workspace_rootfs_normalize_path(link_path);
    let resolved = if link_target.is_absolute() {
        let mut target = root.clone();
        for component in link_target.components() {
            match component {
                std::path::Component::Normal(part) => target.push(part),
                std::path::Component::CurDir | std::path::Component::RootDir => {}
                std::path::Component::ParentDir => {
                    target.pop();
                }
                std::path::Component::Prefix(_) => return None,
            }
        }
        android_workspace_rootfs_normalize_path(&target)
    } else {
        let parent = link_path.parent()?;
        android_workspace_rootfs_normalize_path(&parent.join(link_target))
    };
    resolved.starts_with(&root).then_some(resolved)
}

fn android_workspace_rootfs_relative_symlink_target(
    root: &std::path::Path,
    link_path: &std::path::Path,
    link_target: &std::path::Path,
) -> Option<String> {
    let root = android_workspace_rootfs_normalize_path(root);
    let link_path = android_workspace_rootfs_normalize_path(link_path);
    let parent = link_path.parent()?;
    let resolved = android_workspace_rootfs_resolve_symlink_target(&root, &link_path, link_target)?;
    let from = parent.strip_prefix(&root).ok()?;
    let to = resolved.strip_prefix(&root).ok()?;
    let from_parts = from
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let to_parts = to
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut common = 0usize;
    while common < from_parts.len() && common < to_parts.len() && from_parts[common] == to_parts[common] {
        common += 1;
    }
    let mut relative = Vec::<String>::new();
    for _ in common..from_parts.len() {
        relative.push("..".to_string());
    }
    relative.extend(to_parts.into_iter().skip(common));
    Some(if relative.is_empty() { ".".to_string() } else { relative.join("/") })
}

fn android_workspace_unpack_rootfs_symlink<R: std::io::Read>(
    entry: &mut tar::Entry<'_, R>,
    runtime_root: &std::path::Path,
    relative_path: &std::path::Path,
) -> Result<(), String> {
    let target_path = android_workspace_rootfs_resolve_entry_path(runtime_root, relative_path)?;
    let link_name = entry
        .link_name()
        .map_err(|err| format!("读取 Android Linux 运行环境符号链接目标失败 ({}): {err}", relative_path.display()))?
        .ok_or_else(|| format!("Android Linux 运行环境符号链接缺少目标：{}", relative_path.display()))?
        .into_owned();
    let host_link_target = android_workspace_rootfs_relative_symlink_target(runtime_root, &target_path, &link_name)
        .ok_or_else(|| format!("Android Linux 运行环境符号链接目标非法：{} -> {}", relative_path.display(), link_name.display()))?;
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android Linux 运行环境符号链接目录失败 ({}): {err}", parent.display()))?;
    }
    if let Ok(metadata) = fs::symlink_metadata(&target_path) {
        if metadata.is_dir() && !metadata.file_type().is_symlink() {
            fs::remove_dir_all(&target_path)
                .map_err(|err| format!("清理 Android Linux 运行环境符号链接目录失败 ({}): {err}", relative_path.display()))?;
        } else {
            fs::remove_file(&target_path)
                .map_err(|err| format!("清理 Android Linux 运行环境符号链接目标失败 ({}): {err}", relative_path.display()))?;
        }
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&host_link_target, &target_path)
            .map_err(|err| format!("创建 Android Linux 运行环境符号链接失败 ({} -> {}): {err}", relative_path.display(), host_link_target))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        let _ = host_link_target;
        entry
            .unpack_in(runtime_root)
            .map_err(|err| format!("解压 Android Linux 运行环境符号链接失败 ({}): {err}", relative_path.display()))
    }
}

fn android_workspace_unpack_rootfs_hard_link<R: std::io::Read>(
    entry: &mut tar::Entry<'_, R>,
    runtime_root: &std::path::Path,
    relative_path: &std::path::Path,
) -> Result<(), String> {
    let target_path = android_workspace_rootfs_resolve_entry_path(runtime_root, relative_path)?;
    let link_name = entry
        .link_name()
        .map_err(|err| format!("读取 Android Linux 运行环境硬链接目标失败 ({}): {err}", relative_path.display()))?
        .ok_or_else(|| format!("Android Linux 运行环境硬链接缺少目标：{}", relative_path.display()))?
        .into_owned();
    let source_path = android_workspace_rootfs_resolve_entry_path(runtime_root, &link_name)?;
    let source_metadata = fs::metadata(&source_path)
        .map_err(|err| format!("读取 Android Linux 运行环境硬链接源失败 ({} -> {}): {err}", relative_path.display(), link_name.display()))?;
    if !source_metadata.is_file() {
        return Err(format!(
            "Android Linux 运行环境硬链接源不是文件：{} -> {}",
            relative_path.display(),
            link_name.display()
        ));
    }
    if let Some(parent) = target_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android Linux 运行环境硬链接目录失败 ({}): {err}", parent.display()))?;
    }
    if target_path.exists() {
        fs::remove_file(&target_path)
            .map_err(|err| format!("清理 Android Linux 运行环境硬链接目标失败 ({}): {err}", relative_path.display()))?;
    }
    if let Err(err) = fs::hard_link(&source_path, &target_path) {
        runtime_log_warn(format!(
            "[Android 工作区] Linux 运行环境硬链接创建失败，改为复制文件，path={} source={} error={}",
            relative_path.display(),
            link_name.display(),
            err
        ));
        fs::copy(&source_path, &target_path)
            .map_err(|copy_err| format!("复制 Android Linux 运行环境硬链接源失败 ({} -> {}): {copy_err}", relative_path.display(), link_name.display()))?;
        fs::set_permissions(&target_path, source_metadata.permissions())
            .map_err(|perm_err| format!("设置 Android Linux 运行环境硬链接副本权限失败 ({}): {perm_err}", relative_path.display()))?;
    }
    Ok(())
}

fn extract_android_workspace_rootfs_archive(
    archive_path: &std::path::Path,
    runtime_root: &std::path::Path,
) -> Result<(), String> {
    fs::create_dir_all(runtime_root)
        .map_err(|err| format!("创建 Android Linux 运行环境目录失败 ({}): {err}", runtime_root.display()))?;
    let file = fs::File::open(archive_path)
        .map_err(|err| format!("打开 Android Linux 运行环境包失败 ({}): {err}", archive_path.display()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let entries = archive
        .entries()
        .map_err(|err| format!("读取 Android Linux 运行环境包失败: {err}"))?;
    for entry in entries {
        let mut entry = entry.map_err(|err| format!("读取 Android Linux 运行环境条目失败: {err}"))?;
        let relative_path = entry
            .path()
            .map_err(|err| format!("读取 Android Linux 运行环境条目路径失败: {err}"))?
            .into_owned();
        let entry_type = entry.header().entry_type();
        if entry_type.is_character_special() || entry_type.is_block_special() || entry_type.is_fifo() {
            continue;
        }
        if entry_type.is_hard_link() {
            android_workspace_unpack_rootfs_hard_link(&mut entry, runtime_root, &relative_path)?;
            continue;
        }
        if entry_type.is_symlink() {
            android_workspace_unpack_rootfs_symlink(&mut entry, runtime_root, &relative_path)?;
            continue;
        }
        android_workspace_rootfs_resolve_entry_path(runtime_root, &relative_path)?;
        entry
            .unpack_in(runtime_root)
            .map_err(|err| format!("解压 Android Linux 运行环境条目失败 ({}): {err}", relative_path.display()))?;
    }
    if !runtime_root.join("usr").join("bin").join("dash").is_file() {
        return Err("Android Linux 运行环境缺少 usr/bin/dash".to_string());
    }
    let marker = serde_json::json!({
        "version": ANDROID_WORKSPACE_ROOTFS_VERSION,
        "source": ANDROID_WORKSPACE_ROOTFS_URL,
        "sha256": ANDROID_WORKSPACE_ROOTFS_SHA256,
        "installedAt": now_iso(),
    });
    let marker_body = serde_json::to_vec_pretty(&marker)
        .map_err(|err| format!("序列化 Android Linux 运行环境标记失败: {err}"))?;
    fs::write(runtime_root.join(ANDROID_WORKSPACE_ROOTFS_MARKER_FILE), marker_body)
        .map_err(|err| format!("写入 Android Linux 运行环境标记失败: {err}"))?;
    Ok(())
}

fn install_android_workspace_rootfs_archive(
    root: &std::path::Path,
    archive_path: &std::path::Path,
) -> Result<(), String> {
    let runtime_root = android_workspace_runtime_root(root);
    let staging_root = android_workspace_rootfs_staging_root(root);
    if staging_root.exists() {
        fs::remove_dir_all(&staging_root)
            .map_err(|err| format!("清理 Android Linux 运行环境临时目录失败 ({}): {err}", staging_root.display()))?;
    }
    if let Some(parent) = staging_root.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android Linux 运行环境临时目录失败 ({}): {err}", parent.display()))?;
    }
    if let Err(err) = extract_android_workspace_rootfs_archive(archive_path, &staging_root) {
        let _ = fs::remove_dir_all(&staging_root);
        return Err(err);
    }
    if runtime_root.exists() {
        fs::remove_dir_all(&runtime_root)
            .map_err(|err| format!("清理旧 Android Linux 运行环境失败 ({}): {err}", runtime_root.display()))?;
    }
    if let Some(parent) = runtime_root.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android Linux 运行环境父目录失败 ({}): {err}", parent.display()))?;
    }
    fs::rename(&staging_root, &runtime_root)
        .map_err(|err| format!("安装 Android Linux 运行环境失败 ({} -> {}): {err}", staging_root.display(), runtime_root.display()))?;
    Ok(())
}

async fn ensure_android_workspace_rootfs(
    state: &AppState,
    app: &AppHandle,
    root: &std::path::Path,
) -> Result<(), String> {
    if android_workspace_runtime_ready(root) {
        return Ok(());
    }
    let archive_path = download_android_workspace_rootfs(state, app, root).await?;
    android_workspace_update_download_progress(state, app, root, ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH, "verifying")?;
    verify_android_workspace_rootfs_archive(&archive_path)?;
    android_workspace_update_download_progress(state, app, root, ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH, "extracting")?;
    let root_for_install = root.to_path_buf();
    let archive_for_extract = archive_path.clone();
    tokio::task::spawn_blocking(move || install_android_workspace_rootfs_archive(&root_for_install, &archive_for_extract))
        .await
        .map_err(|err| format!("解压 Android Linux 运行环境任务失败: {err}"))??;
    let _ = fs::remove_file(&archive_path);
    Ok(())
}

async fn import_android_workspace_rootfs_from_archive(
    state: &AppState,
    app: &AppHandle,
    root: &std::path::Path,
    archive_bytes: Vec<u8>,
) -> Result<(), String> {
    let archive_path = android_workspace_rootfs_archive_path(root);
    if archive_path.exists() {
        fs::remove_file(&archive_path)
            .map_err(|err| format!("清理 Android Linux 导入缓存失败 ({}): {err}", archive_path.display()))?;
    }
    let bytes_len = archive_bytes.len() as u64;
    if bytes_len != ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH {
        return Err(format!(
            "Android Linux 运行环境压缩包大小不匹配：expected={} actual={}",
            ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH, bytes_len
        ));
    }
    if let Some(parent) = archive_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("创建 Android Linux 导入目录失败 ({}): {err}", parent.display()))?;
    }
    tokio::fs::write(&archive_path, archive_bytes)
        .await
        .map_err(|err| format!("写入 Android Linux 导入压缩包失败 ({}): {err}", archive_path.display()))?;
    android_workspace_update_download_progress(state, app, root, ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH, "verifying")?;
    verify_android_workspace_rootfs_archive(&archive_path)?;
    android_workspace_update_download_progress(state, app, root, ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH, "extracting")?;
    let root_for_install = root.to_path_buf();
    let archive_for_extract = archive_path.clone();
    tokio::task::spawn_blocking(move || install_android_workspace_rootfs_archive(&root_for_install, &archive_for_extract))
        .await
        .map_err(|err| format!("解压 Android Linux 运行环境任务失败: {err}"))??;
    let _ = fs::remove_file(&archive_path);
    Ok(())
}

fn android_workspace_ready_status(root: &std::path::Path) -> AndroidWorkspaceStatus {
    let now = now_iso();
    AndroidWorkspaceStatus {
        state: AndroidWorkspaceStateKind::Ready,
        root_path: shell_workspace_display_path(root),
        initialized_at: Some(now.clone()),
        updated_at: Some(now),
        last_error: None,
        version: 1,
        runtime_version: Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string()),
        download_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_stage: None,
    }
}

fn is_android_workspace_ready(state: &AppState) -> bool {
    #[cfg(target_os = "android")]
    {
        let status = normalize_android_workspace_status(state);
        matches!(status.state, AndroidWorkspaceStateKind::Ready)
            && android_workspace_layout_ready(&android_workspace_root(state))
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        true
    }
}

fn android_workspace_gate_error_for_tool(tool_name: &str, is_mcp_tool: bool) -> Option<String> {
    #[cfg(target_os = "android")]
    {
        let normalized = tool_name.trim();
        if is_mcp_tool
            || matches!(
                normalized,
                "exec" | "read" | "read_file" | "read_media" | "write" | "delete" | "update" | "move" | "patch" | "config" | "reload" | "operate"
            )
        {
            return Some(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = tool_name;
        let _ = is_mcp_tool;
    }
    None
}

fn android_sandbox_path_is_within(root: &std::path::Path, target: &std::path::Path) -> bool {
    path_is_within(root, target)
}

fn android_workspace_normalize_path_lexically(path: &std::path::Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _ = normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

fn android_workspace_existing_ancestor(path: &std::path::Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

fn android_workspace_canonical_root_if_ready(state: &AppState) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "android")]
    {
        if !is_android_workspace_ready(state) {
            return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        return Ok(Some(root));
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        Ok(None)
    }
}

fn android_workspace_resolve_existing_file_path(
    state: &AppState,
    raw_path: &str,
) -> Result<Option<PathBuf>, String> {
    let Some(root) = android_workspace_canonical_root_if_ready(state)? else {
        return Ok(None);
    };
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("path 不能为空".to_string());
    }
    let raw = PathBuf::from(normalize_terminal_path_input_for_current_platform(trimmed));
    let joined = if raw.is_absolute() { raw } else { root.join(raw) };
    let canonical = joined
        .canonicalize()
        .map_err(|_| format!("文件不存在：{}", joined.display()))?;
    if !android_sandbox_path_is_within(&root, &canonical) {
        return Err(format!("Android 工作区不允许访问沙盒外路径：{}", joined.display()));
    }
    Ok(Some(canonical))
}

fn android_workspace_ensure_paths_within_sandbox(state: &AppState, paths: &[PathBuf]) -> Result<(), String> {
    let Some(root) = android_workspace_canonical_root_if_ready(state)? else {
        return Ok(());
    };
    for path in paths {
        let joined = if path.is_absolute() { path.clone() } else { root.join(path) };
        let normalized = terminal_normalize_for_access_check(&joined);
        let lexical = android_workspace_normalize_path_lexically(&normalized);
        if !android_sandbox_path_is_within(&root, &lexical) {
            return Err(format!("Android 工作区不允许访问沙盒外路径：{}", path.display()));
        }
        let Some(existing) = android_workspace_existing_ancestor(&normalized) else {
            return Err(format!("Android 工作区无法解析目标路径：{}", path.display()));
        };
        let existing_canonical = existing
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区目标路径失败 ({}): {err}", existing.display()))?;
        if !android_sandbox_path_is_within(&root, &existing_canonical) {
            return Err(format!("Android 工作区不允许访问沙盒外路径：{}", path.display()));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidWorkspaceImportResult {
    #[serde(flatten)]
    status: AndroidWorkspaceStatus,
    imported_path: String,
    file_name: String,
    bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidWorkspaceExportResult {
    path: String,
    file_name: String,
    mime: String,
    data_base64: String,
    bytes: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidWorkspaceFileEntry {
    name: String,
    path: String,
    kind: String,
    bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidWorkspaceFileListResult {
    current_path: String,
    parent_path: Option<String>,
    entries: Vec<AndroidWorkspaceFileEntry>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AndroidWorkspaceDeleteResult {
    deleted_path: String,
}

fn android_workspace_sanitize_file_name(raw: &str) -> String {
    let name = std::path::Path::new(raw.trim())
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("imported-file")
        .trim();
    let mut out = String::with_capacity(name.len().max(1));
    for ch in name.chars() {
        if ch == '/' || ch == '\\' || ch == ':' || ch == '*' || ch == '?' || ch == '"' || ch == '<' || ch == '>' || ch == '|' || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim_matches(&['.', ' ', '_'][..]).to_string();
    if trimmed.is_empty() {
        "imported-file".to_string()
    } else {
        trimmed
    }
}

fn android_workspace_unique_sibling_path(candidate: &std::path::Path) -> PathBuf {
    if !candidate.exists() {
        return candidate.to_path_buf();
    }
    let parent = candidate.parent().unwrap_or_else(|| std::path::Path::new(""));
    let file_name = candidate
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("imported-file");
    let path = std::path::Path::new(file_name);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("imported-file");
    let ext = path.extension().and_then(|value| value.to_str()).unwrap_or("");
    for index in 1..1000 {
        let next_name = if ext.is_empty() {
            format!("{stem} ({index})")
        } else {
            format!("{stem} ({index}).{ext}")
        };
        let next = parent.join(next_name);
        if !next.exists() {
            return next;
        }
    }
    parent.join(format!("{}-{}", Uuid::new_v4(), file_name))
}

fn android_workspace_unique_import_path(imports_dir: &std::path::Path, file_name: &str) -> PathBuf {
    android_workspace_unique_sibling_path(&imports_dir.join(file_name))
}

fn android_workspace_resolve_import_target_path(
    root: &std::path::Path,
    file_name: &str,
    target_path: Option<&str>,
) -> Result<PathBuf, String> {
    let safe_name = android_workspace_sanitize_file_name(file_name);
    let trimmed = target_path.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        let imports_dir = root.join("imports");
        fs::create_dir_all(&imports_dir)
            .map_err(|err| format!("创建 Android 工作区导入目录失败 ({}): {err}", imports_dir.display()))?;
        return Ok(android_workspace_unique_import_path(&imports_dir, &safe_name));
    }
    let normalized = normalize_terminal_path_input_for_current_platform(trimmed);
    let raw = PathBuf::from(normalized);
    if raw.is_absolute() {
        return Err("导入路径必须是 Android 沙盒内的相对路径。".to_string());
    }
    let target = root.join(raw);
    if target.file_name().is_none() {
        return Err("导入路径必须包含文件名。".to_string());
    }
    Ok(target)
}

fn android_workspace_relative_display(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn android_workspace_clean_relative_input(raw: &str) -> Result<PathBuf, String> {
    let normalized = normalize_terminal_path_input_for_current_platform(raw.trim());
    if normalized.trim().is_empty() {
        return Ok(PathBuf::new());
    }
    let raw_path = PathBuf::from(normalized);
    if raw_path.is_absolute() {
        return Err("文件管理只允许访问 Android 沙盒内的相对路径。".to_string());
    }
    let mut out = PathBuf::new();
    for component in raw_path.components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err("文件管理路径不能包含上级目录跳转。".to_string());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("文件管理只允许访问 Android 沙盒内的相对路径。".to_string());
            }
        }
    }
    Ok(out)
}

fn android_workspace_root_name_is_reserved_for_file_manager(name: &str) -> bool {
    matches!(
        name,
        "runtime"
            | "tmp"
            | "mcp"
            | "skills"
            | "private-organization"
            | "avatars"
            | "media"
            | "app_config.toml"
            | "app_data.json"
            | ANDROID_WORKSPACE_STATE_FILE
    )
}

fn android_workspace_relative_path_is_user_visible(relative: &std::path::Path, allow_root: bool) -> bool {
    let mut index = 0usize;
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return false;
        };
        let name = part.to_string_lossy();
        if name.is_empty() || name.starts_with('.') {
            return false;
        }
        if index == 0 && android_workspace_root_name_is_reserved_for_file_manager(name.as_ref()) {
            return false;
        }
        index = index.saturating_add(1);
    }
    index > 0 || allow_root
}

fn android_workspace_ensure_user_file_manager_path(
    root: &std::path::Path,
    target: &std::path::Path,
    allow_root: bool,
) -> Result<(), String> {
    let normalized = terminal_normalize_for_access_check(target);
    let lexical = android_workspace_normalize_path_lexically(&normalized);
    if !android_sandbox_path_is_within(root, &lexical) {
        return Err(format!("文件管理不允许访问 Android 沙盒外路径：{}", target.display()));
    }
    let relative = lexical
        .strip_prefix(root)
        .map_err(|_| format!("文件管理无法解析沙盒相对路径：{}", target.display()))?;
    if !android_workspace_relative_path_is_user_visible(relative, allow_root) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    let Some(existing) = android_workspace_existing_ancestor(&normalized) else {
        return Err(format!("文件管理无法解析目标路径：{}", target.display()));
    };
    let existing_canonical = existing
        .canonicalize()
        .map_err(|err| format!("解析 Android 工作区目标路径失败 ({}): {err}", existing.display()))?;
    if !android_sandbox_path_is_within(root, &existing_canonical) {
        return Err(format!("文件管理不允许访问 Android 沙盒外路径：{}", target.display()));
    }
    let existing_relative = existing_canonical
        .strip_prefix(root)
        .map_err(|_| format!("文件管理无法解析沙盒相对路径：{}", existing.display()))?;
    if !android_workspace_relative_path_is_user_visible(existing_relative, true) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    Ok(())
}

fn android_workspace_resolve_file_manager_existing_path(
    state: &AppState,
    raw_path: &str,
    allow_root: bool,
) -> Result<PathBuf, String> {
    let Some(root) = android_workspace_canonical_root_if_ready(state)? else {
        return Err("Android 工作区文件管理仅在 Android 端可用。".to_string());
    };
    let relative = android_workspace_clean_relative_input(raw_path)?;
    if !android_workspace_relative_path_is_user_visible(&relative, allow_root) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    let target = root.join(&relative);
    android_workspace_ensure_user_file_manager_path(&root, &target, allow_root)?;
    let canonical = target
        .canonicalize()
        .map_err(|_| format!("文件不存在：{}", target.display()))?;
    android_workspace_ensure_user_file_manager_path(&root, &canonical, allow_root)?;
    Ok(canonical)
}

fn android_workspace_mime_from_path(path: &std::path::Path) -> String {
    fs::read(path)
        .ok()
        .and_then(|bytes| infer::get(&bytes).map(|kind| kind.mime_type().to_string()))
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

fn android_workspace_file_entry(root: &std::path::Path, path: PathBuf) -> Option<AndroidWorkspaceFileEntry> {
    let metadata = fs::symlink_metadata(&path).ok()?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || (!file_type.is_file() && !file_type.is_dir()) {
        return None;
    }
    android_workspace_ensure_user_file_manager_path(root, &path, false).ok()?;
    let name = path.file_name()?.to_str()?.to_string();
    let relative_path = android_workspace_relative_display(root, &path);
    let kind = if file_type.is_dir() { "directory" } else { "file" }.to_string();
    let bytes = if file_type.is_file() { Some(metadata.len()) } else { None };
    Some(AndroidWorkspaceFileEntry {
        name,
        path: relative_path,
        kind,
        bytes,
    })
}

#[tauri::command]
fn list_android_workspace_files(
    state: State<'_, AppState>,
    path: Option<String>,
) -> Result<AndroidWorkspaceFileListResult, String> {
    #[cfg(target_os = "android")]
    {
        let status = normalize_android_workspace_status(&state);
        if !matches!(status.state, AndroidWorkspaceStateKind::Ready) {
            return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
        let raw_path = path.unwrap_or_default();
        let target = android_workspace_resolve_file_manager_existing_path(&state, &raw_path, true)?;
        let metadata = fs::metadata(&target)
            .map_err(|err| format!("读取 Android 工作区文件夹失败 ({}): {err}", target.display()))?;
        if !metadata.is_dir() {
            return Err("文件管理路径必须是目录。".to_string());
        }
        let root = android_workspace_root(&state)
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
        Ok(AndroidWorkspaceFileListResult {
            current_path,
            parent_path,
            entries,
        })
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        let _ = path;
        Err("Android 工作区文件管理仅在 Android 端可用。".to_string())
    }
}

#[tauri::command]
fn get_android_workspace_status(state: State<'_, AppState>) -> Result<AndroidWorkspaceStatus, String> {
    #[cfg(target_os = "android")]
    {
        Ok(normalize_android_workspace_status(&state))
    }
    #[cfg(not(target_os = "android"))]
    {
        let root = android_workspace_root(&state);
        Ok(AndroidWorkspaceStatus {
            state: AndroidWorkspaceStateKind::Ready,
            root_path: shell_workspace_display_path(&root),
            initialized_at: None,
            updated_at: Some(now_iso()),
            last_error: None,
            version: 1,
            runtime_version: Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string()),
            download_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_stage: None,
        })
    }
}

#[tauri::command]
async fn init_android_workspace(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AndroidWorkspaceStatus, String> {
    #[cfg(target_os = "android")]
    {
        let root = android_workspace_root(&state);
        let mut downloading = normalize_android_workspace_status(&state);
        downloading.state = AndroidWorkspaceStateKind::Downloading;
        downloading.last_error = None;
        downloading.runtime_version = None;
        downloading.download_bytes = Some(0);
        downloading.download_total_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
        downloading.download_stage = Some("preparing".to_string());
        android_workspace_set_status(&state, Some(&app), downloading)?;
        match ensure_android_workspace_layout(&root) {
            Ok(()) => {
                if let Err(err) = ensure_android_workspace_rootfs(&state, &app, &root).await {
                    let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                    failed.last_error = Some(err.clone());
                    let _ = android_workspace_set_status(&state, Some(&app), failed);
                    return Err(err);
                }
                android_workspace_set_status(&state, Some(&app), android_workspace_ready_status(&root))
            }
            Err(err) => {
                let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                failed.last_error = Some(err.clone());
                let _ = android_workspace_set_status(&state, Some(&app), failed);
                Err(err)
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        let root = android_workspace_root(&state);
        Ok(AndroidWorkspaceStatus {
            state: AndroidWorkspaceStateKind::Ready,
            root_path: shell_workspace_display_path(&root),
            initialized_at: None,
            updated_at: Some(now_iso()),
            last_error: None,
            version: 1,
            runtime_version: Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string()),
            download_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_stage: None,
        })
    }
}

#[tauri::command]
async fn import_android_workspace_rootfs_archive(
    app: AppHandle,
    state: State<'_, AppState>,
    file_name: String,
    data_base64: String,
) -> Result<AndroidWorkspaceStatus, String> {
    #[cfg(target_os = "android")]
    {
        let root = android_workspace_root(&state);
        let safe_name = android_workspace_sanitize_file_name(&file_name);
        if !safe_name.ends_with(".tar.gz") && safe_name != ANDROID_WORKSPACE_ROOTFS_FILE_NAME {
            runtime_log_warn(format!(
                "[Android 工作区] 导入 Linux 运行环境压缩包文件名不匹配，继续按内容校验，file_name={}",
                safe_name
            ));
        }
        let mut importing = normalize_android_workspace_status(&state);
        importing.state = AndroidWorkspaceStateKind::Downloading;
        importing.last_error = None;
        importing.runtime_version = None;
        importing.download_bytes = Some(0);
        importing.download_total_bytes = Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
        importing.download_stage = Some("importing".to_string());
        android_workspace_set_status(&state, Some(&app), importing)?;
        match ensure_android_workspace_layout(&root) {
            Ok(()) => {
                let bytes = match B64.decode(data_base64.trim()) {
                    Ok(bytes) => bytes,
                    Err(err) => {
                        let error = format!("解析 Android Linux 运行环境压缩包失败: {err}");
                        let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                        failed.last_error = Some(error.clone());
                        let _ = android_workspace_set_status(&state, Some(&app), failed);
                        return Err(error);
                    }
                };
                android_workspace_update_download_progress(&state, &app, &root, bytes.len() as u64, "importing")?;
                if let Err(err) = import_android_workspace_rootfs_from_archive(&state, &app, &root, bytes).await {
                    let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                    failed.last_error = Some(err.clone());
                    let _ = android_workspace_set_status(&state, Some(&app), failed);
                    return Err(err);
                }
                android_workspace_set_status(&state, Some(&app), android_workspace_ready_status(&root))
            }
            Err(err) => {
                let mut failed = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &root);
                failed.last_error = Some(err.clone());
                let _ = android_workspace_set_status(&state, Some(&app), failed);
                Err(err)
            }
        }
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        let _ = state;
        let _ = file_name;
        let _ = data_base64;
        Err("Android Linux 运行环境压缩包导入仅在 Android 端可用。".to_string())
    }
}

#[tauri::command]
fn reset_android_workspace_state(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<AndroidWorkspaceStatus, String> {
    #[cfg(target_os = "android")]
    {
        let status = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, &android_workspace_root(&state));
        android_workspace_set_status(&state, Some(&app), status)
    }
    #[cfg(not(target_os = "android"))]
    {
        let _ = app;
        let root = android_workspace_root(&state);
        Ok(AndroidWorkspaceStatus {
            state: AndroidWorkspaceStateKind::Ready,
            root_path: shell_workspace_display_path(&root),
            initialized_at: None,
            updated_at: Some(now_iso()),
            last_error: None,
            version: 1,
            runtime_version: Some(ANDROID_WORKSPACE_ROOTFS_VERSION.to_string()),
            download_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
            download_stage: None,
        })
    }
}

#[tauri::command]
fn import_file_to_android_workspace(
    state: State<'_, AppState>,
    file_name: String,
    mime: Option<String>,
    data_base64: String,
    target_path: Option<String>,
) -> Result<AndroidWorkspaceImportResult, String> {
    #[cfg(target_os = "android")]
    {
        let status = normalize_android_workspace_status(&state);
        if !matches!(status.state, AndroidWorkspaceStateKind::Ready) {
            return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
        let _ = mime;
        let root = android_workspace_root(&state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        let mut target = android_workspace_resolve_import_target_path(&root, &file_name, target_path.as_deref())?;
        android_workspace_ensure_paths_within_sandbox(&state, &[target.clone()])?;
        android_workspace_ensure_user_file_manager_path(&root, &target, false)?;
        if target.exists() {
            target = android_workspace_unique_sibling_path(&target);
            android_workspace_ensure_paths_within_sandbox(&state, &[target.clone()])?;
            android_workspace_ensure_user_file_manager_path(&root, &target, false)?;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("创建 Android 工作区导入目录失败 ({}): {err}", parent.display()))?;
        }
        let safe_name = android_workspace_sanitize_file_name(&file_name);
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
        let status = normalize_android_workspace_status(&state);
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
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        let _ = file_name;
        let _ = mime;
        let _ = data_base64;
        let _ = target_path;
        Err("Android 工作区导入仅在 Android 端可用。".to_string())
    }
}

#[tauri::command]
fn export_file_from_android_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<AndroidWorkspaceExportResult, String> {
    #[cfg(target_os = "android")]
    {
        let status = normalize_android_workspace_status(&state);
        if !matches!(status.state, AndroidWorkspaceStateKind::Ready) {
            return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
        let target = android_workspace_resolve_file_manager_existing_path(&state, &path, false)?;
        let root = android_workspace_root(&state)
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
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        let _ = path;
        Err("Android 工作区导出仅在 Android 端可用。".to_string())
    }
}

#[tauri::command]
fn delete_file_from_android_workspace(
    state: State<'_, AppState>,
    path: String,
) -> Result<AndroidWorkspaceDeleteResult, String> {
    #[cfg(target_os = "android")]
    {
        let status = normalize_android_workspace_status(&state);
        if !matches!(status.state, AndroidWorkspaceStateKind::Ready) {
            return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
        let target = android_workspace_resolve_file_manager_existing_path(&state, &path, false)?;
        let root = android_workspace_root(&state)
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
    #[cfg(not(target_os = "android"))]
    {
        let _ = state;
        let _ = path;
        Err("Android 工作区删除仅在 Android 端可用。".to_string())
    }
}
