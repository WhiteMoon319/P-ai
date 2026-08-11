
pub(crate) const TERMINAL_MAX_OUTPUT_BYTES: usize = 256 * 1024;
pub(crate) const TERMINAL_DEFAULT_TIMEOUT_MS: u64 = 300_000;

include!("terminal/runtime.rs");

include!("terminal/output.rs");

include!("terminal/workspace.rs");

include!("terminal/matcher.rs");

include!("terminal/analyzer.rs");

include!("terminal/approval.rs");

include!("terminal/guards.rs");

include!("terminal/exec.rs");
