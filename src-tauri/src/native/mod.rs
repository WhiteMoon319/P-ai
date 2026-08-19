// ========== Android 原生桥（JNI）模块拆分 ==========
// 阶段 6：native_bridge.rs 拆分为 runtime / dispatch / jni 三文件。
// - runtime.rs：NativeRuntime 初始化（Tokio runtime + AppState + IdeContextRuntime 单例）
// - dispatch.rs：方法分发（NativeDispatcherImpl 实现，JSON-RPC 路由到 ide_chat_* 业务函数）
// - jni.rs：JNI 导出（PaiNative_init / PaiNative_call / PaiNative_pollEvents）
//
// 事件队列（native_delta_queue / push_native_delta_event / drain_native_delta_events）
// 已迁至 crates/pai-android-platform::event_queue（阶段 6）。
//
// 设计：
// - 全局 NativeRuntime 单例：自建 Tokio runtime + AppState + IdeContextRuntime
// - nativeInit(appRoot)：用应用数据目录初始化后端（等价原 tauri setup 的 AppState::new_with_root）
// - nativeCall(requestJson)：同步执行 JSON-RPC（block_on dispatch），返回响应 JSON

pub(crate) use pai_android_platform::event_queue::*;

include!("runtime.rs");
include!("dispatch.rs");
include!("jni.rs");