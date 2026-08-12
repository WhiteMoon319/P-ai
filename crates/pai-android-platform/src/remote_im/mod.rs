//! 远程 IM 平台 SDK（阶段 5 逐步迁入）。

pub mod feishu_sdk;
pub mod weixin_oc;
pub mod onebot_v11_ws;
pub mod dingtalk;
pub mod channel_store;
pub mod adapters;
pub mod message_routing;
pub mod contact_snapshots;

pub use feishu_sdk::*;
pub use weixin_oc::*;
pub use onebot_v11_ws::*;
pub use dingtalk::*;
pub use channel_store::*;
pub use adapters::*;
pub use message_routing::*;
pub use contact_snapshots::*;
