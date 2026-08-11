//! 远程 IM SDK 适配层（Dingtalk/OnebotV11/WeixinOc 出站 SDK，阶段 5 迁入）。

mod adapters;

use std::collections::HashMap;
use std::sync::Arc;

pub use serde_json::Value;
pub use uuid::Uuid;

use pai_backend::core::domain::types_config::{RemoteImChannelConfig, RemoteImPlatform};
use pai_backend::core::domain::types_storage::{RemoteImContact, RemoteImGroupMemberInfo};
use pai_backend::logging::{runtime_log_error, runtime_log_info, runtime_log_warn};

pub use adapters::*;
