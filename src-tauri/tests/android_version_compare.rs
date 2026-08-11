// Android 版本比较纯逻辑测试（integration test，只 include 无依赖模块，
// 绕开 Windows 本机 lib test 因 tauri 依赖无法编译的限制）。
#[path = "../src/features/system/version_compare.rs"]
mod version_compare;

use version_compare::*;

#[test]
fn version_parse_and_compare_basic() {
    assert!(android_version_is_newer("1.2.4", "1.2.3"));
    assert!(!android_version_is_newer("1.2.3", "1.2.3"));
    assert!(android_version_is_newer("1.3.0", "1.2.9"));
    assert!(android_version_is_newer("2.0.0", "1.99.99"));
    assert!(!android_version_is_newer("1.2.3", "1.2.4"));
}

#[test]
fn version_compare_handles_v_prefix() {
    assert!(android_version_is_newer("v1.2.4", "1.2.3"));
    assert!(android_version_is_newer("1.2.4", "v1.2.3"));
    assert!(!android_version_is_newer("v1.2.3", "1.2.3"));
    assert!(android_version_is_newer("V1.2.4", "v1.2.3"));
}

#[test]
fn version_compare_handles_prerelease() {
    // 正式版比同号预发布新
    assert!(android_version_is_newer("1.2.3", "1.2.3-alpha.1"));
    assert!(android_version_is_newer("1.2.3", "1.2.3-pre.2"));
    // 预发布之间按字符串序
    assert!(android_version_is_newer("1.2.3-beta", "1.2.3-alpha"));
    assert!(!android_version_is_newer("1.2.3-alpha.2", "1.2.3-alpha.2"));
    // 更高正式版本号压制预发布
    assert!(android_version_is_newer("1.3.0-alpha.1", "1.2.9"));
    assert!(!android_version_is_newer("1.2.4-alpha.1", "1.2.4"));
}

#[test]
fn version_compare_same_version_not_newer() {
    assert!(!android_version_is_newer("0.57.0", "0.57.0"));
    assert!(!android_version_is_newer("v0.57.0", "0.57.0"));
    assert!(!android_version_is_newer("", "0.57.0"));
}

#[test]
fn parse_android_version_normalizes_parts() {
    let parts = parse_android_version("v1.2.3-alpha.1").expect("parse");
    assert_eq!(parts.major, 1);
    assert_eq!(parts.minor, 2);
    assert_eq!(parts.patch, 3);
    assert_eq!(parts.prerelease.as_deref(), Some("alpha.1"));

    let parts = parse_android_version("1.2").expect("parse short");
    assert_eq!((parts.major, parts.minor, parts.patch), (1, 2, 0));
    assert!(parts.prerelease.is_none());
}
