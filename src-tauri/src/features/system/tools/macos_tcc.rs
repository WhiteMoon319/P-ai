// ==================== macOS TCC 权限提示 ====================
// TCC（完全磁盘访问/文件和文件夹）拒绝文件访问时返回 EPERM，
// strerror 文本为 "Operation not permitted (os error 1)"；
// 普通文件权限（chmod/ACL）拒绝返回 EACCES，文本为 "Permission denied (os error 13)"。

#[cfg(target_os = "macos")]
fn looks_like_permission_error(text: &str) -> bool {
    // TCC 拒绝文件访问返回 EPERM，strerror 为 "Operation not permitted"。
    // EACCES（Permission denied）是普通 chmod/ACL 权限问题，不属于 TCC。
    text.to_ascii_lowercase().contains("operation not permitted")
}

/// 路径是否落在 macOS TCC 受保护区域（/etc、/var、/System、/Library、~/Desktop 等）。
/// 用 Path::components 校验直接目录段，避免 /Users/me/Projects/Library 这类嵌套目录误命中。
#[cfg(target_os = "macos")]
fn is_macos_tcc_protected_path(path: &std::path::Path) -> bool {
    use std::path::Component;
    let mut components = path.components();
    if !matches!(components.next(), Some(Component::RootDir)) {
        return false;
    }
    let Some(Component::Normal(first)) = components.next() else {
        return false;
    };
    let first = first.to_string_lossy();
    if matches!(
        first.as_ref(),
        "etc" | "var" | "private" | "System" | "Library"
    ) {
        return true;
    }
    if first.as_ref() != "Users" {
        return false;
    }
    let Some(Component::Normal(user)) = components.next() else {
        return false;
    };
    if user.to_string_lossy().as_ref() == "Shared" {
        // /Users/Shared 不受 TCC 保护
        return false;
    }
    let Some(Component::Normal(folder)) = components.next() else {
        return false;
    };
    matches!(
        folder.to_string_lossy().as_ref(),
        "Desktop" | "Documents" | "Downloads" | "Library"
    )
}

/// macOS 上检测权限类错误并返回引导 LLM 的授权建议；非 macOS 或未命中返回 None。
fn macos_tcc_permission_hint(
    ui_language: &str,
    error_text: &str,
    path: Option<&std::path::Path>,
) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        if !looks_like_permission_error(error_text) {
            return None;
        }
        if let Some(path) = path {
            if !is_macos_tcc_protected_path(path) {
                return None;
            }
        }
        let zh_cn = "该错误可能是 macOS 隐私权限（TCC）拦截：PAI 未被授予读取该路径所需的权限。请引导用户完成授权：1. 若目标是 /etc、/var、~/Library 等系统路径，需要授予「完全磁盘访问权限」——执行 open \"x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles\" 打开设置面板，找到 PAI（或 easycall）并打开开关；若列表中没有，点击 + 手动添加应用。2. 若目标是桌面、文稿、下载等用户目录，可在「隐私与安全性 > 文件和文件夹」中授权，或让 PAI 通过文件选择框选择一次该目录。3. 授权后若仍无效，请重启应用后再试。";
        let zh_tw = "該錯誤可能是 macOS 隱私權限（TCC）攔截：PAI 未被授予讀取該路徑所需的權限。請引導用戶完成授權：1. 若目標是 /etc、/var、~/Library 等系統路徑，需要授予「完全磁碟存取權限」——執行 open \"x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles\" 開啟設定面板，找到 PAI（或 easycall）並開啟開關；若清單中沒有，點擊 + 手動加入應用程式。2. 若目標是桌面、文件、下載等使用者目錄，可在「隱私權與安全性 > 檔案和資料夾」中授權，或讓 PAI 透過檔案選擇框選擇一次該目錄。3. 授權後若仍無效，請重新啟動應用程式後再試。";
        let en = "This error is likely caused by macOS privacy permissions (TCC): PAI has not been granted access to this path. Guide the user to grant access: 1. For system paths like /etc, /var, or ~/Library, PAI needs Full Disk Access — run open \"x-apple.systempreferences:com.apple.preference.security?Privacy_AllFiles\" to open the settings pane, find PAI (or easycall) in the list and enable the toggle; if it is not listed, click + to add the app manually. 2. For user folders like Desktop, Documents, or Downloads, grant access under Privacy & Security > Files and Folders, or have PAI pick the folder once via a file chooser. 3. If access still fails after granting, restart the app and try again.";
        Some(terminal_localized_text(ui_language, zh_cn, zh_tw, en))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (ui_language, error_text, path);
        None
    }
}

