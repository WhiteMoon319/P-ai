//! 核心领域常量与默认值（纯逻辑，无平台依赖）。

pub mod constants;
pub mod remote_customer_service_defaults;
pub mod runtime_defaults;
pub mod types_chat;
pub mod types_config;
pub mod types_foundation;
pub mod types_image_generation;
pub mod types_requests;
pub mod types_storage;

pub use constants::*;
pub use remote_customer_service_defaults::*;
pub use runtime_defaults::*;
pub use types_chat::*;
pub use types_config::*;
pub use types_foundation::*;
pub use types_image_generation::*;
pub use types_requests::*;
pub use types_storage::*;
