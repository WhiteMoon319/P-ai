//! P-AI Android 原生桥：JNI / runtime / dispatch / StateAccess trait / event queue re-export
//!
//! 本 crate 是 Android 后端的运行时中枢：
//! - `StateAccess` trait：抽象 AppState 的缓存+持久化接口，使业务逻辑可脱离 AppState 编译
//! - dispatch：JSON-RPC 方法分发（阶段 6）
//! - JNI 导出（阶段 6）
//! - 任务句柄（阶段 6）
//!
//! 依赖方向：pai-android-bridge → pai-backend（类型） + pai-android-platform（平台操作）

pub mod state_access;

pub use state_access::*;