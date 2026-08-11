//! 远程 IM 平台 SDK（阶段 5 逐步迁入）。

pub mod feishu_sdk;
pub mod weixin_oc;
pub mod onebot_v11_ws;
pub mod dingtalk;

pub use feishu_sdk::*;
pub use weixin_oc::*;
pub use onebot_v11_ws::*;
pub use dingtalk::*;
