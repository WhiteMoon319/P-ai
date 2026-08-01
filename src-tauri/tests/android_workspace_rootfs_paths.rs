#[path = "../src/features/system/commands/android_workspace/rootfs_paths.rs"]
mod android_workspace_rootfs_paths;

use android_workspace_rootfs_paths::*;

#[test]
fn entry_path_should_join_relative_paths_under_root() {
    let root = std::path::Path::new("/runtime/linux");
    let resolved = android_workspace_rootfs_resolve_entry_path(root, std::path::Path::new("usr/bin/dash")).unwrap();
    assert_eq!(resolved, root.join("usr").join("bin").join("dash"));
}

#[test]
fn entry_path_should_reject_parent_traversal() {
    let root = std::path::Path::new("/runtime/linux");
    for raw in ["../escape", "usr/../../escape", "/absolute/path"] {
        let path = std::path::Path::new(raw);
        assert!(
            android_workspace_rootfs_resolve_entry_path(root, path).is_err(),
            "entry path should be rejected: {raw}"
        );
    }
}

#[test]
fn entry_path_should_reject_empty_path() {
    let root = std::path::Path::new("/runtime/linux");
    assert!(android_workspace_rootfs_resolve_entry_path(root, std::path::Path::new(".")).is_err());
}

#[test]
fn normalize_path_should_collapse_dots_and_parents() {
    let normalized = android_workspace_rootfs_normalize_path(std::path::Path::new("/a/./b/../c"));
    assert_eq!(normalized, std::path::Path::new("/a/c"));
}

#[cfg(unix)]
#[test]
fn absolute_symlink_should_resolve_inside_rootfs() {
    let root = std::path::Path::new("/runtime/linux");
    let resolved = android_workspace_rootfs_resolve_symlink_target(
        root,
        std::path::Path::new("/runtime/linux/bin/sh"),
        std::path::Path::new("/bin/dash"),
    );
    assert_eq!(resolved, Some(root.join("bin").join("dash")));
}

#[cfg(unix)]
#[test]
fn absolute_symlink_should_resolve_under_rootfs_root() {
    let root = std::path::Path::new("/runtime/linux");
    let resolved = android_workspace_rootfs_resolve_symlink_target(
        root,
        std::path::Path::new("/runtime/linux/bin/sh"),
        std::path::Path::new("/etc/passwd"),
    );
    // 绝对链接目标按 rootfs 根解析：/etc/passwd -> rootfs/etc/passwd，仍在 rootfs 内。
    assert_eq!(resolved, Some(root.join("etc").join("passwd")));
}

#[cfg(unix)]
#[test]
fn absolute_symlink_with_parent_traversal_escaping_rootfs_should_be_rejected() {
    let root = std::path::Path::new("/runtime/linux");
    // 目标带 .. 且超出 rootfs 根时应拒绝。
    let resolved = android_workspace_rootfs_resolve_symlink_target(
        root,
        std::path::Path::new("/runtime/linux/usr/bin/foo"),
        std::path::Path::new("/../../etc/passwd"),
    );
    assert!(resolved.is_none());
}

#[test]
fn relative_symlink_should_resolve_from_link_directory() {
    let root = std::path::Path::new("/runtime/linux");
    let link = root.join("usr").join("bin").join("sh");
    let resolved = android_workspace_rootfs_resolve_symlink_target(
        &root,
        &link,
        std::path::Path::new("dash"),
    );
    assert_eq!(resolved, Some(root.join("usr").join("bin").join("dash")));
}

#[cfg(unix)]
#[test]
fn relative_symlink_should_resolve_from_link_directory_absolute_target() {
    let root = std::path::Path::new("/runtime/linux");
    let relative = android_workspace_rootfs_relative_symlink_target(
        &root,
        &root.join("usr").join("bin").join("sh"),
        std::path::Path::new("/usr/bin/dash"),
    );
    assert_eq!(relative.as_deref(), Some("dash"));
}

#[cfg(unix)]
#[test]
fn relative_symlink_across_directories_should_use_parent_traversal() {
    let root = std::path::Path::new("/runtime/linux");
    let relative = android_workspace_rootfs_relative_symlink_target(
        &root,
        &root.join("bin").join("sh"),
        std::path::Path::new("/usr/bin/dash"),
    );
    assert_eq!(relative.as_deref(), Some("../usr/bin/dash"));
}

#[test]
fn relative_symlink_within_same_directory_should_be_dot() {
    let root = std::path::Path::new("/runtime/linux");
    let link = root.join("usr").join("bin").join("sh");
    let relative = android_workspace_rootfs_relative_symlink_target(
        &root,
        &link,
        std::path::Path::new("sh"),
    );
    assert_eq!(relative.as_deref(), Some("sh"));
}

#[test]
fn relative_symlink_pointing_to_own_directory_should_be_dot() {
    let root = std::path::Path::new("/runtime/linux");
    let link = root.join("usr").join("bin").join("sh");
    let relative = android_workspace_rootfs_relative_symlink_target(
        &root,
        &link,
        std::path::Path::new("."),
    );
    assert_eq!(relative.as_deref(), Some("."));
}
