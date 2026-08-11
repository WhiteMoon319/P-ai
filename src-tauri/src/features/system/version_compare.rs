// Android 版本号解析与比较（纯逻辑，无 crate 依赖，可被 integration test 直接 include）。
// 用途：updater 判断 latest 是否严格比 current 新，正确处理 v 前缀 / 预发布 / 版本大小关系。

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AndroidVersionParts {
    pub(crate) major: u64,
    pub(crate) minor: u64,
    pub(crate) patch: u64,
    pub(crate) prerelease: Option<String>,
}

pub(crate) fn parse_android_version(raw: &str) -> Option<AndroidVersionParts> {
    let normalized = raw
        .trim()
        .trim_start_matches('v')
        .trim_start_matches('V');
    let (core, prerelease) = match normalized.split_once('-') {
        Some((core, pre)) => (core, Some(pre.to_string())),
        None => (normalized, None),
    };
    let mut segments = core.split('.');
    let major = segments.next()?.trim().parse::<u64>().ok()?;
    let minor = match segments.next() {
        Some(seg) => seg.trim().parse::<u64>().ok()?,
        None => 0,
    };
    let patch = match segments.next() {
        Some(seg) => seg.trim().parse::<u64>().ok()?,
        None => 0,
    };
    Some(AndroidVersionParts {
        major,
        minor,
        patch,
        prerelease: prerelease.map(|p| p.trim().to_string()).filter(|p| !p.is_empty()),
    })
}

impl PartialOrd for AndroidVersionParts {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for AndroidVersionParts {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let core = (self.major, self.minor, self.patch).cmp(&(other.major, other.minor, other.patch));
        if core != std::cmp::Ordering::Equal {
            return core;
        }
        match (&self.prerelease, &other.prerelease) {
            (None, None) => std::cmp::Ordering::Equal,
            // 正式版（无预发布段）比任何预发布新
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (Some(_), None) => std::cmp::Ordering::Less,
            (Some(a), Some(b)) => a.cmp(b),
        }
    }
}

/// 判断 latest 是否严格比 current 新（v 前缀 / 预发布 / 版本大小都正确处理）。
pub(crate) fn android_version_is_newer(latest: &str, current: &str) -> bool {
    let Some(latest_parts) = parse_android_version(latest) else {
        return false;
    };
    let Some(current_parts) = parse_android_version(current) else {
        // 当前版本解析失败时，仅当两侧字符串去 v 后不同才视为有更新（保守）
        return normalize_android_release_version(latest)
            != normalize_android_release_version(current);
    };
    latest_parts > current_parts
}

pub(crate) fn normalize_android_release_version(raw: &str) -> String {
    raw.trim()
        .trim_start_matches('v')
        .trim_start_matches('V')
        .to_string()
}

#[cfg(test)]
mod android_version_comparison_tests {
    use super::*;

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
}
