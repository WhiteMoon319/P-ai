// Android 平台的应用内更新 stub：移动端不支持 GitHub 自更新，
// 保留与 updater.rs 相同的对外签名，命令返回友好错误或空状态。

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GithubUpdateInfo {
    current_version: String,
    latest_version: String,
    has_update: bool,
    release_url: String,
    update_source: String,
    access_mode: String,
    release_notes: String,
    published_at: Option<String>,
    runtime_kind: String,
    can_force_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct GithubUpdateState {
    stage: String,
    current_version: String,
    latest_version: String,
    runtime_kind: String,
    has_prepared_update: bool,
    has_visible_update: bool,
    release_notes: String,
    release_url: String,
    published_at: Option<String>,
    prepared_at: Option<String>,
    last_checked_at: Option<String>,
    last_error: Option<String>,
    skipped_version: String,
}

const ANDROID_UPDATE_UNSUPPORTED: &str = "Android 平台暂不支持应用内更新";

/// Android 更新检查仓库：Android 移植版独立发布仓库（APK 在 GitHub Release 发布）。
const ANDROID_UPDATE_REPO: &str = "WhiteMoon319/P-ai";
const ANDROID_UPDATE_RELEASE_API: &str =
    "https://api.github.com/repos/WhiteMoon319/P-ai/releases/latest";
const ANDROID_UPDATE_CHANGELOG_RAW: &str =
    "https://raw.githubusercontent.com/WhiteMoon319/P-ai/main/docs/changelog/latest.md";

fn normalize_android_release_version(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}


fn sync_update_state_from_skip_version(_app: &NativeAppHandle, _version: &str) {}

/// 最近一次 Android 更新检查结果（内存态，供 get_github_update_state 读取）。
static ANDROID_LAST_CHECK: std::sync::OnceLock<GithubUpdateInfo> = std::sync::OnceLock::new();



fn cleanup_portable_update_temp_artifacts_for_current_runtime() -> Result<(), String> {
    Ok(())
}

fn start_github_auto_update_worker(_app: NativeAppHandle) {}




fn maybe_run_portable_update_helper_from_args() -> Result<bool, String> {
    Ok(false)
}
