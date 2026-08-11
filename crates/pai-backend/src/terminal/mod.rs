//! 终端输出格式化与命令分析（纯逻辑，无平台依赖）。

pub mod analyzer;
pub mod matcher;
pub mod output;

pub use analyzer::*;
pub use matcher::*;
pub use output::*;

use std::path::Path;

/// 终端路径比较归一化（从 src-tauri terminal/workspace.rs 迁入）。
pub fn normalize_terminal_path_for_compare(path: &Path) -> String {
    #[cfg(target_os = "windows")]
    {
        let text = path.to_string_lossy().to_string();
        if let Some(stripped) = text.strip_prefix("\\\\?\\") {
            return stripped.to_ascii_lowercase();
        }
        text.to_ascii_lowercase()
    }
    #[cfg(not(target_os = "windows"))]
    {
        path.to_string_lossy().to_string()
    }
}

/// 终端路径输入归一化（从 src-tauri storage_and_stt.rs 迁入的简化版）。
pub fn normalize_terminal_path_input_for_current_platform(raw: &str) -> String {
    let unquoted = raw.trim();
    let unquoted = if unquoted.len() >= 2 {
        let bytes = unquoted.as_bytes();
        if (bytes[0] == b'"' && bytes[unquoted.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[unquoted.len() - 1] == b'\'')
        {
            &unquoted[1..unquoted.len() - 1]
        } else {
            unquoted
        }
    } else {
        unquoted
    };
    if unquoted.is_empty() {
        return String::new();
    }
    let expanded = if unquoted == "~" {
        std::env::var("HOME").unwrap_or_default()
    } else if let Some(rest) = unquoted.strip_prefix("~/").or_else(|| unquoted.strip_prefix("~\\")) {
        format!("{}/{}", std::env::var("HOME").unwrap_or_default(), rest)
    } else {
        unquoted.to_string()
    };
    #[cfg(target_os = "windows")]
    {
        expanded.replace('/', "\\")
    }
    #[cfg(not(target_os = "windows"))]
    {
        expanded
    }
}
