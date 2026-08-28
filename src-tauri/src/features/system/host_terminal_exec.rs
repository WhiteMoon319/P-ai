// ==================== Host 端终端命令执行 ====================
// 仓库 Android-only 化时删除了桌面 execution 模块（run_command_in_workspace），
// 但非 Android cfg 分支（host 单测编译）仍引用它。本文件提供 host 端等价实现：
// - Windows：std::process::Command + WindowsJobGuard（进程树清理）
// - 非 Windows：tokio::process::Command + 进程组清理
// 仅 #[cfg(not(target_os = "android"))] include（见 lib.rs），Android 不受影响。

#[derive(Debug, Clone)]
struct HostExecutionRequest {
    session_id: String,
    command: String,
    cwd: std::path::PathBuf,
    timeout_ms: u64,
}

#[derive(Debug, Clone)]
struct ExecutionResult {
    ok: bool,
    exit_code: i32,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
    duration_ms: u64,
    #[allow(dead_code)]
    shell_kind: String,
    #[allow(dead_code)]
    shell_path: String,
}

#[cfg(target_os = "windows")]
fn host_windows_wrap_command_for_shell(shell: &TerminalShellProfile, command: &str) -> String {
    if matches!(shell.kind.as_str(), "powershell7" | "powershell5") {
        return format!(
            "$ErrorActionPreference='Continue'; try {{ [Console]::InputEncoding = [System.Text.UTF8Encoding]::new($false); [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false); $OutputEncoding = [Console]::OutputEncoding; chcp.com 65001 > $null; $env:PYTHONUTF8='1'; $env:PYTHONIOENCODING='utf-8'; {command} }} catch {{ Write-Error $_; $global:LASTEXITCODE = 1 }}; exit $(if ($null -eq $LASTEXITCODE) {{ 0 }} else {{ $LASTEXITCODE }})"
        );
    }
    if shell.kind == "git-bash" {
        return format!("chcp.com 65001 > /dev/null 2>&1; export LANG=en_US.UTF-8; export LC_ALL=en_US.UTF-8; export PYTHONUTF8=1; export PYTHONIOENCODING=utf-8; {command}");
    }
    command.to_string()
}

