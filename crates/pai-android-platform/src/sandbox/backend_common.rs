// ==================== 沙盒执行公共收集 ====================
// 各后端统一通过本函数执行子进程：支持可选 stdin 写入、超时中断与取消中断。
// 取消/超时路径依赖各后端 command_builder 上设置的 kill_on_drop(true) 回收进程。

use pai_backend::logging::runtime_log_warn;

pub async fn sandbox_collect_output(
    mut command_builder: tokio::process::Command,
    stdin: Option<Vec<u8>>,
    timeout_ms: u64,
    cancel_token: Option<tokio_util::sync::CancellationToken>,
) -> Result<std::process::Output, String> {
    if stdin.is_some() {
        command_builder.stdin(std::process::Stdio::piped());
    }
    command_builder.stdout(std::process::Stdio::piped());
    command_builder.stderr(std::process::Stdio::piped());
    let mut child = command_builder
        .spawn()
        .map_err(|err| format!("terminal_exec spawn failed: {err}"))?;

    let mut stdout_pipe = child.stdout.take();
    let mut stderr_pipe = child.stderr.take();
    let read_stdout = async move {
        let mut buf = Vec::<u8>::new();
        if let Some(mut pipe) = stdout_pipe.take() {
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut pipe, &mut buf).await;
        }
        buf
    };
    let read_stderr = async move {
        let mut buf = Vec::<u8>::new();
        if let Some(mut pipe) = stderr_pipe.take() {
            let _ = tokio::io::AsyncReadExt::read_to_end(&mut pipe, &mut buf).await;
        }
        buf
    };
    // 先启动 stdout/stderr 读取任务，再写 stdin：避免子进程输出超过管道缓冲、
    // 且依赖 stdin 推进时双方互相等待造成死锁。
    let stdout_reader = tokio::spawn(read_stdout);
    let stderr_reader = tokio::spawn(read_stderr);

    if let Some(bytes) = stdin {
        if let Some(mut child_stdin) = child.stdin.take() {
            if let Err(err) = tokio::io::AsyncWriteExt::write_all(&mut child_stdin, &bytes).await {
                // 子进程可能提前退出并关闭 stdin（EPIPE），此时命令结果仍有效；
                // 仅在非 EPIPE 时视为致命错误。
                if err.kind() != std::io::ErrorKind::BrokenPipe {
                    return Err(format!("terminal_exec stdin write failed: {err}"));
                }
                runtime_log_warn(format!(
                    "[沙盒执行] 写入 stdin 时管道已关闭（子进程提前退出），继续等待结果，error={err}"
                ));
            }
        }
    }

    let wait_and_collect = async {
        let status = child
            .wait()
            .await
            .map_err(|err| format!("terminal_exec wait failed: {err}"))?;
        let stdout = stdout_reader
            .await
            .map_err(|err| format!("Join stdout reader task failed: {err}"))?;
        let stderr = stderr_reader
            .await
            .map_err(|err| format!("Join stderr reader task failed: {err}"))?;
        Ok::<std::process::Output, String>(std::process::Output {
            status,
            stdout,
            stderr,
        })
    };
    let cancel_wait = async {
        match &cancel_token {
            Some(token) => token.cancelled().await,
            None => std::future::pending::<()>().await,
        }
    };

    let run = async {
        tokio::pin!(wait_and_collect);
        tokio::pin!(cancel_wait);
        tokio::select! {
            result = &mut wait_and_collect => result,
            _ = &mut cancel_wait => Err("terminal_exec cancelled".to_string()),
        }
    };
    match tokio::time::timeout(std::time::Duration::from_millis(timeout_ms.max(1)), run).await {
        Ok(result) => result,
        Err(_) => Err(format!("terminal_exec timed out after {}ms", timeout_ms)),
    }
}
