//! P-AI 平台无关业务后端（Android 原生重构阶段 4）。
//!
//! 职责：会话 / 消息 / 记忆 / 任务 / 远程 IM / 配置 / migration 等纯业务逻辑，
//! 不依赖 tauri / jni / Android 平台能力；平台接入由 pai-android-bridge 与
//! pai-android-platform 提供（阶段 5-6）。
//!
//! 迁移策略：从 src-tauri 逐步搬入零平台依赖的纯逻辑模块，
//! 每个模块迁移后保持 `cargo check -p pai-backend` 与 Android 交叉编译通过。

use std::sync::{Mutex, OnceLock};

pub mod archive_host_selector;
pub mod archive_summary_parser;
pub mod core;
pub mod core_provider_gemini;
pub mod core_provider_utils;
pub mod delegate;
pub mod desktop_tools;
pub mod image_generation;
pub mod image_normalizer;
pub mod json_extractor;
pub mod logging;
pub mod mcp;
pub mod memory;
pub mod model_runtime;
pub mod message_store;
pub mod multilingual;
pub mod share_export;
pub mod screenshot_cache;
pub mod screenshot_cache_types;
pub mod skill;
pub mod task;
pub mod tool_loop;
pub mod tool_policy;
pub mod terminal;
pub mod provider_resolution;
pub mod text_codec;
pub mod tool_arg_types;
pub mod version_compare;

// 原生流式事件出口：由平台接入方（Android 的 pai-android-platform event_queue）
// 注册 sink，Kotlin 通过 pollEvents 轮询弹出。pai-backend 保持平台无关，不直接持有"最终队列"。
type NativeDeltaSink = Box<dyn Fn(serde_json::Value) + Send + Sync + 'static>;

static NATIVE_DELTA_SINK: OnceLock<Mutex<Option<NativeDeltaSink>>> = OnceLock::new();

/// 注册原生流式事件转发目标（Android：pai-android-platform event_queue）。
/// 应在平台初始化（nativeInit）时调用一次，确保所有 DeltaChannel::send 事件正确入队。
pub fn set_native_delta_event_sink(sink: impl Fn(serde_json::Value) + Send + Sync + 'static) {
    let slot = NATIVE_DELTA_SINK.get_or_init(|| Mutex::new(None));
    if let Ok(mut guard) = slot.lock() {
        *guard = Some(Box::new(sink));
    }
}

/// 把一条流式事件转发给已注册的平台事件队列（Android 分支专用）。
/// 未注册 sink 时丢弃并记录日志（平台初始化必然先于任何 delta 产生）。
pub fn push_native_delta_event(event: serde_json::Value) {
    let slot = NATIVE_DELTA_SINK.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = slot.lock() {
        if let Some(ref sink) = *guard {
            sink(event);
            return;
        }
    }
    eprintln!("[pai-backend] native delta sink 未注册，丢弃事件: {}", event);
}
