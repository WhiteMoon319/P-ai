#[derive(Debug, Clone)]
pub struct SandboxRequest {
    pub session_id: String,
    pub command: String,
    pub cwd: std::path::PathBuf,
    pub timeout_ms: u64,
    pub cwd_pre_validated: bool,
    pub stdin: Option<Vec<u8>>,
    pub cancel_token: Option<tokio_util::sync::CancellationToken>,
}

#[derive(Debug, Clone)]
pub struct SandboxExecutionResult {
    pub ok: bool,
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub duration_ms: u64,
    #[allow(dead_code)]
    pub shell_kind: String,
    #[allow(dead_code)]
    pub shell_path: String,
}
