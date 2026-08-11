// Android 平台的应用内更新 stub：移动端不支持 GitHub 自更新，
// 保留与 updater.rs 相同的对外签名，命令返回友好错误或空状态。

pub(crate) mod android_version_compare {
    include!("version_compare.rs");
}
use android_version_compare::*;

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GithubUpdateInfo {
    pub(crate) current_version: String,
    pub(crate) latest_version: String,
    pub(crate) has_update: bool,
    pub(crate) release_url: String,
    pub(crate) update_source: String,
    pub(crate) access_mode: String,
    pub(crate) release_notes: String,
    pub(crate) published_at: Option<String>,
    pub(crate) runtime_kind: String,
    pub(crate) can_force_update: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct GithubUpdateState {
    pub(crate) stage: String,
    pub(crate) current_version: String,
    pub(crate) latest_version: String,
    pub(crate) runtime_kind: String,
    pub(crate) has_prepared_update: bool,
    pub(crate) has_visible_update: bool,
    pub(crate) release_notes: String,
    pub(crate) release_url: String,
    pub(crate) published_at: Option<String>,
    pub(crate) prepared_at: Option<String>,
    pub(crate) last_checked_at: Option<String>,
    pub(crate) last_error: Option<String>,
    pub(crate) skipped_version: String,
}

pub(crate) const ANDROID_UPDATE_UNSUPPORTED: &str = "Android 平台暂不支持应用内更新";

/// Android 更新检查仓库：Android 移植版独立发布仓库（APK 在 GitHub Release 发布）。
pub(crate) const ANDROID_UPDATE_REPO: &str = "WhiteMoon319/P-ai";
pub(crate) const ANDROID_UPDATE_RELEASE_API: &str =
    "https://api.github.com/repos/WhiteMoon319/P-ai/releases/latest";
pub(crate) const ANDROID_UPDATE_CHANGELOG_RAW: &str =
    "https://raw.githubusercontent.com/WhiteMoon319/P-ai/main/docs/changelog/latest.md";

/// Android 应用当前版本：优先使用构建期注入的 `PAI_ANDROID_APP_VERSION`
/// （CI 由 patch-android-version.sh 从 git describe 派生，与 APK versionName 同源），
/// 本地构建未注入时回退到 Cargo.toml 版本。禁止运行时读取外部版本文件。
pub(crate) fn android_current_app_version() -> String {
    option_env!("PAI_ANDROID_APP_VERSION")
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string())
}

/// 拉取 GitHub 上的最新 changelog（供前端「关于」页展示版本更新说明）。
pub(crate) async fn fetch_project_changelog_markdown() -> Result<String, String> {
    let client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15));
    let client = features_system_commands::android_workspace_rootfs_installer::android_workspace_apply_static_webpki_roots(client_builder)?
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

pub(crate) fn sync_update_state_from_skip_version(_app: &NativeAppHandle, _version: &str) {}

/// 最近一次 Android 更新检查结果（内存态，供 get_github_update_state 读取）。
pub(crate) static ANDROID_LAST_CHECK: std::sync::OnceLock<GithubUpdateInfo> = std::sync::OnceLock::new();



pub(crate) fn cleanup_portable_update_temp_artifacts_for_current_runtime() -> Result<(), String> {
    Ok(())
}

pub(crate) fn start_github_auto_update_worker(_app: NativeAppHandle) {}

/// 检查 GitHub Release 是否有新版本（真实 API 调用，替代硬编码 hasUpdate=false）。
pub(crate) async fn check_github_update_android(
    _app: &NativeAppHandle,
    _update_method: Option<String>,
    _respect_cooldown: Option<bool>,
) -> Result<GithubUpdateInfo, String> {
    let current_version = android_current_app_version();
    let client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15));
    let client = features_system_commands::android_workspace_rootfs_installer::android_workspace_apply_static_webpki_roots(client_builder)?
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
    // 版本比较必须处理 v 前缀 / 预发布 / 大小关系，不能只做字符串不等判断。
    let has_update = !latest_version.is_empty() && android_version_is_newer(&latest_version, &current_version);
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

pub(crate) fn maybe_run_portable_update_helper_from_args() -> Result<bool, String> {
    Ok(false)
}
