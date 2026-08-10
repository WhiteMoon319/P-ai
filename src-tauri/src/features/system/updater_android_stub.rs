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

#[cfg_attr(not(target_os = "android"), tauri::command)]
async fn fetch_project_changelog_markdown() -> Result<String, String> {
    let client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15));
    let client = android_workspace_apply_static_webpki_roots(client_builder)?
        .build()
        .map_err(|err| format!("构建更新日志请求客户端失败: {err}"))?;
    let resp = client
        .get(ANDROID_UPDATE_CHANGELOG_RAW)
        .header("User-Agent", "P-ai-Android-Updater")
        .send()
        .await
        .map_err(|err| format!("请求更新日志失败: {err}"))?;
    if !resp.status().is_success() {
        return Err(format!("更新日志请求失败: HTTP {}", resp.status()));
    }
    resp.text()
        .await
        .map_err(|err| format!("读取更新日志失败: {err}"))
}

fn sync_update_state_from_skip_version(_app: &AppHandle, _version: &str) {}

/// 最近一次 Android 更新检查结果（内存态，供 get_github_update_state 读取）。
static ANDROID_LAST_CHECK: std::sync::OnceLock<GithubUpdateInfo> = std::sync::OnceLock::new();

#[cfg(not(target_os = "android"))]
#[tauri::command]
fn get_github_update_state(_app: AppHandle) -> Result<GithubUpdateState, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let last = ANDROID_LAST_CHECK.get();
    let has_update = last.as_ref().map(|r| r.has_update).unwrap_or(false);
    let latest_version = last
        .as_ref()
        .map(|r| r.latest_version.clone())
        .unwrap_or_else(|| current_version.clone());
    Ok(GithubUpdateState {
        stage: "idle".to_string(),
        current_version,
        latest_version,
        runtime_kind: "android".to_string(),
        has_prepared_update: false,
        has_visible_update: has_update,
        release_notes: last
            .as_ref()
            .map(|r| r.release_notes.clone())
            .unwrap_or_default(),
        release_url: last.as_ref().map(|r| r.release_url.clone()).unwrap_or_default(),
        published_at: last.as_ref().and_then(|r| r.published_at.clone()),
        prepared_at: None,
        last_checked_at: last.as_ref().map(|_| chrono::Utc::now().to_rfc3339()),
        last_error: None,
        skipped_version: String::new(),
    })
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn check_github_update(
    _app: AppHandle,
    _update_method: Option<String>,
    _respect_cooldown: Option<bool>,
) -> Result<GithubUpdateInfo, String> {
    let current_version = env!("CARGO_PKG_VERSION").to_string();
    let client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15));
    let client = android_workspace_apply_static_webpki_roots(client_builder)?
        .build()
        .map_err(|err| format!("构建更新检查客户端失败: {err}"))?;
    let resp = client
        .get(ANDROID_UPDATE_RELEASE_API)
        .header("User-Agent", "P-ai-Android-Updater")
        .send()
        .await
        .map_err(|err| format!("检查更新失败: {err}"))?;
    if !resp.status().is_success() {
        return Ok(GithubUpdateInfo {
            current_version: current_version.clone(),
            latest_version: current_version.clone(),
            has_update: false,
            release_url: String::new(),
            update_source: "unsupported".to_string(),
            access_mode: "direct".to_string(),
            release_notes: String::new(),
            published_at: None,
            runtime_kind: "android".to_string(),
            can_force_update: false,
        });
    }
    let payload: serde_json::Value = resp
        .json()
        .await
        .map_err(|err| format!("解析更新信息失败: {err}"))?;
    let latest_tag = payload
        .get("tag_name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let latest_version = normalize_android_release_version(&latest_tag);
    let release_url = payload
        .get("html_url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let published_at = payload
        .get("published_at")
        .and_then(|v| v.as_str())
        .map(ToOwned::to_owned);
    let release_notes = payload
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let has_update = !latest_version.is_empty() && latest_version != current_version;
    let info = GithubUpdateInfo {
        current_version: current_version.clone(),
        latest_version: latest_version.clone(),
        has_update,
        release_url: release_url.clone(),
        update_source: "github".to_string(),
        access_mode: "direct".to_string(),
        release_notes: release_notes.clone(),
        published_at: published_at.clone(),
        runtime_kind: "android".to_string(),
        can_force_update: false,
    };
    let _ = ANDROID_LAST_CHECK.set(info);
    runtime_log_info(format!(
        "[远程更新] Android 检查完成，current={}，latest={}，has_update={}，url={}",
        current_version, latest_version, has_update, release_url
    ));
    Ok(GithubUpdateInfo {
        current_version,
        latest_version,
        has_update,
        release_url,
        update_source: "github".to_string(),
        access_mode: "direct".to_string(),
        release_notes,
        published_at,
        runtime_kind: "android".to_string(),
        can_force_update: false,
    })
}

fn cleanup_portable_update_temp_artifacts_for_current_runtime() -> Result<(), String> {
    Ok(())
}

fn start_github_auto_update_worker(_app: AppHandle) {}

#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn start_github_update(
    _app: AppHandle,
    _force: bool,
    _update_method: Option<String>,
) -> Result<(), String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn cancel_github_update() -> Result<(), String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

#[cfg(not(target_os = "android"))]
#[tauri::command]
async fn apply_prepared_github_update(_app: AppHandle) -> Result<(), String> {
    Err(ANDROID_UPDATE_UNSUPPORTED.to_string())
}

fn maybe_run_portable_update_helper_from_args() -> Result<bool, String> {
    Ok(false)
}
