// 状态 v2 契约测试：验证 v1 JSON（缺少 llmWorkspaceRoot/runtimeRoot 字段）仍可反序列化，
// 且新构造的状态写入 version=2 并携带两个新路径字段。
use std::path::Path;

const ANDROID_WORKSPACE_ROOTFS_CONTENT_LENGTH: u64 = 29_865_086;

fn android_workspace_status_paths(root: &Path) -> (String, String) {
    let runtime_base = root
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| root.to_path_buf())
        .join("runtime")
        .join("android-workspace")
        .join("default");
    (
        root.to_string_lossy().to_string(),
        runtime_base.join("linux").to_string_lossy().to_string(),
    )
}

fn shell_workspace_display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn now_iso() -> String {
    "2026-08-01T00:00:00+08:00".to_string()
}

#[path = "../src/features/system/commands/android_workspace/types.rs"]
mod android_workspace_types;

use android_workspace_types::*;

#[test]
fn v1_status_json_should_deserialize_with_defaults() {
    // 模拟旧版本写入的状态文件：只有 version=1 的字段，没有 v2 新字段。
    let v1 = r#"{
        "state": "ready",
        "rootPath": "/data/data/app/llm-workspace",
        "initializedAt": "2026-07-01T00:00:00+08:00",
        "updatedAt": "2026-07-01T00:00:00+08:00",
        "lastError": null,
        "version": 1,
        "runtimeVersion": "ubuntu-base-24.04.3-arm64",
        "downloadBytes": 29865086,
        "downloadTotalBytes": 29865086,
        "downloadStage": null
    }"#;
    let status: AndroidWorkspaceStatus = serde_json::from_str(v1).expect("v1 status should deserialize");
    assert_eq!(status.version, 1);
    assert_eq!(status.llm_workspace_root, "", "缺少字段应回退为空字符串，由 normalize 回填");
    assert_eq!(status.runtime_root, "");
    assert!(matches!(status.state, AndroidWorkspaceStateKind::Ready));
}

#[test]
fn new_status_should_be_version_2_with_path_fields() {
    let root = Path::new("/data/data/app/llm-workspace");
    let status = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::NotDownloaded, root);
    assert_eq!(status.version, 2);
    assert_eq!(status.llm_workspace_root, "/data/data/app/llm-workspace");
    let expected_runtime = Path::new("/data/data/app")
        .join("runtime")
        .join("android-workspace")
        .join("default")
        .join("linux")
        .to_string_lossy()
        .to_string();
    assert_eq!(status.runtime_root, expected_runtime);
    assert_eq!(status.root_path, status.llm_workspace_root);
}

#[test]
fn v2_status_should_round_trip_through_json() {
    let root = Path::new("/data/data/app/llm-workspace");
    let status = AndroidWorkspaceStatus::new(AndroidWorkspaceStateKind::Downloading, root);
    let body = serde_json::to_string(&status).expect("serialize");
    let parsed: AndroidWorkspaceStatus = serde_json::from_str(&body).expect("deserialize round trip");
    assert_eq!(parsed.version, 2);
    assert_eq!(parsed.llm_workspace_root, status.llm_workspace_root);
    assert_eq!(parsed.runtime_root, status.runtime_root);
    assert!(matches!(parsed.state, AndroidWorkspaceStateKind::Downloading));
}
