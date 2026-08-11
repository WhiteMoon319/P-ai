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
pub mod delegate;
pub mod desktop_tools;
pub mod image_generation;
pub mod json_extractor;
pub mod logging;
pub mod mcp;
pub mod memory;
pub mod skill;
pub mod task;
pub mod text_codec;
pub mod version_compare;

/// 原生流式事件队列：Kotlin 通过 pollEvents 轮询弹出。
/// Android 原生模式下所有 delta 事件 push 进来，AppViewModel/前端轮询取出。
/// （阶段 4 从 src-tauri native_bridge 迁入；桌面端 tauri Channel 分支已剥离。）
static NATIVE_DELTA_QUEUE: OnceLock<Mutex<Vec<serde_json::Value>>> = OnceLock::new();

fn native_delta_queue() -> &'static Mutex<Vec<serde_json::Value>> {
    NATIVE_DELTA_QUEUE.get_or_init(|| Mutex::new(Vec::new()))
}

/// 把一条流式事件追加进原生事件队列（Android 分支专用）。
pub fn push_native_delta_event(event: serde_json::Value) {
    if let Ok(mut guard) = native_delta_queue().lock() {
        guard.push(event);
        // 队列只作短暂缓冲，Kotlin 高频轮询清空，不会无限增长。
        if guard.len() > 4096 {
            let len = guard.len();
            let overflow = guard.split_off(len - 2048);
            *guard = overflow;
        }
    }
}