#[cfg(all(test, target_os = "macos"))]
mod macos_tcc_tests {
    use super::*;

    #[test]
    fn permission_hint_should_match_eperm_text() {
        let hint = macos_tcc_permission_hint(
            "zh-CN",
            "cat: /etc/hosts: Operation not permitted",
            Some(std::path::Path::new("/etc/hosts")),
        );
        let hint = hint.expect("hint for EPERM on protected path");
        assert!(hint.contains("完全磁盘访问权限"));
        assert!(hint.contains("Privacy_AllFiles"));
    }

    #[test]
    fn permission_hint_should_ignore_unrelated_errors() {
        let hint = macos_tcc_permission_hint(
            "zh-CN",
            "No such file or directory",
            Some(std::path::Path::new("/etc/hosts")),
        );
        assert!(hint.is_none());
    }

    #[test]
    fn permission_hint_should_ignore_eacces_text() {
        // EACCES（Permission denied）是普通 chmod/ACL 权限，不属于 TCC
        let hint = macos_tcc_permission_hint(
            "zh-CN",
            "Permission denied",
            Some(std::path::Path::new("/etc/hosts")),
        );
        assert!(hint.is_none());
    }

    #[test]
    fn permission_hint_should_ignore_unprotected_paths() {
        let hint = macos_tcc_permission_hint(
            "zh-CN",
            "Operation not permitted",
            Some(std::path::Path::new("/Users/me/Projects/app/main.rs")),
        );
        assert!(hint.is_none());
    }

    #[test]
    fn permission_hint_should_ignore_nested_folders_with_protected_names() {
        // 嵌套目录含 Desktop/Library 名字不应命中
        let nested_desktop = macos_tcc_permission_hint(
            "zh-CN",
            "Operation not permitted",
            Some(std::path::Path::new("/Users/me/Projects/Desktop-notes/a.txt")),
        );
        assert!(nested_desktop.is_none());
        let nested_library = macos_tcc_permission_hint(
            "zh-CN",
            "Operation not permitted",
            Some(std::path::Path::new("/Users/me/Projects/Library/a.txt")),
        );
        assert!(nested_library.is_none());
    }

    #[test]
    fn permission_hint_without_path_should_match_permission_text() {
        let hint = macos_tcc_permission_hint("en-US", "Operation not permitted", None);
        let hint = hint.expect("hint for EPERM without path");
        assert!(hint.contains("Full Disk Access"));
    }

    #[test]
    fn protected_path_should_detect_system_and_user_folders() {
        assert!(is_macos_tcc_protected_path(std::path::Path::new("/etc/hosts")));
        assert!(is_macos_tcc_protected_path(std::path::Path::new(
            "/var/log/system.log"
        )));
        assert!(is_macos_tcc_protected_path(std::path::Path::new(
            "/Library/Application Support/x"
        )));
        assert!(is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/me/Desktop/a.txt"
        )));
        assert!(is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/me/Documents/a.txt"
        )));
        assert!(is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/me/Downloads/a.txt"
        )));
        assert!(is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/me/Library/Application Support/x"
        )));
        assert!(!is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/me/Projects/app/main.rs"
        )));
        assert!(!is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/me/Projects/Desktop-notes/a.txt"
        )));
        assert!(!is_macos_tcc_protected_path(std::path::Path::new(
            "/Users/Shared/x.txt"
        )));
        assert!(!is_macos_tcc_protected_path(std::path::Path::new("/tmp/x")));
    }
}
