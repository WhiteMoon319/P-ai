use super::*;

pub(crate) const TERMINAL_MAX_OUTPUT_BYTES: usize = 256 * 1024;
pub(crate) const TERMINAL_DEFAULT_TIMEOUT_MS: u64 = 300_000;

#[path = "terminal/runtime.rs"]
mod terminal_runtime;
pub(crate) use terminal_runtime::*;

// terminal/output.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::output::*;
// terminal/matcher.rs 与 terminal/analyzer.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::matcher::*;
pub(crate) use pai_backend::terminal::analyzer::*;

#[path = "terminal/workspace.rs"]
mod terminal_workspace;
pub(crate) use terminal_workspace::*;

#[path = "terminal/approval.rs"]
mod terminal_approval;
pub(crate) use terminal_approval::*;

// terminal/guards.rs 已迁至 crates/pai-backend（阶段 4）。
pub(crate) use pai_backend::terminal::guards::*;

#[path = "terminal/exec.rs"]
mod terminal_exec;
pub(crate) use terminal_exec::*;
