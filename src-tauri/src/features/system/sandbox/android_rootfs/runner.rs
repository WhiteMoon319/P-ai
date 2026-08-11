use super::*;

pub(crate) const ANDROID_PROOT_EXEC: &str = "libproot_exec.so";
pub(crate) const ANDROID_PROOT_LOADER: &str = "libproot_loader.so";
pub(crate) const ANDROID_PROOT_REQUIRED_LIBS: &[&str] = &["libtalloc.so", "libandroid-shmem.so"];
pub(crate) const ANDROID_PROOT_TALLOC_COMPAT_LIB: &str = "libtalloc.so.2";
pub(crate) const ANDROID_PROOT_WORKSPACE_DIR: &str = "/workspace";
pub(crate) const ANDROID_PROOT_ASSISTANT_SPACE_DIR: &str = "/root/.pai";

pub(crate) fn android_proot_candidate_dirs() -> Vec<std::path::PathBuf> {
    let mut out = Vec::<std::path::PathBuf>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    let mut push = |path: std::path::PathBuf| {
        if !path.is_dir() {
            return;
        }
        let key = path.to_string_lossy().to_string();
        if seen.insert(key) {
            out.push(path);
        }
    };

    if let Ok(paths) = std::env::var("LD_LIBRARY_PATH") {
        for raw in paths.split(':') {
            let trimmed = raw.trim();
            if !trimmed.is_empty() {
                push(std::path::PathBuf::from(trimmed));
            }
        }
    }

    if let Ok(maps) = std::fs::read_to_string("/proc/self/maps") {
        for token in maps.split_whitespace() {
            if !token.ends_with(".so") {
                continue;
            }
            let path = std::path::PathBuf::from(token);
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if matches!(file_name, "libeasy_call_ai_lib.so" | "libtauri_app.so") || file_name.starts_with("libeasy_call") {
                if let Some(parent) = path.parent() {
                    push(parent.to_path_buf());
                }
            }
        }
    }

    out
}

pub(crate) fn android_proot_binary_paths() -> Result<(std::path::PathBuf, std::path::PathBuf, Option<std::path::PathBuf>), String> {
    let candidates = android_proot_candidate_dirs();
    for dir in &candidates {
        let proot = dir.join(ANDROID_PROOT_EXEC);
        if proot.is_file() {
            let loader = dir.join(ANDROID_PROOT_LOADER);
            let loader = if loader.is_file() { Some(loader) } else { None };
            return Ok((dir.clone(), proot, loader));
        }
    }
    let searched = candidates
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(format!(
        "Android Linux 执行层缺失：未找到 {ANDROID_PROOT_EXEC}。请把合法来源的 proot native libs 放入 src-tauri/android/jniLibs/<abi>/ 后重新打包。已搜索：{}",
        if searched.is_empty() { "无候选 nativeLibraryDir" } else { searched.as_str() }
    ))
}

pub(crate) fn android_proot_ensure_libs_dir(native_dir: &std::path::Path, temp_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let libs_dir = temp_dir.join("libs");
    std::fs::create_dir_all(&libs_dir)
        .map_err(|err| format!("创建 Android proot 库目录失败 ({}): {err}", libs_dir.display()))?;
    let mut missing = Vec::<String>::new();
    for lib_name in ANDROID_PROOT_REQUIRED_LIBS {
        let target = libs_dir.join(lib_name);
        if target.is_file() {
            continue;
        }
        let direct_source = native_dir.join(lib_name);
        let source = if direct_source.is_file() {
            Some(direct_source)
        } else if *lib_name == "libtalloc.so" {
            let compat_source = native_dir.join(ANDROID_PROOT_TALLOC_COMPAT_LIB);
            compat_source.is_file().then_some(compat_source)
        } else {
            None
        };
        if let Some(src) = source {
            std::fs::copy(&src, &target)
                .map_err(|err| format!("复制 Android proot 依赖库失败 ({} -> {}): {err}", src.display(), target.display()))?;
            continue;
        }
        missing.push((*lib_name).to_string());
    }
    if !missing.is_empty() {
        return Err(format!(
            "Android proot 依赖库缺失：missing={}，native_dir={}。请确认 APK 已打包 libtalloc.so 和 libandroid-shmem.so，且 proot 的 NEEDED 已从 libtalloc.so.2 修补为 libtalloc.so。",
            missing.join(","),
            native_dir.display()
        ));
    }
    Ok(libs_dir)
}

