use unicode_segmentation::UnicodeSegmentation;

// 群聊长度门改写（默认禁用，保留待重新启用）。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MultilingualTextUnitCount {
    east_asian_graphemes: usize,
    word_units: usize,
    emoji_graphemes: usize,
    contains_japanese_kana: bool,
}

#[allow(dead_code)]
impl MultilingualTextUnitCount {
    fn total(self) -> usize {
        self.east_asian_graphemes
            .saturating_add(self.word_units)
            .saturating_add(self.emoji_graphemes)
    }
}

#[allow(dead_code)]
fn remote_im_is_han_scalar(code_point: u32) -> bool {
    matches!(
        code_point,
        0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2FA1F
    )
}

#[allow(dead_code)]
fn remote_im_is_japanese_kana_scalar(code_point: u32) -> bool {
    matches!(
        code_point,
        0x3040..=0x309F | 0x30A0..=0x30FF | 0x31F0..=0x31FF | 0xFF65..=0xFF9F
    )
}

#[allow(dead_code)]
fn remote_im_is_hangul_scalar(code_point: u32) -> bool {
    matches!(
        code_point,
        0x1100..=0x11FF | 0x3130..=0x318F | 0xA960..=0xA97F | 0xAC00..=0xD7AF | 0xD7B0..=0xD7FF
    )
}

#[allow(dead_code)]
fn remote_im_is_east_asian_text_scalar(code_point: u32) -> bool {
    remote_im_is_han_scalar(code_point)
        || remote_im_is_japanese_kana_scalar(code_point)
        || remote_im_is_hangul_scalar(code_point)
}

#[allow(dead_code)]
fn remote_im_is_emoji_scalar(code_point: u32) -> bool {
    matches!(
        code_point,
        0x1F1E6..=0x1F1FF
            | 0x1F300..=0x1FAFF
            | 0x2600..=0x26FF
            | 0x2700..=0x27BF
            | 0x00A9
            | 0x00AE
            | 0x203C
            | 0x2049
            | 0x2122
            | 0x2139
            | 0x3030
            | 0x303D
            | 0x3297
            | 0x3299
    )
}

#[allow(dead_code)]
fn count_remote_im_multilingual_text_units(text: &str) -> MultilingualTextUnitCount {
    let normalized = remote_im_strip_simple_markdown(text);
    let mut count = MultilingualTextUnitCount::default();
    let mut word_text = String::with_capacity(normalized.len());
    for grapheme in normalized.graphemes(true) {
        let mut has_east_asian_text = false;
        let mut has_japanese_kana = false;
        let mut has_emoji = false;
        for character in grapheme.chars() {
            let code_point = character as u32;
            has_east_asian_text |= remote_im_is_east_asian_text_scalar(code_point);
            has_japanese_kana |= remote_im_is_japanese_kana_scalar(code_point);
            has_emoji |= remote_im_is_emoji_scalar(code_point);
        }
        if has_east_asian_text {
            count.east_asian_graphemes = count.east_asian_graphemes.saturating_add(1);
            count.contains_japanese_kana |= has_japanese_kana;
            word_text.push(' ');
        } else if has_emoji {
            count.emoji_graphemes = count.emoji_graphemes.saturating_add(1);
            word_text.push(' ');
        } else {
            word_text.push_str(grapheme);
        }
    }
    count.word_units = word_text.unicode_words().count();
    count
}

#[allow(dead_code)]
fn remote_im_multilingual_text_units(text: &str) -> usize {
    count_remote_im_multilingual_text_units(text).total()
}

#[allow(dead_code)]
fn remote_im_multilingual_text_units_exceed_ratio(
    text: &str,
    configured_limit: u32,
    numerator: u32,
    denominator: u32,
) -> bool {
    if configured_limit == 0 || denominator == 0 {
        return false;
    }
    let threshold = u64::from(configured_limit)
        .saturating_mul(u64::from(numerator))
        / u64::from(denominator);
    u64::try_from(remote_im_multilingual_text_units(text))
        .map(|units| units > threshold)
        .unwrap_or(true)
}

#[cfg(test)]
mod remote_im_multilingual_text_units_tests {
    use super::*;

    #[test]
    fn multilingual_units_should_count_chinese_graphemes_without_punctuation() {
        let count = count_remote_im_multilingual_text_units("**打开冰箱门，A1。**");
        assert_eq!(count.east_asian_graphemes, 5);
        assert_eq!(count.word_units, 1);
        assert_eq!(count.emoji_graphemes, 0);
        assert_eq!(count.total(), 6);
    }

    #[test]
    fn multilingual_units_should_count_japanese_graphemes() {
        let count = count_remote_im_multilingual_text_units("冷蔵庫を開けて、OKです。");
        assert_eq!(count.east_asian_graphemes, 9);
        assert_eq!(count.word_units, 1);
        assert!(count.contains_japanese_kana);
        assert_eq!(count.total(), 10);
    }

    #[test]
    fn multilingual_units_should_count_english_unicode_words() {
        assert_eq!(
            remote_im_multilingual_text_units("I'll check the OpenAI v4 response."),
            6
        );
    }

    #[test]
    fn multilingual_units_should_add_mixed_language_words_and_emoji() {
        assert_eq!(remote_im_multilingual_text_units("请 check this issue 😊"), 5);
    }

    #[test]
    fn multilingual_units_should_rewrite_only_above_two_hundred_percent() {
        assert!(!remote_im_multilingual_text_units_exceed_ratio(
            &"字".repeat(40),
            20,
            2,
            1,
        ));
        assert!(remote_im_multilingual_text_units_exceed_ratio(
            &"字".repeat(41),
            20,
            2,
            1,
        ));
    }
}
