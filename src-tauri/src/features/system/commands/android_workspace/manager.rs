use super::*;
use std::path::PathBuf;

pub(crate) fn android_sandbox_path_is_within(root: &std::path::Path, target: &std::path::Path) -> bool {
    path_is_within(root, target)
}

pub(crate) fn android_workspace_normalize_path_lexically(path: &std::path::Path) -> PathBuf {
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

pub(crate) fn android_workspace_existing_ancestor(path: &std::path::Path) -> Option<PathBuf> {
    let mut current = Some(path);
    while let Some(candidate) = current {
        if candidate.exists() {
            return Some(candidate.to_path_buf());
        }
        current = candidate.parent();
    }
    None
}

pub(crate) fn android_workspace_canonical_root(state: &AppState) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "android")]
    {
        let root = android_workspace_root(state)
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区失败: {err}"))?;
        return Ok(Some(root));
    }
}

pub(crate) fn android_workspace_canonical_root_if_ready(state: &AppState) -> Result<Option<PathBuf>, String> {
    #[cfg(target_os = "android")]
    {
        if !is_android_workspace_ready(state) {
            return Err(ANDROID_WORKSPACE_NOT_READY_MESSAGE.to_string());
        }
    }
    android_workspace_canonical_root(state)
}

pub(crate) fn android_workspace_ensure_tool_visible_path(
    root: &std::path::Path,
    target: &std::path::Path,
    allow_root: bool,
) -> Result<(), String> {
    let relative = target
        .strip_prefix(root)
        .map_err(|_| format!("Android 工作区无法解析沙盒相对路径：{}", target.display()))?;
    if !android_workspace_relative_path_is_tool_visible(relative, allow_root) {
        return Err("Android 工作区不允许访问内部系统路径。".to_string());
    }
    Ok(())
}

pub(crate) fn android_workspace_ensure_tool_visible_lexical_path(
    root: &std::path::Path,
    raw_root: &std::path::Path,
    target: &std::path::Path,
    allow_root: bool,
) -> Result<(), String> {
    if android_sandbox_path_is_within(root, target) {
        return android_workspace_ensure_tool_visible_path(root, target, allow_root);
    }
    if android_sandbox_path_is_within(raw_root, target) {
        return android_workspace_ensure_tool_visible_path(raw_root, target, allow_root);
    }
    Err(format!("Android 工作区不允许访问沙盒外路径：{}", target.display()))
}

pub(crate) fn android_workspace_raw_root_lexical(state: &AppState) -> PathBuf {
    android_workspace_normalize_path_lexically(&terminal_normalize_for_access_check(&android_workspace_root(state)))
}

pub(crate) fn android_workspace_resolve_existing_file_path(
    state: &AppState,
    raw_path: &str,
) -> Result<Option<PathBuf>, String> {
    let Some(root) = android_workspace_canonical_root(state)? else {
        return Ok(None);
    };
    let raw_root = android_workspace_raw_root_lexical(state);
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return Err("path 不能为空".to_string());
    }
    let raw = PathBuf::from(normalize_terminal_path_input_for_current_platform(trimmed));
    let raw = android_workspace_map_guest_path_to_host(&root, &raw);
    let joined = if raw.is_absolute() { raw } else { root.join(raw) };
    let normalized = terminal_normalize_for_access_check(&joined);
    let lexical = android_workspace_normalize_path_lexically(&normalized);
    android_workspace_ensure_tool_visible_lexical_path(&root, &raw_root, &lexical, false)?;
    let canonical = normalized
        .canonicalize()
        .map_err(|_| format!("文件不存在：{}", joined.display()))?;
    if !android_sandbox_path_is_within(&root, &canonical) {
        return Err(format!("Android 工作区不允许访问沙盒外路径：{}", joined.display()));
    }
    android_workspace_ensure_tool_visible_path(&root, &canonical, false)?;
    Ok(Some(canonical))
}

pub(crate) fn android_workspace_ensure_paths_within_sandbox(state: &AppState, paths: &[PathBuf]) -> Result<(), String> {
    let Some(root) = android_workspace_canonical_root(state)? else {
        return Ok(());
    };
    let raw_root = android_workspace_raw_root_lexical(state);
    for path in paths {
        let mapped = android_workspace_map_guest_path_to_host(&root, path);
        let joined = if mapped.is_absolute() { mapped.clone() } else { root.join(&mapped) };
        let normalized = terminal_normalize_for_access_check(&joined);
        let lexical = android_workspace_normalize_path_lexically(&normalized);
        android_workspace_ensure_tool_visible_lexical_path(&root, &raw_root, &lexical, false)?;
        let Some(existing) = android_workspace_existing_ancestor(&normalized) else {
            return Err(format!("Android 工作区无法解析目标路径：{}", path.display()));
        };
        let existing_canonical = existing
            .canonicalize()
            .map_err(|err| format!("解析 Android 工作区目标路径失败 ({}): {err}", existing.display()))?;
        if !android_sandbox_path_is_within(&root, &existing_canonical) {
            return Err(format!("Android 工作区不允许访问沙盒外路径：{}", path.display()));
        }
        android_workspace_ensure_tool_visible_path(&root, &existing_canonical, true)?;
    }
    Ok(())
}
