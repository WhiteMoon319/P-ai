//! 消息存储索引/清单/路径（纯逻辑，无平台依赖）。

pub mod index;
pub mod manifest;
pub mod paths;

pub use index::*;
pub use manifest::*;
pub use paths::*;
