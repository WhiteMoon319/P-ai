#[derive(Debug, Clone)]
pub(crate) struct SandboxRequest {
    pub(crate) session_id: String,
    pub(crate) command: String,
    pub(crate) cwd: std::path::PathBuf,
    pub(crate) timeout_ms: u64,
    pub(crate) cwd_pre_validated: bool,
    pub(crate) stdin: Option<Vec<u8>>,
    pub(crate) cancel_token: Option<tokio_util::sync::CancellationToken>,
}

#[derive(Debug, Clone)]
pub(crate) struct SandboxExecutionResult {
    pub(crate) ok: bool,
    pub(crate) exit_code: i32,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) duration_ms: u64,
    #[allow(dead_code)]
    pub(crate) shell_kind: String,
    #[allow(dead_code)]
    pub(crate) shell_path: String,
}
