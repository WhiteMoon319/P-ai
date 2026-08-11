pub(crate) fn remote_im_group_reply_focus_matches(
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

pub(crate) fn build_remote_im_group_reply_length_reminder(focus: bool, max_chars: u32) -> String {
    let unit_rule = "中文/日文/韩文按可见字形计 1，英语等按 Unicode 单词计 1，数字词和 Emoji 各计 1，标点与空白不计。";
    if focus {
        format!("[系统提醒]\n请认真回答，最多 {max_chars} 个有效文本单位。{unit_rule}")
    } else {
        format!("[系统提醒]\n请在 {max_chars} 个有效文本单位内进行回应。{unit_rule}")
    }
}

pub(crate) fn effective_remote_im_group_reply_char_count(text: &str) -> usize {
    remote_im_strip_simple_markdown(text)
        .chars()
        .filter(|character| !character.is_whitespace())
        .count()
}

#[cfg(test)]
pub(crate) mod remote_im_group_reply_focus_tests {
    use super::*;

    #[test]
    fn group_reply_length_reminder_should_distinguish_focus() {
        assert_eq!(
            build_remote_im_group_reply_length_reminder(false, 20),
            "[系统提醒]\n请在 20 个有效文本单位内进行回应。中文/日文/韩文按可见字形计 1，英语等按 Unicode 单词计 1，数字词和 Emoji 各计 1，标点与空白不计。"
        );
        assert_eq!(
            build_remote_im_group_reply_length_reminder(true, 200),
            "[系统提醒]\n请认真回答，最多 200 个有效文本单位。中文/日文/韩文按可见字形计 1，英语等按 Unicode 单词计 1，数字词和 Emoji 各计 1，标点与空白不计。"
        );
    }

}
