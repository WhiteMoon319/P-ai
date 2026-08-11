//! P-AI 进程内 JSON-RPC 协议基础类型。
//!
//! 唯一协议来源：`contracts/native-rpc/methods.json` / `events.json`。
//! 本 crate 零平台依赖，Rust dispatcher 与 Kotlin client 共用同一套方法名。

pub mod errors;
pub mod events;
pub mod methods;
pub mod rpc;

pub use rpc::{RpcError, RpcRequest, RpcResponse};
