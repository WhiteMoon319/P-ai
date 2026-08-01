use std::path::{Path, PathBuf};

pub(crate) fn android_workspace_rootfs_resolve_entry_path(
    root: &Path,
    path: &Path,
) -> Result<PathBuf, String> {
    let mut target = root.to_path_buf();
    let mut has_component = false;
    for component in path.components() {
        match component {
            std::path::Component::Normal(part) => {
                has_component = true;
                target.push(part);
            }
            std::path::Component::CurDir => {}
            std::path::Component::Prefix(_) | std::path::Component::RootDir | std::path::Component::ParentDir => {
                return Err(format!("Android Linux 运行环境包包含非法路径：{}", path.display()));
            }
        }
    }
    if !has_component {
        return Err("Android Linux 运行环境包包含空路径".to_string());
    }
    Ok(target)
}

pub(crate) fn android_workspace_rootfs_normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            std::path::Component::RootDir => normalized.push(std::path::MAIN_SEPARATOR.to_string()),
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::Normal(part) => normalized.push(part),
        }
    }
    normalized
}

pub(crate) fn android_workspace_rootfs_resolve_symlink_target(
    root: &Path,
    link_path: &Path,
    link_target: &Path,
) -> Option<PathBuf> {
    let root = android_workspace_rootfs_normalize_path(root);
    let link_path = android_workspace_rootfs_normalize_path(link_path);
    let resolved = if link_target.is_absolute() {
        let mut target = root.clone();
        for component in link_target.components() {
            match component {
                std::path::Component::Normal(part) => target.push(part),
                std::path::Component::CurDir | std::path::Component::RootDir => {}
                std::path::Component::ParentDir => {
                    target.pop();
                }
                std::path::Component::Prefix(_) => return None,
            }
        }
        android_workspace_rootfs_normalize_path(&target)
    } else {
        let parent = link_path.parent()?;
        android_workspace_rootfs_normalize_path(&parent.join(link_target))
    };
    resolved.starts_with(&root).then_some(resolved)
}

pub(crate) fn android_workspace_rootfs_relative_symlink_target(
    root: &Path,
    link_path: &Path,
    link_target: &Path,
) -> Option<String> {
    let root = android_workspace_rootfs_normalize_path(root);
    let link_path = android_workspace_rootfs_normalize_path(link_path);
    let parent = link_path.parent()?;
    let resolved = android_workspace_rootfs_resolve_symlink_target(&root, &link_path, link_target)?;
    let from = parent.strip_prefix(&root).ok()?;
    let to = resolved.strip_prefix(&root).ok()?;
    let from_parts = from
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let to_parts = to
        .components()
        .filter_map(|component| match component {
            std::path::Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>();
    let mut common = 0usize;
    while common < from_parts.len() && common < to_parts.len() && from_parts[common] == to_parts[common] {
        common += 1;
    }
    let mut relative = Vec::<String>::new();
    for _ in common..from_parts.len() {
        relative.push("..".to_string());
    }
    relative.extend(to_parts.into_iter().skip(common));
    Some(if relative.is_empty() { ".".to_string() } else { relative.join("/") })
}
