use super::*;
use std::{fs, path::PathBuf};
use uuid::Uuid;

pub(crate) fn android_workspace_sanitize_file_name(raw: &str) -> String {
    let name = std::path::Path::new(raw.trim())
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("imported-file")
        .trim();
    let mut out = String::with_capacity(name.len().max(1));
    for ch in name.chars() {
        if ch == '/' || ch == '\\' || ch == ':' || ch == '*' || ch == '?' || ch == '"' || ch == '<' || ch == '>' || ch == '|' || ch.is_control() {
            out.push('_');
        } else {
            out.push(ch);
        }
    }
    let trimmed = out.trim_matches(&['.', ' ', '_'][..]).to_string();
    if trimmed.is_empty() {
        "imported-file".to_string()
    } else {
        trimmed
    }
}

pub(crate) fn android_workspace_unique_sibling_path(candidate: &std::path::Path) -> PathBuf {
    if !candidate.exists() {
        return candidate.to_path_buf();
    }
    let parent = candidate.parent().unwrap_or_else(|| std::path::Path::new(""));
    let file_name = candidate
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("imported-file");
    let path = std::path::Path::new(file_name);
    let stem = path.file_stem().and_then(|value| value.to_str()).unwrap_or("imported-file");
    let ext = path.extension().and_then(|value| value.to_str()).unwrap_or("");
    for index in 1..1000 {
        let next_name = if ext.is_empty() {
            format!("{stem} ({index})")
        } else {
            format!("{stem} ({index}).{ext}")
        };
        let next = parent.join(next_name);
        if !next.exists() {
            return next;
        }
    }
    parent.join(format!("{}-{}", Uuid::new_v4(), file_name))
}

pub(crate) fn android_workspace_unique_import_path(imports_dir: &std::path::Path, file_name: &str) -> PathBuf {
    android_workspace_unique_sibling_path(&imports_dir.join(file_name))
}

pub(crate) fn android_workspace_resolve_import_target_path(
    root: &std::path::Path,
    file_name: &str,
    target_path: Option<&str>,
) -> Result<PathBuf, String> {
    let safe_name = android_workspace_sanitize_file_name(file_name);
    let trimmed = target_path.map(str::trim).unwrap_or_default();
    if trimmed.is_empty() {
        let imports_dir = root.join("imports");
        fs::create_dir_all(&imports_dir)
            .map_err(|err| format!("创建 Android 工作区导入目录失败 ({}): {err}", imports_dir.display()))?;
        return Ok(android_workspace_unique_import_path(&imports_dir, &safe_name));
    }
    let normalized = normalize_terminal_path_input_for_current_platform(trimmed);
    let raw = PathBuf::from(normalized);
    if raw.is_absolute() {
        return Err("导入路径必须是 Android 沙盒内的相对路径。".to_string());
    }
    let target = root.join(raw);
    if target.file_name().is_none() {
        return Err("导入路径必须包含文件名。".to_string());
    }
    Ok(target)
}

pub(crate) fn android_workspace_relative_display(root: &std::path::Path, path: &std::path::Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

pub(crate) fn android_workspace_ensure_user_file_manager_path(
    root: &std::path::Path,
    target: &std::path::Path,
    allow_root: bool,
) -> Result<(), String> {
    let normalized = terminal_normalize_for_access_check(target);
    let lexical = android_workspace_normalize_path_lexically(&normalized);
    if !android_sandbox_path_is_within(root, &lexical) {
        return Err(format!("文件管理不允许访问 Android 沙盒外路径：{}", target.display()));
    }
    let relative = lexical
        .strip_prefix(root)
        .map_err(|_| format!("文件管理无法解析沙盒相对路径：{}", target.display()))?;
    if !android_workspace_relative_path_is_user_visible(relative, allow_root) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    let Some(existing) = android_workspace_existing_ancestor(&normalized) else {
        return Err(format!("文件管理无法解析目标路径：{}", target.display()));
    };
    let existing_canonical = existing
        .canonicalize()
        .map_err(|err| format!("解析 Android 工作区目标路径失败 ({}): {err}", existing.display()))?;
    if !android_sandbox_path_is_within(root, &existing_canonical) {
        return Err(format!("文件管理不允许访问 Android 沙盒外路径：{}", target.display()));
    }
    let existing_relative = existing_canonical
        .strip_prefix(root)
        .map_err(|_| format!("文件管理无法解析沙盒相对路径：{}", existing.display()))?;
    if !android_workspace_relative_path_is_user_visible(existing_relative, true) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    Ok(())
}

pub(crate) fn android_workspace_resolve_file_manager_existing_path(
    state: &AppState,
    raw_path: &str,
    allow_root: bool,
) -> Result<PathBuf, String> {
    let Some(root) = android_workspace_canonical_root(state)? else {
        return Err("Android 工作区文件管理仅在 Android 端可用。".to_string());
    };
    let relative = android_workspace_clean_relative_input(raw_path)?;
    if !android_workspace_relative_path_is_user_visible(&relative, allow_root) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    let target = root.join(&relative);
    android_workspace_ensure_user_file_manager_path(&root, &target, allow_root)?;
    let canonical = target
        .canonicalize()
        .map_err(|_| format!("文件不存在：{}", target.display()))?;
    android_workspace_ensure_user_file_manager_path(&root, &canonical, allow_root)?;
    Ok(canonical)
}

pub(crate) fn android_workspace_resolve_file_manager_target_path(
    state: &AppState,
    raw_path: &str,
    allow_root: bool,
) -> Result<PathBuf, String> {
    let Some(root) = android_workspace_canonical_root(state)? else {
        return Err("Android 工作区文件管理仅在 Android 端可用。".to_string());
    };
    let relative = android_workspace_clean_relative_input(raw_path)?;
    if !android_workspace_relative_path_is_user_visible(&relative, allow_root) {
        return Err("文件管理不允许访问系统目录。".to_string());
    }
    let target = root.join(&relative);
    android_workspace_ensure_user_file_manager_path(&root, &target, allow_root)?;
    Ok(target)
}

pub(crate) fn android_workspace_mime_from_path(path: &std::path::Path) -> String {
    fs::read(path)
        .ok()
        .and_then(|bytes| infer::get(&bytes).map(|kind| kind.mime_type().to_string()))
        .unwrap_or_else(|| "application/octet-stream".to_string())
}

pub(crate) fn android_workspace_file_entry(root: &std::path::Path, path: PathBuf) -> Option<AndroidWorkspaceFileEntry> {
    let metadata = fs::symlink_metadata(&path).ok()?;
    let file_type = metadata.file_type();
    if file_type.is_symlink() || (!file_type.is_file() && !file_type.is_dir()) {
        return None;
    }
    android_workspace_ensure_user_file_manager_path(root, &path, false).ok()?;
    let name = path.file_name()?.to_str()?.to_string();
    let relative_path = android_workspace_relative_display(root, &path);
    let kind = if file_type.is_dir() { "directory" } else { "file" }.to_string();
    let bytes = if file_type.is_file() { Some(metadata.len()) } else { None };
    Some(AndroidWorkspaceFileEntry {
        name,
        path: relative_path,
        kind,
        bytes,
    })
}
