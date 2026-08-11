//! 工具循环保护（纯逻辑，无平台依赖）。

pub mod repeat_guard;
pub mod tool_event_projection;

pub use repeat_guard::*;
pub use tool_event_projection::*;
