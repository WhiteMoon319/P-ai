// Android rootfs 路径安全测试：绝对路径 / `..` / symlink 逃逸都必须被拒绝或限制在 root 内。
#[path = "../src/features/system/commands/android_workspace/rootfs_paths.rs"]
mod rootfs_paths;

use rootfs_paths::*;

#[test]
fn resolve_entry_path_rejects_absolute_and_parent_traversal() {
    let root = std::path::Path::new("/tmp/rootfs").to_path_buf();
    assert!(android_workspace_rootfs_resolve_entry_path(&root, std::path::Path::new("/etc/passwd")).is_err());
    assert!(android_workspace_rootfs_resolve_entry_path(&root, std::path::Path::new("../outside")).is_err());
    assert!(android_workspace_rootfs_resolve_entry_path(&root, std::path::Path::new("a/../../outside")).is_err());
    assert!(android_workspace_rootfs_resolve_entry_path(&root, std::path::Path::new("")).is_err());
    // 正常相对路径可解析
    let ok = android_workspace_rootfs_resolve_entry_path(&root, std::path::Path::new("usr/bin/dash")).expect("ok");
    assert!(ok.starts_with(&root));
}

#[test]
fn resolve_symlink_target_never_escapes_root() {
    let root = std::path::Path::new("/tmp/rootfs").to_path_buf();
    // 绝对 symlink 目标会被重新锚定到 root 内（Windows 上 /usr 前缀无法锚定到
    // 非盘根 root 时按拒绝处理；Unix 上应锚定到 root 内）
    let target = android_workspace_rootfs_resolve_symlink_target(
        &root,
        &root.join("bin/sh"),
        std::path::Path::new("/usr/bin/dash"),
    );
    match target {
        Some(t) => assert!(t.starts_with(&root), "target must stay inside root: {t:?}"),
        None => { /* 平台差异：Windows 上无法把 /usr 锚定到 root 时按拒绝处理 */ }
    }

    // `..` 逃逸目标解析后不在 root 内则返回 None
    let escaped = android_workspace_rootfs_resolve_symlink_target(
        &root,
        &root.join("bin/sh"),
        std::path::Path::new("../../../../etc/passwd"),
    );
    assert!(escaped.is_none(), "escape must be rejected");

    // 相对链接在 root 内解析（link_path 必须带 root 前缀，与实际调用一致）
    let rel = android_workspace_rootfs_resolve_symlink_target(
        &root,
        &root.join("usr/bin/sh"),
        std::path::Path::new("../bin/dash"),
    );
    let rel = rel.expect("relative resolved");
    assert!(rel.starts_with(&root));
    assert_eq!(rel, root.join("usr/bin/dash"));
}

#[test]
fn normalize_path_collapses_parent_components() {
    assert_eq!(
        android_workspace_rootfs_normalize_path(std::path::Path::new("a/b/../c")),
        std::path::PathBuf::from("a/c"),
    );
}
