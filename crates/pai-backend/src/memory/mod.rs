//! 记忆域（纯逻辑部分，阶段 4 逐步迁入）。

pub mod matcher;
pub mod providers;
pub mod store;

pub use matcher::*;
pub use providers::*;
pub use store::*;
