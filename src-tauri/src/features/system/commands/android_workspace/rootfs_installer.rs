use super::*;

use pai_android_bridge::TaskManager;

pub(crate) fn android_workspace_rootfs_archive_path(root: &std::path::Path) -> PathBuf {
    android_workspace_runtime_base(root)
        .join("tmp")
        .join(ANDROID_WORKSPACE_ROOTFS_FILE_NAME)
}

pub(crate) fn android_workspace_rootfs_staging_root(root: &std::path::Path) -> PathBuf {
    android_workspace_runtime_base(root).join("tmp").join("rootfs-staging")
}

// android_workspace_apply_static_webpki_roots 已迁至 crates/pai-android-platform
// （阶段 5），通过 crate 根重导出生效。

pub(crate) fn android_workspace_update_download_progress(
    state: &AppState,
    app: &NativeAppHandle,
    root: &std::path::Path,
    bytes: u64,
    stage: &str,
) -> Result<(), String> {
    let (llm_workspace_root, runtime_root) = android_workspace_status_paths(root);
    let status = AndroidWorkspaceStatus {
        state: AndroidWorkspaceStateKind::Downloading,
        root_path: llm_workspace_root.clone(),
        llm_workspace_root,
        runtime_root,
        initialized_at: None,
        updated_at: Some(now_iso()),
        last_error: None,
        version: ANDROID_WORKSPACE_STATUS_VERSION,
        runtime_version: None,
        download_bytes: Some(bytes),
        download_total_bytes: Some(ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH),
        download_stage: Some(stage.to_string()),
    };
    // 通过 DefaultTaskManager 推送统一的任务进度事件（Kotlin pollEvents 的 task.progress）。
    // taskId 与 android_workspace 长任务入口（init/import）创建的 workspace-init 一致。
    let progress = (bytes as f64 / ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH as f64).clamp(0.0, 1.0);
    let _ = pai_android_bridge::DefaultTaskManager.update_task(
        "workspace-init",
        pai_android_bridge::TaskState::Running,
        progress,
        stage,
    );
    android_workspace_set_status(state, Some(app), status).map(|_| ())
}

pub(crate) async fn download_android_workspace_rootfs(
    state: &AppState,
    app: &NativeAppHandle,
    root: &std::path::Path,
) -> Result<PathBuf, String> {
    let archive_path = android_workspace_rootfs_archive_path(root);
    if archive_path.exists() {
        fs::remove_file(&archive_path)
            .map_err(|err| format!("清理 Android Linux 下载缓存失败 ({}): {err}", archive_path.display()))?;
    }
    android_workspace_update_download_progress(state, app, root, 0, "connecting")?;
    eprintln!("[Android 工作区] 开始下载 Linux 运行环境 url={} expected_bytes={}", ANDROID_WORKSPACE_ROOTFS_URL, ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH);
    let client_builder = reqwest::Client::builder()
        .user_agent(app_http_user_agent())
        .timeout(std::time::Duration::from_secs(600))
        .connect_timeout(std::time::Duration::from_secs(20))
        .redirect(reqwest::redirect::Policy::limited(10));
    #[cfg(target_os = "android")]
    let client_builder = android_workspace_apply_static_webpki_roots(client_builder)?;
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

pub(crate) fn verify_android_workspace_rootfs_archive(path: &std::path::Path) -> Result<(), String> {
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

// rootfs_paths 已迁至 crates/pai-android-platform（阶段 5）。
use pai_android_platform::android_workspace::rootfs_paths::*;

pub(crate) fn android_workspace_unpack_rootfs_symlink<R: std::io::Read>(
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
            .map(|_| ())
            .map_err(|err| format!("解压 Android Linux 运行环境符号链接失败 ({}): {err}", relative_path.display()))
    }
}

pub(crate) fn android_workspace_unpack_rootfs_hard_link<R: std::io::Read>(
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

pub(crate) fn write_android_workspace_rootfs_marker(runtime_root: &std::path::Path) -> Result<(), String> {
    let marker = serde_json::json!({
        "version": ANDROID_WORKSPACE_ROOTFS_VERSION,
        "source": ANDROID_WORKSPACE_ROOTFS_URL,
        "sha256": ANDROID_WORKSPACE_ROOTFS_SHA256,
        "installedAt": now_iso(),
    });
    let marker_body = serde_json::to_vec_pretty(&marker)
        .map_err(|err| format!("序列化 Android Linux 运行环境标记失败: {err}"))?;
    fs::write(runtime_root.join(ANDROID_WORKSPACE_ROOTFS_MARKER_FILE), marker_body)
        .map_err(|err| format!("写入 Android Linux 运行环境标记失败: {err}"))
}

pub(crate) fn extract_android_workspace_rootfs_archive(
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
    write_android_workspace_rootfs_marker(runtime_root)?;
    Ok(())
}

pub(crate) fn install_android_workspace_rootfs_archive(
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

pub(crate) async fn ensure_android_workspace_rootfs(
    state: &AppState,
    app: &NativeAppHandle,
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

pub(crate) async fn import_android_workspace_rootfs_from_archive(
    state: &AppState,
    app: &NativeAppHandle,
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
