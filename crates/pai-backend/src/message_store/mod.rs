//! 消息存储索引/清单/路径/校验/快照/活跃计划（纯逻辑，无平台依赖）。

pub mod active_plan;
pub mod index;
pub mod jsonl_snapshot;
pub mod manifest;
pub mod meta;
pub mod migration;
pub mod paths;
pub mod persist;
pub mod sqlite;
pub mod store;
pub mod usage_trail;
pub mod verification;

pub use active_plan::*;
pub use index::*;
pub use jsonl_snapshot::*;
pub use manifest::*;
pub use meta::*;
pub use migration::*;
pub use paths::*;
pub use persist::*;
pub use sqlite::*;
pub use store::*;
pub use usage_trail::*;
pub use verification::*;
