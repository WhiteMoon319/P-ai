fn remote_im_group_reply_focus_matches(
    state: &AppState,
    contact: &RemoteImContact,
    text: &str,
) -> bool {
    let pacing = effective_remote_im_group_reply_pacing(state, contact);
    let normalized = text.to_lowercase();
    pacing
        .focus_instructions
        .iter()
        .any(|phrase| normalized.contains(&phrase.to_lowercase()))
}

fn build_remote_im_group_reply_length_reminder(focus: bool, max_chars: u32) -> String {
    if focus {
        format!("[系统提醒]\n请认真回答，最多 {max_chars} 字。")
    } else {
        format!("[系统提醒]\n请在 {max_chars} 个字内进行回应。")
    }
}

fn effective_remote_im_group_reply_char_count(text: &str) -> usize {
    remote_im_strip_simple_markdown(text)
        .chars()
        .filter(|character| !character.is_whitespace())
        .count()
}

fn select_complete_remote_im_group_reply_within_budget(
    text: &str,
    max_chars: u32,
) -> Option<String> {
    let max_chars = max_chars as usize;
    if max_chars == 0 {
        return None;
    }
    if effective_remote_im_group_reply_char_count(text) <= max_chars {
        return Some(text.trim().to_string());
    }
    let mut count = 0usize;
    let mut last_boundary = None::<usize>;
    for (index, character) in text.char_indices() {
        if !character.is_whitespace() && !matches!(character, '*' | '_' | '`' | '#' | '>' | '~') {
            count = count.saturating_add(1);
        }
        if count > max_chars {
            break;
        }
        if matches!(character, '。' | '！' | '？' | '!' | '?' | '\n') {
            last_boundary = Some(index + character.len_utf8());
        }
    }
    last_boundary
        .and_then(|index| text.get(..index))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn remote_im_group_reply_safe_ack(max_chars: u32) -> Option<String> {
    ["收到。", "明白。", "好。"]
        .into_iter()
        .find(|text| effective_remote_im_group_reply_char_count(text) <= max_chars as usize)
        .map(ToOwned::to_owned)
}

fn enforce_remote_im_group_reply_length(text: &str, max_chars: u32) -> Option<String> {
    select_complete_remote_im_group_reply_within_budget(text, max_chars)
        .or_else(|| remote_im_group_reply_safe_ack(max_chars))
}

#[cfg(test)]
mod remote_im_group_reply_focus_tests {
    use super::*;

    #[test]
    fn group_reply_length_reminder_should_distinguish_focus() {
        assert_eq!(
            build_remote_im_group_reply_length_reminder(false, 20),
            "[系统提醒]\n请在 20 个字内进行回应。"
        );
        assert_eq!(
            build_remote_im_group_reply_length_reminder(true, 200),
            "[系统提醒]\n请认真回答，最多 200 字。"
        );
    }

    #[test]
    fn group_reply_length_gate_should_keep_complete_sentence_only() {
        assert_eq!(
            select_complete_remote_im_group_reply_within_budget(
                "第一句话很完整。第二句话会超过预算而且不能被硬切。",
                10,
            ),
            Some("第一句话很完整。".to_string())
        );
        assert_eq!(enforce_remote_im_group_reply_length("这是一条没有句号的超长回复", 4), Some("收到。".to_string()));
    }
}
