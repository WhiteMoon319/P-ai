//! 时间语义统一（纯逻辑，无平台依赖）。
//! 当地时间（local）：用户/LLM/UI 可见时间。
//! 真实时间（UTC）：数据层存储、调度比较、跨时区稳定时间。

use time::{format_description::well_known::Rfc3339, OffsetDateTime, UtcOffset};

/// 当前 UTC 时刻（从 src-tauri runtime_state::now_utc 迁入）。
pub fn now_utc() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

/// 当前 UTC 时刻的 RFC3339 文本（与 now_utc_rfc3339 同义，从 runtime_state 迁入）。
pub fn now_iso() -> String {
    now_utc_rfc3339()
}

/// 解析 RFC3339 文本为 UTC 时刻（从 runtime_state 迁入）。
pub fn parse_iso(value: &str) -> Option<OffsetDateTime> {
    parse_rfc3339_time(value)
}

pub fn now_utc_rfc3339() -> String {
    now_utc()
        .replace_nanosecond(0)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string())
}

pub fn parse_rfc3339_time(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value.trim(), &Rfc3339).ok()
}

pub fn normalize_time_for_utc_storage(dt: OffsetDateTime) -> Result<String, String> {
    dt.to_offset(UtcOffset::UTC)
        .replace_nanosecond(0)
        .map_err(|err| format!("Normalize UTC time failed: {err}"))?
        .format(&Rfc3339)
        .map_err(|err| format!("Format UTC time failed: {err}"))
}

pub fn normalize_rfc3339_to_utc_storage(field_name: &str, value: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(format!("{field_name} must not be empty"));
    }
    let parsed = parse_rfc3339_time(trimmed).ok_or_else(|| {
        format!(
            "{field_name} must be RFC3339 with timezone offset, for example 2026-03-10T09:30:00+08:00"
        )
    })?;
    normalize_time_for_utc_storage(parsed)
}

pub fn local_utc_offset() -> Option<UtcOffset> {
    match UtcOffset::current_local_offset() {
        Ok(offset) => Some(offset),
        Err(err) => {
            eprintln!("[时间语义] 获取本地 UTC 偏移失败，回退为 UTC 显示: {err}");
            None
        }
    }
}

pub fn to_local_datetime(dt: OffsetDateTime) -> OffsetDateTime {
    if let Some(offset) = local_utc_offset() {
        dt.to_offset(offset)
    } else {
        dt
    }
}

pub fn format_offset_datetime_to_local_text(dt: OffsetDateTime) -> String {
    let local = to_local_datetime(dt);
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        local.year(),
        local.month() as u8,
        local.day(),
        local.hour(),
        local.minute(),
        local.second()
    )
}

pub fn format_offset_datetime_to_local_rfc3339(dt: OffsetDateTime) -> String {
    to_local_datetime(dt)
        .replace_nanosecond(0)
        .ok()
        .and_then(|value| value.format(&Rfc3339).ok())
        .unwrap_or_else(|| format_offset_datetime_to_local_text(dt))
}

pub fn now_local_rfc3339() -> String {
    format_offset_datetime_to_local_rfc3339(now_utc())
}

pub fn format_utc_storage_time_to_local_rfc3339(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(dt) = parse_rfc3339_time(trimmed) {
        return format_offset_datetime_to_local_rfc3339(dt);
    }
    trimmed.to_string()
}

pub fn format_utc_storage_time_to_local_text(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(dt) = parse_rfc3339_time(trimmed) {
        return format_offset_datetime_to_local_text(dt);
    }
    let mut normalized = trimmed.replace('T', " ");
    if let Some((head, _)) = normalized.split_once('.') {
        normalized = head.to_string();
    }
    if normalized.ends_with('Z') {
        normalized.pop();
    }
    if normalized.chars().count() > 19 {
        normalized.chars().take(19).collect::<String>()
    } else {
        normalized
    }
}

// 与前端 ChatView 的时间分隔标签规则保持一致。
pub fn format_offset_datetime_to_local_relative_label(dt: OffsetDateTime) -> String {
    let now = now_utc();
    let local = to_local_datetime(dt);
    let now_local = to_local_datetime(now);
    let elapsed_minutes = (now - dt).whole_minutes();
    let clock = format!("{:02}:{:02}", local.hour(), local.minute());

    if local.date() == now_local.date()
        && local.hour() == now_local.hour()
        && elapsed_minutes > 0
    {
        return format!("{elapsed_minutes} 分钟前");
    }

    if local.date() == now_local.date() {
        return clock;
    }

    let month_day = format!("{:02}-{:02}", local.month() as u8, local.day());
    if local.year() == now_local.year() {
        return format!("{month_day} {clock}");
    }

    format!("{:04}-{month_day} {clock}", local.year())
}

pub fn format_utc_storage_time_to_local_relative_label(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if let Some(dt) = parse_rfc3339_time(trimmed) {
        return format_offset_datetime_to_local_relative_label(dt);
    }
    trimmed.to_string()
}

#[cfg(test)]
pub(crate) mod time_semantics_tests {
    use super::*;

    #[test]
    fn parse_and_normalize_roundtrip() {
        let raw = "2026-03-10T09:30:00+08:00";
        let dt = parse_rfc3339_time(raw).expect("parse");
        let normalized = normalize_time_for_utc_storage(dt).expect("normalize");
        assert_eq!(normalized, "2026-03-10T01:30:00Z");
    }

    #[test]
    fn local_rfc3339_preserves_offset() {
        let dt = parse_rfc3339_time("2026-03-10T09:30:00+08:00").expect("parse");
        let formatted = format_offset_datetime_to_local_rfc3339(dt);
        assert!(formatted.starts_with("2026-03-10T"));
        assert!(formatted.contains('+') || formatted.ends_with('Z'));
    }

    #[test]
    fn storage_text_fallback_handles_non_rfc3339() {
        let raw = "2026-03-10 09:30:00.123";
        let text = format_utc_storage_time_to_local_text(raw);
        assert!(text.starts_with("2026-03-10 09:30:00"));
    }
}
