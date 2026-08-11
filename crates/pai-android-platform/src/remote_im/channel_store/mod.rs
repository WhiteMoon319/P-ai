//! 远程 IM 渠道私有状态存储（阶段 5 迁入）。

mod store;

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};

/// 渠道私有状态存储上下文：替代 src-tauri AppState 的 data_path + 写锁 map。
pub struct RemoteImChannelStoreCtx<'a> {
    pub data_path: &'a Path,
    pub write_locks: &'a Arc<Mutex<HashMap<String, Arc<Mutex<()>>>>>,
}

pub use store::*;