pub(crate) fn android_proot_workspace_cwd(root: &std::path::Path, cwd: &std::path::Path) -> Result<String, String> {
    let normalized = terminal_normalize_for_access_check(cwd);
    if !path_is_within(root, &normalized) {
        return Err(format!("Android Linux 命令工作目录不在沙盒内：{}", cwd.display()));
    }
    let relative = normalized
        .strip_prefix(root)
        .map_err(|_| format!("Android Linux 命令无法解析沙盒相对目录：{}", cwd.display()))?;
    let relative_text = relative.to_string_lossy().replace('\\', "/");
    if relative_text.trim().is_empty() {
        Ok(ANDROID_PROOT_WORKSPACE_DIR.to_string())
    } else {
        Ok(format!("{}/{}", ANDROID_PROOT_WORKSPACE_DIR, relative_text.trim_matches('/')))
    }
}

pub(crate) fn android_proot_shell_args(_runtime_root: &std::path::Path) -> (&'static str, Vec<&'static str>) {
    ("/bin/sh", vec!["-c"])
}

pub(crate) async fn sandbox_run_with_android_proot_backend(
    request: &SandboxRequest,
    state: &AppState,
) -> Result<SandboxExecutionResult, String> {
    // 使用 app 原生可写路径（不 canonicalize）：Android 上 /data/user/0/<pkg> 才是
    // app 可写前缀，canonicalize 会翻成 /data/data 别名导致 proot chmod glue 失败。
    // rootfs 与 PROOT_TMP_DIR 必须同源。
    let raw_root = android_workspace_root(state);
    let root = features_system_commands::android_workspace_manager::android_workspace_canonical_root_if_ready(state)?
        .ok_or_else(|| ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string())?;
    let runtime_root = android_workspace_runtime_root(&root);
    if !android_workspace_runtime_ready(&root) {
        return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
    }
    let temp_dir = android_workspace_runtime_base(&raw_root).join("tmp").join("proot");
    std::fs::create_dir_all(&temp_dir)
        .map_err(|err| format!("创建 Android proot 临时目录失败 ({}): {err}", temp_dir.display()))?;
    // proot --link2symlink 会在 PROOT_TMP_DIR 下创建 glue rootfs 并 chmod；
    // 目录不可写时 proot 直接失败并导致 rootfs 未挂载。启动前显式验证可写。
    let probe = temp_dir.join(".pai-proot-write-probe");
    std::fs::write(&probe, b"ok")
        .map_err(|err| format!("Android proot 临时目录不可写 ({}): {err}", temp_dir.display()))?;
    let _ = std::fs::remove_file(&probe);
    android_proot_ensure_host_pai_layout(&root)?;
    android_proot_patch_rootfs(&runtime_root)?;

    let (native_dir, proot, loader) = android_proot_binary_paths()?;
    if loader.is_none() {
        return Err(format!(
            "Android proot 缺少 libproot_loader.so（guest 进程初始化必需），请确认 APK 已打包该库：{}",
            native_dir.display()
        ));
    }
    let libs_dir = android_proot_ensure_libs_dir(&native_dir, &temp_dir)?;
    let proot_cwd = android_proot_workspace_cwd(&root, &request.cwd)?;
    let (shell_path, shell_args) = android_proot_shell_args(&runtime_root);

    // 预检 proot 入口：/bin/sh 在 Ubuntu 中经 /bin -> /usr/bin symlink 解析为 /usr/bin/sh，
    // 两者任一缺失都会在 execve 阶段直接失败，这里先给出可诊断错误。
    let usr_sh = runtime_root.join("usr").join("bin").join("sh");
    let bin_sh = runtime_root.join("bin").join("sh");
    if !usr_sh.is_file() && !bin_sh.is_file() {
        return Err(format!(
            "Android Linux rootfs 缺少可用 shell 入口（/usr/bin/sh 或 /bin/sh）。请先执行修复沙盒：{} / {}",
            usr_sh.display(),
            bin_sh.display()
        ));
    }

    let mut command_builder = tokio::process::Command::new(&proot);
    command_builder.kill_on_drop(true);
    command_builder.current_dir(&root);
    for key in [
        "LD_LIBRARY_PATH",
        "LD_PRELOAD",
        "LD_AUDIT",
        "DYLD_INSERT_LIBRARIES",
        "PROOT_LOADER",
        "PROOT_TMP_DIR",
    ] {
        command_builder.env_remove(key);
    }
    let ld_library_path = format!("{}:{}", libs_dir.display(), native_dir.display());
    command_builder.env("LD_LIBRARY_PATH", ld_library_path);
    if let Some(loader) = &loader {
        command_builder.env("PROOT_LOADER", loader);
    }
    command_builder.env("PROOT_TMP_DIR", &temp_dir);
    // proot 进程运行在宿主侧，TMPDIR 必须指向宿主可写目录（应用私有 runtime），
    // 不能写 guest 内的 /tmp，否则 --link2symlink glue 创建时 chmod 失败。
    command_builder.env("TMPDIR", &temp_dir);
    command_builder.env("HOME", "/root");
    command_builder.env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    command_builder.env("TERM", "xterm-256color");
    command_builder.env("LANG", "C.UTF-8");
    command_builder.env("LC_ALL", "C.UTF-8");
    command_builder.env("USER", "root");
    command_builder.env("SHELL", shell_path);
    command_builder.env("PAI_WORKSPACE", ANDROID_PROOT_WORKSPACE_DIR);
    command_builder.env("PAI_ASSISTANT_SPACE", ANDROID_PROOT_ASSISTANT_SPACE_DIR);
    command_builder.env("PAI_SKILLS_DIR", "/root/.pai/skills");

    command_builder.arg("--root-id");
    command_builder.arg("--link2symlink");
    command_builder.arg("--kill-on-exit");
    command_builder.arg("-r");
    command_builder.arg(&runtime_root);
    command_builder.arg("-w");
    command_builder.arg(&proot_cwd);
    command_builder.arg("-b");
    command_builder.arg(format!("{}:{ANDROID_PROOT_WORKSPACE_DIR}", root.display()));
    command_builder.arg("-b");
    command_builder.arg(format!("{}:{ANDROID_PROOT_ASSISTANT_SPACE_DIR}", root.display()));
    for path in ["/dev", "/proc", "/sys"] {
        if std::path::Path::new(path).exists() {
            command_builder.arg("-b");
            command_builder.arg(path);
        }
    }

    command_builder.arg(shell_path);
    for arg in shell_args {
        command_builder.arg(arg);
    }
    command_builder.arg("unset LD_LIBRARY_PATH LD_PRELOAD LD_AUDIT; cd -- \"$1\" && eval \"$2\"");
    command_builder.arg("pai-android");
    command_builder.arg(&proot_cwd);
    command_builder.arg(&request.command);

    runtime_log_debug(format!(
        "[Android 工作区] 启动 proot：rootfs={} tmp={} shell={} cwd={}",
        runtime_root.display(),
        temp_dir.display(),
        shell_path,
        proot_cwd
    ));

    let started = std::time::Instant::now();
    let output = sandbox_collect_output(
        command_builder,
        request.stdin.clone(),
        request.timeout_ms,
        request.cancel_token.clone(),
    )
    .await
    .map_err(|err| format!("terminal_exec android proot failed: {err}"))?;

    if !output.status.success() {
        runtime_log_warn(format!(
            "[Android 工作区] proot 执行失败 exit={:?} stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let exit_code = output.status.code().unwrap_or(-1);
    Ok(SandboxExecutionResult {
        ok: output.status.success(),
        exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        duration_ms,
        shell_kind: "android-proot".to_string(),
        shell_path: format!("{} via {}", shell_path, proot.to_string_lossy()),
    })
}
