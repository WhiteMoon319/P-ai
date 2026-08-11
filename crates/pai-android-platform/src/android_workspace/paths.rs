pub const ANDROID_WORKSPACE_STATE_FILE: &str = "android_workspace_state.json";

pub fn android_workspace_runtime_base(root: &std::path::Path) -> std::path::PathBuf {
    root.parent()
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| root.to_path_buf())
        .join("runtime")
        .join("android-workspace")
        .join("default")
}

pub fn android_workspace_runtime_root(root: &std::path::Path) -> std::path::PathBuf {
    android_workspace_runtime_base(root).join("linux")
}

#[cfg(any(target_os = "android", test))]
pub fn android_workspace_tool_requires_linux_runtime(tool_name: &str, is_mcp_tool: bool) -> bool {
    is_mcp_tool || tool_name.trim() == "exec"
}

pub fn android_workspace_guest_path_suffix<'a>(path_text: &'a str, guest_prefix: &str) -> Option<&'a str> {
    if path_text == guest_prefix {
        return Some("");
    }
    path_text.strip_prefix(&format!("{guest_prefix}/"))
}

pub fn android_workspace_join_guest_suffix(host_base: &std::path::Path, suffix: &str) -> std::path::PathBuf {
    let mut out = host_base.to_path_buf();
    for part in suffix.split('/') {
        if !part.is_empty() {
            out.push(part);
        }
    }
    out
}

pub fn android_workspace_map_guest_path_to_host(
    root: &std::path::Path,
    raw: &std::path::Path,
) -> std::path::PathBuf {
    let path_text = raw.to_string_lossy().replace('\\', "/");
    for (guest_prefix, host_base) in [
        ("/root/.pai", root.to_path_buf()),
        ("/workspace", root.to_path_buf()),
    ] {
        if let Some(suffix) = android_workspace_guest_path_suffix(&path_text, guest_prefix) {
            return android_workspace_join_guest_suffix(&host_base, suffix);
        }
    }
    raw.to_path_buf()
}

pub fn android_workspace_root_name_is_reserved_for_file_tools(name: &str) -> bool {
    matches!(
        name,
        ".pai"
            | "runtime"
            | "tmp"
            | "mcp"
            | "private-organization"
            | "avatars"
            | "media"
            | "app_config.toml"
            | "app_data.json"
            | ANDROID_WORKSPACE_STATE_FILE
    )
}

pub fn android_workspace_relative_path_is_tool_visible(
    relative: &std::path::Path,
    allow_root: bool,
) -> bool {
    let mut index = 0usize;
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return false;
        };
        let name = part.to_string_lossy();
        if name.is_empty() {
            return false;
        }
        if index == 0 && android_workspace_root_name_is_reserved_for_file_tools(name.as_ref()) {
            return false;
        }
        index = index.saturating_add(1);
    }
    index > 0 || allow_root
}

#[cfg(test)]
pub fn android_workspace_normalize_path_input(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(not(test))]
pub fn android_workspace_normalize_path_input(raw: &str) -> String {
    // 简化实现（从 src-tauri storage_and_stt.rs normalize 链迁入）：
    // 去引号 + home 展开 + windows 反斜杠归一化。
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

pub fn android_workspace_clean_relative_input(raw: &str) -> Result<std::path::PathBuf, String> {
    let normalized = android_workspace_normalize_path_input(raw.trim());
    if normalized.trim().is_empty() {
        return Ok(std::path::PathBuf::new());
    }
    let raw_path = std::path::PathBuf::from(normalized);
    if raw_path.is_absolute() {
        return Err("文件管理只允许访问 Android 沙盒内的相对路径。".to_string());
    }
    let mut out = std::path::PathBuf::new();
    for component in raw_path.components() {
        match component {
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                return Err("文件管理路径不能包含上级目录跳转。".to_string());
            }
            std::path::Component::RootDir | std::path::Component::Prefix(_) => {
                return Err("文件管理只允许访问 Android 沙盒内的相对路径。".to_string());
            }
        }
    }
    Ok(out)
}

pub fn android_workspace_root_name_is_reserved_for_file_manager(name: &str) -> bool {
    matches!(
        name,
        "runtime"
            | "tmp"
            | "mcp"
            | "skills"
            | "private-organization"
            | "avatars"
            | "media"
            | "app_config.toml"
            | "app_data.json"
            | ANDROID_WORKSPACE_STATE_FILE
    )
}

pub fn android_workspace_relative_path_is_user_visible(
    relative: &std::path::Path,
    allow_root: bool,
) -> bool {
    let mut index = 0usize;
    for component in relative.components() {
        let std::path::Component::Normal(part) = component else {
            return false;
        };
        let name = part.to_string_lossy();
        if name.is_empty() || name.starts_with('.') {
            return false;
        }
        if index == 0 && android_workspace_root_name_is_reserved_for_file_manager(name.as_ref()) {
            return false;
        }
        index = index.saturating_add(1);
    }
    index > 0 || allow_root
}

pub fn android_workspace_glob_to_regex(pattern: &str) -> Result<regex::Regex, String> {
    if pattern.trim().is_empty() {
        return Err("glob pattern 不能为空".to_string());
    }
    let normalized = pattern.replace('\\', "/");
    let mut out = String::from("^");
    let mut chars = normalized.chars().peekable();
    while let Some(ch) = chars.next() {
        match ch {
            '*' => {
                if chars.peek() == Some(&'*') {
                    let _ = chars.next();
                    if chars.peek() == Some(&'/') {
                        let _ = chars.next();
                        out.push_str("(?:.*/)?");
                    } else {
                        out.push_str(".*");
                    }
                } else {
                    out.push_str("[^/]*");
                }
            }
            '?' => out.push_str("[^/]"),
            '.' | '+' | '(' | ')' | '|' | '^' | '$' | '{' | '}' | '[' | ']' | '\\' => {
                out.push('\\');
                out.push(ch);
            }
            '/' => out.push('/'),
            other => out.push(other),
        }
    }
    out.push('$');
    regex::Regex::new(&out).map_err(|err| format!("glob pattern 无效：{err}"))
}

pub fn android_workspace_relative_matches_glob(relative_path: &str, pattern: &str) -> Result<bool, String> {
    Ok(android_workspace_glob_to_regex(pattern)?.is_match(relative_path))
}
