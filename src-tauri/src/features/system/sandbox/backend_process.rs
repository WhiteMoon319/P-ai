#[cfg(not(any(target_os = "android", target_os = "windows", target_os = "linux", target_os = "macos")))]
pub(crate) async fn sandbox_run_with_process_backend(
    shell: &TerminalShellProfile,
    request: &SandboxRequest,
) -> Result<SandboxExecutionResult, String> {
    let mut command_builder = tokio::process::Command::new(&shell.path);
    command_builder.kill_on_drop(true);
    command_builder.current_dir(&request.cwd);
    for arg in &shell.args_prefix {
        command_builder.arg(arg);
    }
    command_builder.arg(&request.command);

    let started = std::time::Instant::now();
    let output = sandbox_collect_output(
        command_builder,
        request.stdin.clone(),
        request.timeout_ms,
        request.cancel_token.clone(),
    )
    .await?;

    let duration_ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
    let exit_code = output.status.code().unwrap_or(-1);
    Ok(SandboxExecutionResult {
        ok: output.status.success(),
        exit_code,
        stdout: output.stdout,
        stderr: output.stderr,
        duration_ms,
        shell_kind: shell.kind.clone(),
        shell_path: shell.path.clone(),
    })
}
