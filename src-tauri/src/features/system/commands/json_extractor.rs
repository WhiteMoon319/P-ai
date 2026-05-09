fn strip_json_code_fences(text: &str) -> String {
    text.replace("```json", "")
        .replace("```JSON", "")
        .replace("```", "")
        .trim()
        .to_string()
}

fn find_balanced_json_object_candidates(text: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    let mut in_string = false;
    let mut escaped = false;
    let mut depth = 0usize;
    let mut start_index: Option<usize> = None;

    for (index, ch) in text.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => {
                if depth == 0 {
                    start_index = Some(index);
                }
                depth += 1;
            }
            '}' => {
                if depth == 0 {
                    continue;
                }
                depth -= 1;
                if depth == 0 {
                    if let Some(start) = start_index.take() {
                        let end = index + ch.len_utf8();
                        if let Some(candidate) = text.get(start..end) {
                            candidates.push(candidate);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    candidates
}

fn find_json_object_candidates_from_each_start(text: &str) -> Vec<&str> {
    let mut candidates = Vec::new();
    for (start, ch) in text.char_indices() {
        if ch != '{' {
            continue;
        }
        let mut depth = 0usize;
        let mut in_string = false;
        let mut escaped = false;
        for (offset, ch) in text[start..].char_indices() {
            if in_string {
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '"' {
                    in_string = false;
                }
                continue;
            }
            match ch {
                '"' => in_string = true,
                '{' => depth += 1,
                '}' => {
                    if depth == 0 {
                        break;
                    }
                    depth -= 1;
                    if depth == 0 {
                        let end = start + offset + ch.len_utf8();
                        if let Some(candidate) = text.get(start..end) {
                            candidates.push(candidate);
                        }
                        break;
                    }
                }
                _ => {}
            }
        }
    }
    candidates
}

fn select_best_json_object_candidate(
    candidates: Vec<&str>,
    required_fields: &[&str],
    optional_fields: &[&str],
) -> Option<serde_json::Value> {
    let mut best: Option<(usize, serde_json::Value)> = None;
    for candidate in candidates {
        let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate) else {
            continue;
        };
        if !parsed.is_object() {
            continue;
        }
        let Some(object) = parsed.as_object() else {
            continue;
        };
        if !required_fields
            .iter()
            .all(|field| object.contains_key(*field))
        {
            continue;
        }
        let score = optional_fields
            .iter()
            .filter(|field| object.contains_key(**field))
            .count();
        if best
            .as_ref()
            .map(|(best_score, _)| score >= *best_score)
            .unwrap_or(true)
        {
            best = Some((score, parsed));
        }
    }

    best.map(|(_, value)| value)
}

fn extract_best_json_object_value(
    text: &str,
    separator: &str,
    required_fields: &[&str],
    optional_fields: &[&str],
) -> Option<serde_json::Value> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return None;
    }

    let json_part = if !separator.is_empty() && trimmed.contains(separator) {
        trimmed.split_once(separator)?.1.trim()
    } else {
        trimmed
    };
    let json_part = strip_json_code_fences(json_part);
    let candidates = find_balanced_json_object_candidates(&json_part);
    if let Some(value) =
        select_best_json_object_candidate(candidates, required_fields, optional_fields)
    {
        return Some(value);
    }

    select_best_json_object_candidate(
        find_json_object_candidates_from_each_start(&json_part),
        required_fields,
        optional_fields,
    )
}

#[cfg(test)]
mod json_extractor_tests {
    use super::*;

    #[test]
    fn extract_best_json_object_should_return_plain_object() {
        let raw = r#"{"question":"Which is the highest mountain in the world?","answer":"Mount Everest"}"#;
        let parsed = extract_best_json_object_value(raw, "---JSON---", &["question"], &["answer"])
            .expect("extract plain json");
        assert_eq!(parsed["answer"], "Mount Everest");
    }

    #[test]
    fn extract_best_json_object_should_support_direct_fenced_output() {
        let fence = "```";
        let raw = format!(
            "{}json\n{{\"summary\":\"归档摘要\",\"memoryActions\":[]}}\n{}",
            fence, fence
        );
        let parsed =
            extract_best_json_object_value(&raw, "---JSON---", &["memoryActions"], &["summary"])
                .expect("extract fenced json");
        assert_eq!(parsed["summary"], "归档摘要");
    }

    #[test]
    fn extract_best_json_object_should_pick_last_when_scores_tie() {
        let raw = r#"EXAMPLE JSON OUTPUT:
{"summary":"示例摘要","memoryActions":[]}

最终输出：
{"summary":"真实摘要","memoryActions":[]}"#;
        let parsed =
            extract_best_json_object_value(raw, "---JSON---", &["memoryActions"], &["summary"])
                .expect("extract last tied json");
        assert_eq!(parsed["summary"], "真实摘要");
    }

    #[test]
    fn extract_best_json_object_should_prefer_higher_score() {
        let raw = r#"{"memoryActions":[]}
{"summary":"真实摘要","openLoops":[],"memoryActions":[]}"#;
        let parsed = extract_best_json_object_value(
            raw,
            "---JSON---",
            &["memoryActions"],
            &["summary", "openLoops"],
        )
        .expect("extract highest scored json");
        assert_eq!(parsed["summary"], "真实摘要");
    }

    #[test]
    fn extract_best_json_object_should_handle_braces_inside_strings() {
        let raw = r#"说明 {"bad": true
{"summary":"包含 { 花括号 } 的文本","memoryActions":[]}"#;
        let parsed =
            extract_best_json_object_value(raw, "---JSON---", &["memoryActions"], &["summary"])
                .expect("extract json with braces in string");
        assert_eq!(parsed["summary"], "包含 { 花括号 } 的文本");
    }

    #[test]
    fn extract_best_json_object_should_use_separator_tail() {
        let raw = r#"{"summary":"分隔符前","memoryActions":[]}
---JSON---
{"summary":"分隔符后","memoryActions":[]}"#;
        let parsed =
            extract_best_json_object_value(raw, "---JSON---", &["memoryActions"], &["summary"])
                .expect("extract after separator");
        assert_eq!(parsed["summary"], "分隔符后");
    }
}
