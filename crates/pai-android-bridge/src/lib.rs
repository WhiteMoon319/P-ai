//! P-AI Android 原生桥：JNI / runtime / dispatch / StateAccess trait / event queue re-export
//!
//! 本 crate 是 Android 后端的运行时中枢：
//! - `StateAccess` trait：抽象 AppState 的缓存+持久化接口，使业务逻辑可脱离 AppState 编译
//! - `NativeDispatcher` trait：JSON-RPC 方法分发接口（实现位于 src-tauri 侧）
//! - `TaskManager` trait + 任务状态机类型：长任务进度追踪（workspace/rootfs/migration）
//! - JNI 导出（阶段 6，待拆）
//!
//! 依赖方向：pai-android-bridge → pai-backend（类型） + pai-android-platform（平台操作）

pub mod dispatch;
pub mod state_access;
pub mod task;

pub use dispatch::*;
pub use state_access::*;
pub use task::*;