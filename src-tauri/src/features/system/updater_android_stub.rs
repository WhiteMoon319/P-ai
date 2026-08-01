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

#[tauri::command]
async fn fetch_project_changelog_markdown() -> Result<String, String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

fn sync_update_state_from_skip_version(_app: &AppHandle, _version: &str) {}

#[tauri::command]
fn get_github_update_state(_app: AppHandle) -> Result<GithubUpdateState, String> {
    Ok(GithubUpdateState {
        stage: "idle".to_string(),
        current_version: env!("CARGO_PKG_VERSION").to_string(),
        latest_version: env!("CARGO_PKG_VERSION").to_string(),
        runtime_kind: "android".to_string(),
        ..GithubUpdateState::default()
    })
}

#[tauri::command]
async fn check_github_update(
    _app: AppHandle,
    _update_method: Option<String>,
    _respect_cooldown: Option<bool>,
) -> Result<GithubUpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    Ok(GithubUpdateInfo {
        current_version: current_version.clone(),
        latest_version: current_version,
        has_update: false,
        release_url: String::new(),
        update_source: "unsupported".to_string(),
        access_mode: "direct".to_string(),
        release_notes: String::new(),
        published_at: None,
        runtime_kind: "android".to_string(),
        can_force_update: false,
    })
}

fn cleanup_portable_update_temp_artifacts_for_current_runtime() -> Result<(), String> {
    Ok(())
}

fn start_github_auto_update_worker(_app: AppHandle) {}

#[tauri::command]
async fn start_github_update(
    _app: AppHandle,
    _force: bool,
    _update_method: Option<String>,
) -> Result<(), String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

#[tauri::command]
async fn cancel_github_update() -> Result<(), String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

#[tauri::command]
async fn apply_prepared_github_update(_app: AppHandle) -> Result<(), String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

fn maybe_run_portable_update_helper_from_args() -> Result<bool, String> {
    Ok(false)
}
