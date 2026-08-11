
pub(crate) const TERMINAL_MAX_OUTPUT_BYTES: usize = 256 * 1024;
pub(crate) const TERMINAL_DEFAULT_TIMEOUT_MS: u64 = 300_000;

include!("terminal/runtime.rs");

// terminal/output.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::output::*;

include!("terminal/workspace.rs");

include!("terminal/matcher.rs");

include!("terminal/analyzer.rs");

include!("terminal/approval.rs");

include!("terminal/guards.rs");

include!("terminal/exec.rs");
