//! P-AI Android 原生桥：JNI / runtime / dispatch / StateAccess trait / 任务状态机
//!
//! 本 crate 是 Android 后端的运行时中枢（阶段 6 交付物）：
//! - `StateAccess` trait：抽象 AppState 的缓存+持久化接口，使业务逻辑可脱离 AppState 编译
//! - `NativeDispatcher` trait：JSON-RPC 方法分发接口（实现位于 src-tauri 侧 native_bridge.rs）
//! - `TaskManager` trait + `DefaultTaskManager`：长任务进度追踪，自动推送事件到 Kotlin
//!
//! 依赖方向：pai-android-bridge → pai-backend（类型） + pai-android-platform（平台操作）

pub mod dispatch;
pub mod state_access;
pub mod task;

pub use dispatch::*;
pub use state_access::*;
pub use task::*;