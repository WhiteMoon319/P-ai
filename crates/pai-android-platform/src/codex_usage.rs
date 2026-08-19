//! Codex 用量查询纯逻辑（端点解析 / 窗口计算 / 快照转换 / 网络请求）。
//! 阶段 6 由 src-tauri features/system/commands/codex_usage.rs 迁入。

use pai_backend::core::domain::types_config::{
    DEFAULT_CODEX_BASE_URL, default_codex_auth_mode, default_codex_local_auth_path,
};
use pai_backend::core::domain::runtime_types::CodexRuntimeAuth;
use pai_backend::logging::runtime_log_info;
use pai_backend::core::time_semantics::now_iso;

use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexGetRateLimitsInput {
    pub provider_id: String,
    #[serde(default = "default_codex_auth_mode")]
    pub auth_mode: String,
    #[serde(default = "default_codex_local_auth_path")]
    pub local_auth_path: String,
    #[serde(default = "default_codex_usage_base_url")]
    pub base_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitWindow {
    pub used_percent: i32,
    pub window_duration_mins: Option<i64>,
    pub resets_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexCreditsSnapshot {
    pub has_credits: bool,
    pub unlimited: bool,
    pub balance: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitSnapshot {
    pub limit_id: String,
    pub limit_name: String,
    pub primary: Option<CodexRateLimitWindow>,
    pub secondary: Option<CodexRateLimitWindow>,
    pub credits: Option<CodexCreditsSnapshot>,
    pub plan_type: String,
    pub rate_limit_reached_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexRateLimitQueryResult {
    pub usage_url: String,
    pub preferred_snapshot: Option<CodexRateLimitSnapshot>,
    pub snapshots: Vec<CodexRateLimitSnapshot>,
    pub rate_limit_reset_credit_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexConsumeRateLimitResetCreditResult {
    pub outcome: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsagePayload {
    #[serde(default)]
    pub plan_type: String,
    #[serde(default)]
    pub rate_limit: Option<CodexUsageRateLimitDetails>,
    #[serde(default)]
    pub credits: Option<CodexUsageCreditsDetails>,
    #[serde(default)]
    pub additional_rate_limits: Option<Vec<CodexUsageAdditionalRateLimitDetails>>,
    #[serde(default)]
    pub rate_limit_reached_type: Option<CodexUsageRateLimitReachedType>,
    #[serde(default)]
    pub rate_limit_reset_credits: Option<CodexUsageRateLimitResetCredits>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsageRateLimitResetCredits {
    #[serde(default)]
    pub available_count: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsageRateLimitDetails {
    #[serde(default)]
    pub primary_window: Option<CodexUsageWindowSnapshot>,
    #[serde(default)]
    pub secondary_window: Option<CodexUsageWindowSnapshot>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsageWindowSnapshot {
    #[serde(default)]
    pub used_percent: i32,
    #[serde(default)]
    pub limit_window_seconds: i32,
    #[serde(default)]
    pub reset_at: i32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsageCreditsDetails {
    #[serde(default)]
    pub has_credits: bool,
    #[serde(default)]
    pub unlimited: bool,
    #[serde(default)]
    pub balance: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsageAdditionalRateLimitDetails {
    #[serde(default)]
    pub limit_name: String,
    #[serde(default)]
    pub metered_feature: String,
    #[serde(default)]
    pub rate_limit: Option<CodexUsageRateLimitDetails>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CodexUsageRateLimitReachedType {
    #[serde(rename = "type", default)]
    pub kind: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexUsagePathStyle {
    ChatGptApi,
    CodexApi,
}

#[derive(Debug)]
pub struct CodexUsageRequestError {
    pub status_code: Option<u16>,
    pub message: String,
}

pub fn default_codex_usage_base_url() -> String {
    DEFAULT_CODEX_BASE_URL.to_string()
}

pub fn codex_usage_log_info(
    provider_id: &str,
    status: &str,
    trigger: &str,
    duration_ms: u128,
    extra_fields: &[(&str, String)],
) {
    let extras = extra_fields
        .iter()
        .map(|(key, value)| format!("{key}={value}"))
        .collect::<Vec<_>>()
        .join(" ");
    let suffix = if extras.is_empty() {
        String::new()
    } else {
        format!(" {extras}")
    };
    runtime_log_info(format!(
        "[Codex用量] 任务=周用量查询 状态={} provider_id={} 触发={} 耗时毫秒={} 时间={}{}",
        status,
        provider_id.trim(),
        trigger,
        duration_ms,
        now_iso(),
        suffix
    ));
}

pub fn codex_usage_resolve_base_url(base_url: &str) -> (String, CodexUsagePathStyle) {
    let trimmed = {
        let candidate = base_url.trim().trim_end_matches('/');
        if candidate.is_empty() {
            DEFAULT_CODEX_BASE_URL.trim_end_matches('/').to_string()
        } else {
            candidate.to_string()
        }
    };
    let lower = trimmed.to_ascii_lowercase();

    if let Some(index) = lower.find("/backend-api") {
        let backend_base = trimmed[..index + "/backend-api".len()]
            .trim_end_matches('/')
            .to_string();
        return (backend_base, CodexUsagePathStyle::ChatGptApi);
    }

    if lower.starts_with("https://chatgpt.com") || lower.starts_with("https://chat.openai.com") {
        return (
            format!("{}/backend-api", trimmed.trim_end_matches('/')),
            CodexUsagePathStyle::ChatGptApi,
        );
    }

    if let Some(index) = lower.find("/api/codex") {
        let api_base = trimmed[..index].trim_end_matches('/').to_string();
        return (api_base, CodexUsagePathStyle::CodexApi);
    }

    if lower.ends_with("/v1") {
        let api_base = trimmed[..trimmed.len().saturating_sub(3)]
            .trim_end_matches('/')
            .to_string();
        return (api_base, CodexUsagePathStyle::CodexApi);
    }

    (trimmed, CodexUsagePathStyle::CodexApi)
}

pub fn codex_usage_endpoint(base_url: &str) -> String {
    let (resolved_base, path_style) = codex_usage_resolve_base_url(base_url);
    match path_style {
        CodexUsagePathStyle::ChatGptApi => format!("{resolved_base}/wham/usage"),
        CodexUsagePathStyle::CodexApi => format!("{resolved_base}/api/codex/usage"),
    }
}

pub fn codex_rate_limit_reset_consume_endpoint(base_url: &str) -> String {
    let (resolved_base, path_style) = codex_usage_resolve_base_url(base_url);
    match path_style {
        CodexUsagePathStyle::ChatGptApi => {
            format!("{resolved_base}/wham/rate-limit-reset-credits/consume")
        }
        CodexUsagePathStyle::CodexApi => {
            format!("{resolved_base}/api/codex/rate-limit-reset-credits/consume")
        }
    }
}

pub fn codex_window_duration_mins(limit_window_seconds: i32) -> Option<i64> {
    if limit_window_seconds <= 0 {
        return None;
    }
    let seconds = i64::from(limit_window_seconds);
    Some((seconds + 59) / 60)
}

pub fn codex_rate_limit_window_from_usage(
    window: Option<CodexUsageWindowSnapshot>,
) -> Option<CodexRateLimitWindow> {
    let snapshot = window?;
    Some(CodexRateLimitWindow {
        used_percent: snapshot.used_percent,
        window_duration_mins: codex_window_duration_mins(snapshot.limit_window_seconds),
        resets_at: (snapshot.reset_at > 0).then_some(i64::from(snapshot.reset_at)),
    })
}

pub fn codex_credits_snapshot_from_usage(
    credits: Option<CodexUsageCreditsDetails>,
) -> Option<CodexCreditsSnapshot> {
    let details = credits?;
    Some(CodexCreditsSnapshot {
        has_credits: details.has_credits,
        unlimited: details.unlimited,
        balance: details.balance.and_then(|value| {
            let trimmed = value.trim().to_string();
            (!trimmed.is_empty()).then_some(trimmed)
        }),
    })
}

pub fn codex_rate_limit_snapshot_from_usage(
    limit_id: Option<String>,
    limit_name: Option<String>,
    rate_limit: Option<CodexUsageRateLimitDetails>,
    credits: Option<CodexUsageCreditsDetails>,
    plan_type: &str,
    rate_limit_reached_type: &str,
) -> CodexRateLimitSnapshot {
    let primary = rate_limit
        .as_ref()
        .and_then(|details| codex_rate_limit_window_from_usage(details.primary_window.clone()));
    let secondary = rate_limit
        .as_ref()
        .and_then(|details| codex_rate_limit_window_from_usage(details.secondary_window.clone()));
    let limit_id_value = limit_id
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_default();
    let limit_name_value = limit_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| limit_id_value.clone());
    CodexRateLimitSnapshot {
        limit_id: limit_id_value,
        limit_name: limit_name_value,
        primary,
        secondary,
        credits: codex_credits_snapshot_from_usage(credits),
        plan_type: plan_type.trim().to_string(),
        rate_limit_reached_type: rate_limit_reached_type.trim().to_string(),
    }
}

pub fn codex_rate_limit_snapshots_from_payload(
    payload: CodexUsagePayload,
) -> Vec<CodexRateLimitSnapshot> {
    let CodexUsagePayload {
        plan_type,
        rate_limit,
        credits,
        additional_rate_limits,
        rate_limit_reached_type,
        ..
    } = payload;
    let rate_limit_reached_kind = rate_limit_reached_type
        .map(|details| details.kind)
        .unwrap_or_default();
    let mut snapshots = vec![codex_rate_limit_snapshot_from_usage(
        Some("codex".to_string()),
        None,
        rate_limit,
        credits,
        &plan_type,
        &rate_limit_reached_kind,
    )];

    if let Some(additional) = additional_rate_limits {
        for item in additional {
            snapshots.push(codex_rate_limit_snapshot_from_usage(
                Some(item.metered_feature),
                Some(item.limit_name),
                item.rate_limit,
                None,
                &plan_type,
                "",
            ));
        }
    }

    snapshots
}

pub async fn codex_fetch_usage_payload(
    url: &str,
    auth: &CodexRuntimeAuth,
) -> Result<(CodexUsagePayload, String), CodexUsageRequestError> {
    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30));
    #[cfg(target_os = "android")]
    {
        client_builder = crate::tls::android_workspace_apply_static_webpki_roots(client_builder)
            .map_err(|err| CodexUsageRequestError {
                status_code: None,
                message: err,
            })?;
    }
    let client = client_builder
        .build()
        .map_err(|err| CodexUsageRequestError {
            status_code: None,
            message: format!("构建 Codex 用量客户端失败: {err}"),
        })?;

    let mut request = client
        .get(url)
        .header(AUTHORIZATION, format!("Bearer {}", auth.access_token))
        .header("User-Agent", "easy-call-ai/codex-usage");
    if let Some(account_id) = auth
        .account_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        request = request.header("ChatGPT-Account-Id", account_id.to_string());
    }

    let response = request.send().await.map_err(|err| CodexUsageRequestError {
        status_code: None,
        message: format!("请求 Codex 用量接口失败: {err}"),
    })?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(CodexUsageRequestError {
            status_code: Some(status.as_u16()),
            message: format!("Codex 用量接口返回异常: {} | {}", status, body),
        });
    }

    let payload = serde_json::from_str::<CodexUsagePayload>(&body).map_err(|err| CodexUsageRequestError {
        status_code: None,
        message: format!("解析 Codex 用量响应失败: {err}"),
    })?;
    Ok((payload, body))
}

pub async fn codex_consume_rate_limit_reset_credit_request(
    url: &str,
    auth: &CodexRuntimeAuth,
) -> Result<CodexConsumeRateLimitResetCreditResult, CodexUsageRequestError> {
    let mut client_builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30));
    #[cfg(target_os = "android")]
    {
        client_builder = crate::tls::android_workspace_apply_static_webpki_roots(client_builder)
            .map_err(|err| CodexUsageRequestError {
                status_code: None,
                message: err,
            })?;
    }
    let client = client_builder
        .build()
        .map_err(|err| CodexUsageRequestError {
            status_code: None,
            message: format!("构建 Codex 重置客户端失败: {err}"),
        })?;
    let mut request = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {}", auth.access_token))
        .header("User-Agent", "easy-call-ai/codex-usage")
        .json(&serde_json::json!({ "redeem_request_id": Uuid::new_v4().to_string() }));
    if let Some(account_id) = auth.account_id.as_deref().map(str::trim).filter(|value| !value.is_empty()) {
        request = request.header("ChatGPT-Account-Id", account_id.to_string());
    }
    let response = request.send().await.map_err(|err| CodexUsageRequestError {
        status_code: None,
        message: format!("请求 Codex 重置额度失败: {err}"),
    })?;
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        return Err(CodexUsageRequestError {
            status_code: Some(status.as_u16()),
            message: format!("Codex 重置额度接口返回异常: {status} | {body}"),
        });
    }
    serde_json::from_str(&body).map_err(|err| CodexUsageRequestError {
        status_code: None,
        message: format!("解析 Codex 重置额度响应失败: {err}"),
    })
}



#[cfg(test)]
pub mod codex_usage_tests {
    use super::*;

    #[test]
    fn codex_usage_resolve_base_url_supports_current_chatgpt_default() {
        let (base_url, path_style) =
            codex_usage_resolve_base_url("https://chatgpt.com/backend-api/codex");
        assert_eq!(base_url, "https://chatgpt.com/backend-api");
        assert_eq!(path_style, CodexUsagePathStyle::ChatGptApi);
    }

    #[test]
    fn codex_usage_resolve_base_url_supports_codex_api_style() {
        let (base_url, path_style) =
            codex_usage_resolve_base_url("https://example.com/api/codex");
        assert_eq!(base_url, "https://example.com");
        assert_eq!(path_style, CodexUsagePathStyle::CodexApi);
    }

    #[test]
    fn codex_reset_credit_consume_endpoint_matches_usage_path_style() {
        assert_eq!(
            codex_rate_limit_reset_consume_endpoint("https://chatgpt.com/backend-api/codex"),
            "https://chatgpt.com/backend-api/wham/rate-limit-reset-credits/consume"
        );
        assert_eq!(
            codex_rate_limit_reset_consume_endpoint("https://example.com/api/codex"),
            "https://example.com/api/codex/rate-limit-reset-credits/consume"
        );
    }

    #[test]
    fn codex_rate_limit_snapshots_from_payload_prefers_secondary_window_data() {
        let snapshots = codex_rate_limit_snapshots_from_payload(CodexUsagePayload {
            plan_type: "pro".to_string(),
            rate_limit: Some(CodexUsageRateLimitDetails {
                primary_window: Some(CodexUsageWindowSnapshot {
                    used_percent: 18,
                    limit_window_seconds: 300,
                    reset_at: 1_700_000_000,
                }),
                secondary_window: Some(CodexUsageWindowSnapshot {
                    used_percent: 64,
                    limit_window_seconds: 7 * 24 * 60 * 60,
                    reset_at: 1_700_100_000,
                }),
            }),
            credits: Some(CodexUsageCreditsDetails {
                has_credits: true,
                unlimited: false,
                balance: Some("12.5".to_string()),
            }),
            additional_rate_limits: None,
            rate_limit_reached_type: Some(CodexUsageRateLimitReachedType {
                kind: "rate_limit_reached".to_string(),
            }),
            rate_limit_reset_credits: None,
        });

        assert_eq!(snapshots.len(), 1);
        let snapshot = &snapshots[0];
        assert_eq!(snapshot.limit_id, "codex");
        assert_eq!(snapshot.plan_type, "pro");
        assert_eq!(snapshot.primary.as_ref().map(|item| item.used_percent), Some(18));
        assert_eq!(
            snapshot
                .secondary
                .as_ref()
                .and_then(|item| item.window_duration_mins),
            Some(10080)
        );
        assert_eq!(
            snapshot
                .credits
                .as_ref()
                .and_then(|item| item.balance.clone()),
            Some("12.5".to_string())
        );
    }
}
