use super::*;

pub(crate) const TERMINAL_MAX_OUTPUT_BYTES: usize = 256 * 1024;
pub(crate) const TERMINAL_DEFAULT_TIMEOUT_MS: u64 = 300_000;

include!("terminal/runtime.rs");

// terminal/output.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::output::*;
// terminal/matcher.rs 与 terminal/analyzer.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::matcher::*;
pub(crate) use pai_backend::terminal::analyzer::*;

include!("terminal/workspace.rs");

include!("terminal/approval.rs");

// terminal/guards.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::guards::*;

include!("terminal/exec.rs");
