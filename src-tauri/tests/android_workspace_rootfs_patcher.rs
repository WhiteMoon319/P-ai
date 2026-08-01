#[path = "../src/features/system/sandbox/android_rootfs/patcher.rs"]
mod android_workspace_rootfs_patcher;

use android_workspace_rootfs_patcher::*;
use std::fs;
use std::path::PathBuf;

fn temp_rootfs(name: &str) -> PathBuf {
    let base = std::env::temp_dir().join("pai-android-patcher-tests");
    let root = base.join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn patch_rootfs_should_be_idempotent() {
    let root = temp_rootfs("patch-idempotent");
    let etc = root.join("etc");
    fs::create_dir_all(&etc).unwrap();
    fs::write(etc.join("group"), "root:x:0:\n").unwrap();
    fs::write(etc.join("hosts"), "127.0.0.1 localhost\n").unwrap();
    fs::write(etc.join("hostname"), "localhost\n").unwrap();
    // patch_rootfs 会先校验 usr/bin/dash 可用，需要先放置。
    let usr_bin = root.join("usr").join("bin");
    fs::create_dir_all(&usr_bin).unwrap();
    fs::write(usr_bin.join("dash"), "dash placeholder").unwrap();

    android_workspace_rootfs_patcher::android_proot_patch_rootfs(&root).unwrap();
    let hosts_after_first = fs::read_to_string(etc.join("hosts")).unwrap();
    let group_after_first = fs::read_to_string(etc.join("group")).unwrap();
    let locale_after_first = fs::read_to_string(etc.join("default").join("locale")).unwrap();

    // 幂等：第二次 patch 不应改变已写入内容
    android_workspace_rootfs_patcher::android_proot_patch_rootfs(&root).unwrap();
    assert_eq!(fs::read_to_string(etc.join("hosts")).unwrap(), hosts_after_first);
    assert_eq!(fs::read_to_string(etc.join("group")).unwrap(), group_after_first);
    assert_eq!(fs::read_to_string(etc.join("default").join("locale")).unwrap(), locale_after_first);

    // DNS 配置会被写为外部 DNS
    let resolv = fs::read_to_string(etc.join("resolv.conf")).unwrap();
    assert!(resolv.contains("nameserver 1.1.1.1"));

    // tmp/var/tmp/root 目录就绪
    assert!(root.join("tmp").is_dir());
    assert!(root.join("var").join("tmp").is_dir());
    assert!(root.join("root").is_dir());
}

#[test]
fn patch_group_file_should_preserve_existing_ids() {
    let root = temp_rootfs("patch-group");
    let etc = root.join("etc");
    fs::create_dir_all(&etc).unwrap();
    fs::write(etc.join("group"), "root:x:0:\nusers:x:100:\n").unwrap();

    android_workspace_rootfs_patcher::android_proot_patch_group_file(&etc).unwrap();
    let text = fs::read_to_string(etc.join("group")).unwrap();
    assert!(text.contains("root:x:0:"));
    assert!(text.contains("users:x:100:"));
    // 重复 patch 不产生重复行
    android_workspace_rootfs_patcher::android_proot_patch_group_file(&etc).unwrap();
    let again = fs::read_to_string(etc.join("group")).unwrap();
    assert_eq!(text, again);
}

#[test]
fn patch_resolv_conf_should_replace_loopback_only_dns() {
    let root = temp_rootfs("patch-resolv");
    let etc = root.join("etc");
    fs::create_dir_all(&etc).unwrap();
    fs::write(etc.join("resolv.conf"), "nameserver 127.0.0.1\n").unwrap();

    android_workspace_rootfs_patcher::android_proot_patch_resolv_conf(&etc).unwrap();
    let text = fs::read_to_string(etc.join("resolv.conf")).unwrap();
    assert!(!text.contains("127.0.0.1"));
    assert!(text.contains("1.1.1.1"));
}

#[test]
fn ensure_rootfs_entrypoints_should_self_heal_missing_sh() {
    let root = temp_rootfs("entrypoints");
    let usr_bin = root.join("usr").join("bin");
    fs::create_dir_all(&usr_bin).unwrap();
    fs::write(usr_bin.join("dash"), "dash placeholder").unwrap();
    // 不创建 bin/sh，模拟入口损坏
    let bin_dir = root.join("bin");
    fs::create_dir_all(&bin_dir).unwrap();

    android_workspace_rootfs_patcher::android_proot_ensure_rootfs_entrypoints(&root).unwrap();
    let sh = bin_dir.join("sh");
    assert!(sh.is_file(), "/bin/sh should be self-healed from usr/bin/dash");
    let usr_sh = usr_bin.join("sh");
    assert!(usr_sh.is_file(), "/usr/bin/sh should also be self-healed from usr/bin/dash");
}

#[test]
fn symlink_repair_should_relativize_absolute_targets() {
    #[cfg(unix)]
    {
        let root = temp_rootfs("symlink-relativize");
        let bin_dir = root.join("bin");
        fs::create_dir_all(&bin_dir).unwrap();
        let usr_bin = root.join("usr").join("bin");
        fs::create_dir_all(&usr_bin).unwrap();
        // 创建绝对目标符号链接 bin/sh -> /usr/bin/dash
        std::os::unix::fs::symlink("/usr/bin/dash", bin_dir.join("sh")).unwrap();
        std::os::unix::fs::symlink("/usr/bin/dash", usr_bin.join("sh")).unwrap();

        android_workspace_rootfs_patcher::android_proot_repair_rootfs_symlink_if_needed(&root, "bin/sh").unwrap();
        let target = fs::read_link(bin_dir.join("sh")).unwrap();
        assert!(!target.is_absolute(), "repaired symlink should be relative: {target:?}");

        android_workspace_rootfs_patcher::android_proot_repair_rootfs_symlink_if_needed(&root, "usr/bin/sh").unwrap();
        let target2 = fs::read_link(usr_bin.join("sh")).unwrap();
        assert!(!target2.is_absolute(), "repaired symlink should be relative: {target2:?}");
    }
}

#[cfg(unix)]
#[test]
fn relative_symlink_target_should_resolve_cross_directory() {
    let root = std::path::Path::new("/runtime/linux");
    let relative = android_workspace_rootfs_patcher::android_proot_relative_rootfs_symlink_target(
        root,
        &root.join("bin").join("sh"),
        std::path::Path::new("/usr/bin/dash"),
    );
    assert_eq!(relative.as_deref(), Some("../usr/bin/dash"));
}
