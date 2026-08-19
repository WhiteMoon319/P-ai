//! JSON-RPC 方法分发器 trait。
//!
//! `NativeDispatcher` 抽象了从方法名到 handler 的路由逻辑，
//! 使 `native_bridge.rs` 的 dispatch 部分可脱离 `NativeRuntime` 具体类型编译。
//!
//! 实现位于 src-tauri 侧（native_bridge.rs 或 native/dispatch_impl.rs）。

use serde_json::Value;

/// JSON-RPC 方法分发结果。
pub type DispatchResult = Result<Value, String>;

/// 原生方法分发器。
///
/// 接收 JSON-RPC 请求的方法名、参数与 id，返回响应值或错误。
/// 同步接口：实现方自行处理异步（tokio::task::block_in_place / runtime.block_on）。
pub trait NativeDispatcher: Send + Sync {
    /// 分发一个 JSON-RPC 方法调用。
    fn dispatch(&self, method: &str, params: Value, id: Option<Value>) -> DispatchResult;
}