#[cfg(target_os = "windows")]
fn host_windows_process_compatible_path(path: &std::path::Path) -> std::path::PathBuf {
    let raw = path.as_os_str().to_string_lossy();
    if let Some(rest) = raw.strip_prefix(r"\\?\UNC\") {
        return std::path::PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = raw.strip_prefix(r"\\?\") {
        return std::path::PathBuf::from(rest);
    }
    path.to_path_buf()
}

#[cfg(target_os = "windows")]
fn host_run_command_in_workspace_windows_blocking(
    shell: &TerminalShellProfile,
    request: &HostExecutionRequest,
) -> Result<ExecutionResult, String> {
    use std::io::Read as _;
    use std::os::windows::io::AsRawHandle as _;
    use std::os::windows::process::CommandExt as _;

    use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;

    let mut command_builder = std::process::Command::new(&shell.path);
    let cwd = host_windows_process_compatible_path(&request.cwd);
    let wrapped_command = host_windows_wrap_command_for_shell(shell, &request.command);
    command_builder.current_dir(&cwd);
    command_builder.stdout(std::process::Stdio::piped());
    command_builder.stderr(std::process::Stdio::piped());
    command_builder.stdin(std::process::Stdio::null());
    command_builder.creation_flags(CREATE_NO_WINDOW);
    terminal_apply_windows_utf8_env(&mut command_builder);
    for arg in &shell.args_prefix {
        command_builder.arg(arg);
    }
    command_builder.arg(&wrapped_command);

    let mut child = command_builder
        .spawn()
        .map_err(|err| format!("terminal_exec host command spawn failed: {err}"))?;

    // 进程树清理：子进程退出或超时 kill 时整树回收，防止后台派生进程悬挂。
    let job = WindowsJobGuard::create_kill_on_close()?;
    job.assign_raw_process_handle(
        child.as_raw_handle() as windows_sys::Win32::Foundation::HANDLE,
    )
    .map_err(|err| format!("{}: pid={}", err, child.id()))?;

    let mut stdout_pipe = child
        .stdout
        .take()
        .ok_or_else(|| "Capture child stdout failed.".to_string())?;
    let mut stderr_pipe = child
        .stderr
        .take()
        .ok_or_else(|| "Capture child stderr failed.".to_string())?;

    let stdout_reader = std::thread::spawn(move || {
        let mut buf = Vec::<u8>::new();
        let _ = stdout_pipe.read_to_end(&mut buf);
        buf
    });
    let stderr_reader = std::thread::spawn(move || {
        let mut buf = Vec::<u8>::new();
        let _ = stderr_pipe.read_to_end(&mut buf);
        buf
    });

    let timeout_ms = request.timeout_ms.max(1);
    let started = std::time::Instant::now();
    loop {
        if let Some(_status) = child
            .try_wait()
            .map_err(|err| format!("terminal_exec try_wait failed: {err}"))?
        {
            break;
        }
        if started.elapsed().as_millis() >= timeout_ms as u128 {
            drop(job);
            let _ = child.kill();
            let cleanup_started = std::time::Instant::now();
            while cleanup_started.elapsed().as_millis() < 2_000 {
                if child
                    .try_wait()
                    .map_err(|err| format!("terminal_exec cleanup try_wait failed: {err}"))?
                    .is_some()
                {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            return Err(format!(
                "terminal_exec timed out after {}ms",
                timeout_ms
            ));
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }

    let status = child
        .wait()
        .map_err(|err| format!("terminal_exec wait failed: {err}"))?;
    // 根进程退出后立即关闭 Job，避免后代进程持有 stdout/stderr 句柄导致 reader 挂起。
    drop(job);
    let stdout = stdout_reader
        .join()
        .map_err(|_| "Join stdout reader thread failed.".to_string())?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "Join stderr reader thread failed.".to_string())?;
    let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;

    Ok(ExecutionResult {
        ok: status.success(),
        exit_code: status.code().unwrap_or(-1),
        stdout,
        stderr,
        duration_ms,
        shell_kind: shell.kind.clone(),
        shell_path: shell.path.clone(),
    })
}

async fn run_command_in_workspace(
    state: &AppState,
    _session_id: &str,
    command: &str,
    cwd: &std::path::Path,
    timeout_ms: u64,
    _cwd_policy_exempt: bool,
) -> Result<ExecutionResult, String> {
    let runtime_shell = terminal_shell_for_state(state);
    let request = HostExecutionRequest {
        session_id: _session_id.to_string(),
        command: command.to_string(),
        cwd: cwd.to_path_buf(),
        timeout_ms,
    };

    #[cfg(target_os = "windows")]
    {
        let shell = runtime_shell.clone();
        tokio::task::spawn_blocking(move || {
            host_run_command_in_workspace_windows_blocking(&shell, &request)
        })
        .await
        .map_err(|err| format!("Join host command backend worker failed: {err}"))?
    }

    #[cfg(not(target_os = "windows"))]
    {
        use tokio::io::AsyncReadExt as _;

        let mut command_builder = tokio::process::Command::new(&runtime_shell.path);
        command_builder.kill_on_drop(true);
        #[cfg(unix)]
        {
            command_builder.process_group(0);
        }
        command_builder.current_dir(&request.cwd);
        command_builder.stdout(std::process::Stdio::piped());
        command_builder.stderr(std::process::Stdio::piped());
        command_builder.stdin(std::process::Stdio::null());
        for arg in &runtime_shell.args_prefix {
            command_builder.arg(arg);
        }
        command_builder.arg(&request.command);

        let mut child = command_builder
            .spawn()
            .map_err(|err| format!("terminal_exec host spawn failed: {err}"))?;
        let child_pid = child.id();
        #[cfg(unix)]
        let mut process_group_guard = HostProcessGroupGuard::new(child_pid);
        let mut stdout_pipe = child
            .stdout
            .take()
            .ok_or_else(|| "Capture child stdout failed.".to_string())?;
        let mut stderr_pipe = child
            .stderr
            .take()
            .ok_or_else(|| "Capture child stderr failed.".to_string())?;

        let stdout_task = tokio::spawn(async move {
            let mut buf = Vec::<u8>::new();
            stdout_pipe.read_to_end(&mut buf).await.map(|_| buf)
        });
        let stderr_task = tokio::spawn(async move {
            let mut buf = Vec::<u8>::new();
            stderr_pipe.read_to_end(&mut buf).await.map(|_| buf)
        });

        let timeout_ms = request.timeout_ms.max(1);
        let started = std::time::Instant::now();
        let status = tokio::select! {
            status = child.wait() => {
                status.map_err(|err| format!("terminal_exec host wait failed: {err}"))?
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(timeout_ms)) => {
                host_kill_process_group(child_pid);
                let _ = child.kill().await;
                let _ = child.wait().await;
                #[cfg(unix)]
                process_group_guard.disarm();
                return Err(format!("terminal_exec timed out after {}ms", timeout_ms));
            }
        };

        let stdout = host_join_reader_task(stdout_task, "stdout").await?;
        let stderr = host_join_reader_task(stderr_task, "stderr").await?;
        let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
        #[cfg(unix)]
        process_group_guard.disarm();
        let exit_code = status.code().unwrap_or(-1);
        Ok(ExecutionResult {
            ok: status.success(),
            exit_code,
            stdout,
            stderr,
            duration_ms,
            shell_kind: runtime_shell.kind.clone(),
            shell_path: runtime_shell.path.clone(),
        })
    }
}

#[cfg(not(target_os = "windows"))]
async fn host_join_reader_task(
    task: tokio::task::JoinHandle<Result<Vec<u8>, std::io::Error>>,
    name: &str,
) -> Result<Vec<u8>, String> {
    match task.await {
        Ok(Ok(buf)) => Ok(buf),
        Ok(Err(err)) => Err(format!("terminal_exec host read {name} failed: {err}")),
        Err(err) => Err(format!("Join {name} reader task failed: {err}")),
    }
}

#[cfg(unix)]
struct HostProcessGroupGuard {
    pid: Option<u32>,
    armed: bool,
}

#[cfg(unix)]
impl HostProcessGroupGuard {
    fn new(pid: Option<u32>) -> Self {
        Self {
            pid,
            armed: pid.is_some(),
        }
    }

    fn disarm(&mut self) {
        self.armed = false;
    }
}

#[cfg(unix)]
impl Drop for HostProcessGroupGuard {
    fn drop(&mut self) {
        if self.armed {
            if let Some(pid) = self.pid {
                host_kill_process_group(Some(pid));
            }
        }
    }
}

#[cfg(unix)]
fn host_kill_process_group(pid: Option<u32>) {
    if let Some(pid) = pid {
        unsafe {
            libc::kill(-(pid as i32), libc::SIGKILL);
        }
    }
}