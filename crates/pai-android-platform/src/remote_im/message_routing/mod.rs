//! 远程 IM 消息路由与展示辅助（阶段 5 迁入）。

mod routing;

use std::collections::HashMap;
use std::sync::Arc;

pub use serde_json::Value;
pub use uuid::Uuid;

use pai_backend::core::domain::runtime_types::{RemoteImPresenceState, RemoteImWorkState};
use pai_backend::core::domain::types_chat::{ChatMessage, RemoteImActivationSource};
use pai_backend::core::domain::types_config::{AppConfig, RemoteImChannelConfig, RemoteImPlatform};
use pai_backend::core::domain::types_requests::{
    AttachmentMetaInput, BinaryPart, RemoteImEnqueueInput,
};
use pai_backend::core::domain::types_storage::RemoteImContact;

use crate::local_port_service::ChannelLogEntry;
use crate::remote_im::dingtalk::dingtalk_stream_manager;
use crate::remote_im::onebot_v11_ws::onebot_v11_ws_manager;
use crate::remote_im::weixin_oc::weixin_oc_manager;

pub use routing::*;
