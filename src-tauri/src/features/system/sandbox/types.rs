#[derive(Debug, Clone)]
struct SandboxRequest {
    session_id: String,
    command: String,
    cwd: std::path::PathBuf,
    timeout_ms: u64,
    cwd_pre_validated: bool,
    stdin: Option<Vec<u8>>,
    cancel_token: Option<tokio_util::sync::CancellationToken>,
}

#[derive(Debug, Clone)]
struct SandboxExecutionResult {
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
