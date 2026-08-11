//! 终端输出格式化与命令分析（纯逻辑，无平台依赖）。

pub mod analyzer;
pub mod guards;
pub mod matcher;
pub mod output;

pub use analyzer::*;
pub use guards::*;
pub use matcher::*;
pub use output::*;

use std::path::{Path, PathBuf};

/// 路径是否在 base 内（从 src-tauri terminal/workspace.rs 迁入）。
pub fn path_is_within(base: &Path, target: &Path) -> bool {
    let base_norm = normalize_terminal_path_for_compare(base);
    let target_norm = normalize_terminal_path_for_compare(target);
    let separator = std::path::MAIN_SEPARATOR.to_string();
    let base_prefix = if base_norm.ends_with(&separator) {
        base_norm.clone()
    } else {
        format!("{base_norm}{separator}")
    };
    target_norm == base_norm || target_norm.strip_prefix(&base_prefix).is_some()
}

/// 路径词法归一化（从 src-tauri android_workspace/manager.rs 迁入）。
pub fn android_workspace_normalize_path_lexically(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(component.as_os_str()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                let _ = normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

/// 访问检查路径归一化（从 src-tauri terminal/workspace.rs 迁入）。
pub fn terminal_normalize_for_access_check(path: &Path) -> PathBuf {
    if let Ok(canonical) = path.canonicalize() {
        return canonical;
    }
    let mut missing = Vec::<std::ffi::OsString>::new();
    let mut cursor = path;
    loop {
        if let Ok(canonical) = cursor.canonicalize() {
            return missing.into_iter().rev().fold(canonical, |base, name| base.join(name));
        }
        let Some(name) = cursor.file_name() else { break };
        missing.push(name.to_os_string());
        let Some(parent) = cursor.parent() else { break };
        cursor = parent;
    }
    path.to_path_buf()
}

/// Android 沙盒路径是否在根内（从 src-tauri android_workspace/manager.rs 迁入）。
pub fn android_sandbox_path_is_within(root: &Path, target: &Path) -> bool {
    path_is_within(root, target)
}

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
