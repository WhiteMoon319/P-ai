#[derive(Debug, Clone)]
pub(crate) struct TerminalCommandPathCandidate {
    pub(crate) path: PathBuf,
    pub(crate) is_absolute: bool,
}

pub(crate) fn terminal_tokenize(command: &str) -> Vec<String> {
    terminal_lex_command(command)
}

pub(crate) fn terminal_unquote_token(token: &str) -> String {
    let trimmed = token.trim();
    if trimmed.len() >= 2 {
        let bytes = trimmed.as_bytes();
        if (bytes[0] == b'\'' && bytes[trimmed.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[trimmed.len() - 1] == b'"')
        {
            return trimmed[1..trimmed.len() - 1].to_string();
        }
    }
    trimmed.to_string()
}

#[cfg(target_os = "windows")]
pub(crate) fn terminal_has_windows_drive_prefix(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.len() < 2 || bytes[1] != b':' || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    if bytes.len() == 2 {
        return true;
    }
    matches!(bytes[2], b'\\' | b'/')
}

#[cfg(not(target_os = "windows"))]
pub(crate) fn terminal_has_windows_drive_prefix(_token: &str) -> bool {
    false
}

pub(crate) fn terminal_is_posix_style_shell(shell_kind: &str) -> bool {
    matches!(shell_kind, "git-bash" | "bash" | "zsh" | "sh")
}

pub(crate) fn terminal_is_virtual_sink_path(token: &str, shell_kind: &str) -> bool {
    let trimmed = token.trim().to_ascii_lowercase();
    if terminal_is_posix_style_shell(shell_kind) {
        return matches!(
            trimmed.as_str(),
            "/dev/null" | "/dev/stdout" | "/dev/stderr" | "/dev/tty"
        );
    }
    matches!(trimmed.as_str(), "nul" | "con" | "prn" | "aux")
}

#[cfg(test)]
pub(crate) fn terminal_command_contains_absolute_path_token(command: &str, shell_kind: &str) -> bool {
    let tokens = terminal_tokenize(command);
    for token in tokens {
        let unquoted = terminal_unquote_token(&token);
        let trimmed = unquoted.trim();
        if trimmed.is_empty() {
            continue;
        }
        if terminal_is_virtual_sink_path(trimmed, shell_kind) {
            continue;
        }
        if trimmed.contains("://") {
            continue;
        }
        if PathBuf::from(trimmed).is_absolute() || terminal_has_windows_drive_prefix(trimmed) {
            return true;
        }
    }
    false
}

pub(crate) fn terminal_resolve_candidate_path(cwd: &Path, raw: &str) -> Option<PathBuf> {
    let token = terminal_unquote_token(raw);
    if token.is_empty() {
        return None;
    }
    if token.starts_with('-') {
        return None;
    }
    if token.contains('*') || token.contains('?') {
        return None;
    }
    if token.contains("://") {
        return None;
    }
    if matches!(token.as_str(), "|" | "||" | "&" | "&&" | ";" | ">" | ">>" | "<")
    {
        return None;
    }
    let normalized = normalize_terminal_path_input_for_current_platform(&token);
    if normalized.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(&normalized);
    let joined = if candidate.is_absolute() {
        candidate
    } else {
        cwd.join(candidate)
    };
    Some(joined)
}

pub(crate) fn terminal_raw_token_is_absolute_path(raw: &str) -> bool {
    let token = terminal_unquote_token(raw);
    if token.is_empty() {
        return false;
    }
    let normalized = normalize_terminal_path_input_for_current_platform(&token);
    if normalized.is_empty() {
        return false;
    }
    PathBuf::from(&normalized).is_absolute() || terminal_has_windows_drive_prefix(&normalized)
}

pub(crate) fn terminal_dedup_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    let mut seen = std::collections::HashSet::<String>::new();
    for path in paths {
        let key = normalize_terminal_path_for_compare(&path);
        if seen.insert(key) {
            out.push(path);
        }
    }
    out
}


#[cfg(test)]
pub(crate) fn terminal_collect_command_path_candidates(
    cwd: &Path,
    command: &str,
    shell_kind: &str,
) -> Vec<PathBuf> {
    terminal_analyze_command(cwd, command, shell_kind)
        .path_candidates()
        .into_iter()
        .map(|item| item.path)
        .collect()
}

#[cfg(test)]
mod terminal_matcher_tests {
    use super::*;

    #[test]
    fn should_ignore_dev_null_in_absolute_path_check_for_git_bash() {
        let cmd = r#"ls -la ./skills/ 2>/dev/null || echo "No skills directory""#;
        assert!(!terminal_command_contains_absolute_path_token(cmd, "git-bash"));
    }

    #[test]
    fn should_collect_relative_path_but_not_dev_null_for_git_bash() {
        let cwd = PathBuf::from("C:\\Users\\tester\\llm-workspace");
        let cmd = r#"find ./skills -name "mcp-setup*" -o -name "*mcp*setup" 2>/dev/null | head -10"#;
        let paths = terminal_collect_command_path_candidates(&cwd, cmd, "git-bash");
        assert!(
            paths.iter().any(|p| p.to_string_lossy().contains("skills")),
            "expected ./skills to be collected"
        );
        assert!(
            !paths.iter().any(|p| {
                p.to_string_lossy()
                    .to_ascii_lowercase()
                    .replace('\\', "/")
                    .contains("dev/null")
            }),
            "expected /dev/null to be ignored"
        );
    }
}
