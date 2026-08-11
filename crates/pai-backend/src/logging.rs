//! 运行日志（纯逻辑，无平台依赖）。
//!
//! 阶段 4 从 src-tauri debug_log_commands.rs 迁入的轻量实现：
//! 只做 stderr 输出，不维护 UI 环形缓冲（原缓冲在 Android 模式无消费者）。

/// 按级别推送一条日志到 stderr。
pub fn runtime_log_push(level: &str, message: String) {
    let _ = std::io::Write::write_all(&mut std::io::stderr(), format!("[{level}] {message}\n").as_bytes());
}

pub fn runtime_log_info(message: String) {
    runtime_log_push("info", message);
}

pub fn runtime_log_warn(message: String) {
    runtime_log_push("warn", message);
}

pub fn runtime_log_error(message: String) {
    runtime_log_push("error", message);
}

pub fn runtime_log_debug(message: String) {
    runtime_log_push("debug", message);
}
