//! 原生流式事件队列（阶段 6 迁入）。
//!
//! Kotlin 通过 pollEvents 轮询弹出，dispatch_assistant_delta_to_active_view
//! 在 Android 分支把所有 delta 事件 push 进来。
//!
//! 设计：单一队列 + 顺序 tryEmit（禁止并发 launch emit / collectLatest）。

use std::sync::{Mutex, OnceLock};

static NATIVE_DELTA_QUEUE: OnceLock<Mutex<Vec<serde_json::Value>>> = OnceLock::new();

/// 获取原生事件队列（全局单例）。
pub fn native_delta_queue() -> &'static Mutex<Vec<serde_json::Value>> {
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

/// 弹出并清空当前事件队列，返回 JSON 数组字符串。
pub fn drain_native_delta_events() -> String {
    let mut events = Vec::new();
    if let Ok(mut guard) = native_delta_queue().lock() {
        std::mem::swap(&mut events, &mut guard);
    }
    match serde_json::to_string(&events) {
        Ok(json) => json,
        Err(_) => "[]".to_string(),
    }
}