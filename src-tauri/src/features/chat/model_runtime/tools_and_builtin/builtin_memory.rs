fn normalize_memory_keywords(raw: &[String]) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for item in raw {
        let trimmed = item.trim();
        let v = if let Some((_, raw_user_id)) = trimmed.split_once(':') {
            if trimmed
                .get(..8)
                .map(|prefix| prefix.eq_ignore_ascii_case("user_id:"))
                .unwrap_or(false)
            {
                let user_id = raw_user_id.trim();
                if user_id.is_empty() {
                    continue;
                }
                format!("user_id:{}", user_id)
            } else {
                trimmed.to_lowercase()
            }
        } else {
            trimmed.to_lowercase()
        };
        if v.chars().count() < 2 {
            continue;
        }
        if !out.iter().any(|x| x == &v) {
            out.push(v);
        }
        if out.len() >= 12 {
            break;
        }
    }
    out
}

fn memory_contains_sensitive(content: &str, keywords: &[String]) -> bool {
    let mut full = content.to_lowercase();
    if !keywords.is_empty() {
        full.push('\n');
        full.push_str(&keywords.join(" ").to_lowercase());
    }
    let danger_tokens = [
        "password",
        "passwd",
        "api key",
        "apikey",
        "token",
        "secret",
        "private key",
        "sk-",
        "ssh-rsa",
        "验证码",
        "密码",
        "密钥",
        "身份证",
        "银行卡",
        "cvv",
    ];
    danger_tokens.iter().any(|token| full.contains(token))
}

#[derive(Debug, Clone)]
struct MemorySaveDraft {
    memory_type: String,
    judgment: String,
    reasoning: String,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct MemorySaveUpsertItemResult {
    saved: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
}

fn parse_memory_save_draft(
    memory_type_raw: &str,
    judgment: &str,
    reasoning: &str,
    tags_raw: Vec<String>,
) -> Result<MemorySaveDraft, String> {
    let judgment = judgment.trim();
    if judgment.is_empty() {
        return Err("memory_save.judgment is required".to_string());
    }
    let memory_type = memory_store_normalize_memory_type(memory_type_raw)?;
    let tags = normalize_memory_keywords(&tags_raw);
    if tags.is_empty() {
        return Err("memory_save.tags must contain at least one valid tag".to_string());
    }
    if memory_contains_sensitive(judgment, &tags) {
        return Err("memory_save 含敏感内容，已阻止写入".to_string());
    }
    Ok(MemorySaveDraft {
        memory_type,
        judgment: judgment.to_string(),
        reasoning: clean_text(reasoning),
        tags,
    })
}

#[cfg(test)]
mod builtin_memory_tests {
    use super::*;

    #[test]
    fn normalize_memory_keywords_should_preserve_user_id_payload() {
        let tags = normalize_memory_keywords(&vec![
            " Profile ".to_string(),
            "USER_ID:o9cq80-MfLBeC-BBD-hStiFtlJSk@im.wechat".to_string(),
            "PROFILE_ATTR:Fact".to_string(),
        ]);
        assert_eq!(
            tags,
            vec![
                "profile".to_string(),
                "user_id:o9cq80-MfLBeC-BBD-hStiFtlJSk@im.wechat".to_string(),
                "profile_attr:fact".to_string(),
            ]
        );
    }
}

#[derive(Debug, Clone)]
struct MemoryAgentContext {
    owner_agent_id: Option<String>,
    effective_agent_id: String,
    private_memory_enabled: bool,
}

fn build_memory_agent_context(
    agent_id: &str,
    private_memory_enabled: bool,
) -> Result<MemoryAgentContext, String> {
    let effective_agent_id = agent_id.trim();
    if effective_agent_id.is_empty() {
        return Err("缺少当前人格 ID，无法读写记忆。".to_string());
    }
    Ok(MemoryAgentContext {
        owner_agent_id: private_memory_enabled.then_some(effective_agent_id.to_string()),
        effective_agent_id: effective_agent_id.to_string(),
        private_memory_enabled,
    })
}

fn memory_agent_context_from_agent(agent: &AgentProfile) -> Result<MemoryAgentContext, String> {
    if agent.is_built_in_user {
        return Err(format!("当前人格不支持读写记忆：agent_id={}", agent.id));
    }
    build_memory_agent_context(&agent.id, agent.private_memory_enabled)
}

fn upsert_memories(
    app_state: &AppState,
    memory_context: &MemoryAgentContext,
    drafts: &[MemorySaveDraft],
) -> Result<(Vec<MemorySaveUpsertItemResult>, usize), String> {
    let inputs = drafts
        .iter()
        .map(|d| MemoryDraftInput {
            memory_type: d.memory_type.clone(),
            judgment: d.judgment.clone(),
            reasoning: d.reasoning.clone(),
            tags: d.tags.clone(),
            owner_agent_id: memory_context.owner_agent_id.clone(),
        })
        .collect::<Vec<_>>();
    let (results, total_memories) = memory_store_upsert_drafts(&app_state.data_path, &inputs)?;
    let loop_started_at = std::time::Instant::now();
    for (draft, result) in drafts.iter().zip(results.iter()) {
        let elapsed_ms = loop_started_at
            .elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
        let trigger = if result.id.is_some() {
            "result.id=Some"
        } else {
            "result.id=None"
        };
        let id_text = result.id.as_deref().unwrap_or("none");
        eprintln!(
            "[简单记忆回灌] 任务名=简单记忆回灌 状态=完成 触发条件={} id={} memory_type={} tags={} judgment_len={} tags_count={} 耗时毫秒={}",
            trigger,
            id_text,
            draft.memory_type,
            draft.tags.join(","),
            draft.judgment.chars().count(),
            draft.tags.len(),
            elapsed_ms
        );
    }
    Ok((results, total_memories))
}

fn builtin_memory_save(
    app_state: &AppState,
    memory_context: &MemoryAgentContext,
    args: Value,
) -> Result<Value, String> {
    let memory_type = args
        .get("memory_type")
        .or_else(|| args.get("memoryType"))
        .and_then(Value::as_str)
        .ok_or_else(|| "memory_save.memoryType is required".to_string())?;
    let judgment = args
        .get("judgment")
        .and_then(Value::as_str)
        .ok_or_else(|| "memory_save.judgment is required".to_string())?;
    let reasoning = args
        .get("reasoning")
        .and_then(Value::as_str)
        .unwrap_or("");
    let tags_raw = args
        .get("tags")
        .and_then(Value::as_array)
        .ok_or_else(|| "memory_save.tags is required".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let draft = parse_memory_save_draft(memory_type, judgment, reasoning, tags_raw)?;
    if memory_contains_sensitive(&draft.judgment, &draft.tags) {
        eprintln!(
            "[简单记忆回灌] 任务名=简单记忆回灌 状态=跳过 触发条件=敏感内容检测 id=none memory_type={} tags={} judgment_len={} tags_count={} 耗时毫秒=0",
            draft.memory_type,
            draft.tags.join(","),
            draft.judgment.chars().count(),
            draft.tags.len()
        );
        return Ok(serde_json::json!({
          "ok": false,
          "saved": false,
          "id": Value::Null,
          "tags": draft.tags,
          "updatedAt": Value::Null,
          "reason": "memory_save 含敏感内容，已阻止写入",
          "totalMemories": Value::Null
        }));
    }
    let (results, total_memories) = upsert_memories(app_state, memory_context, &[draft])?;
    let first = results
        .into_iter()
        .next()
        .ok_or_else(|| "memory_save failed to produce result".to_string())?;
    Ok(serde_json::json!({
      "ok": first.saved,
      "saved": first.saved,
      "id": first.id,
      "tags": first.tags,
      "updatedAt": first.updated_at,
      "reason": first.reason,
      "totalMemories": total_memories
    }))
}

fn builtin_recall(
    app_state: &AppState,
    memory_context: &MemoryAgentContext,
    query: &str,
) -> Result<Value, String> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        return Err("recall.query is required".to_string());
    }
    let memories = memory_store_list_memories_visible_for_agent(
        &app_state.data_path,
        &memory_context.effective_agent_id,
        memory_context.private_memory_enabled,
    )?;

    let recall_hit_ids = memory_recall_hit_ids(&app_state.data_path, &memories, trimmed_query);
    let latest_recall_ids = memory_board_ids_from_current_hits(&recall_hit_ids, 7);
    let memory_board = build_memory_board_xml_from_recall_ids(&memories, &latest_recall_ids)
        .unwrap_or_default();

    Ok(serde_json::json!({
      "ok": true,
      "memoryBoard": memory_board,
      "count": latest_recall_ids.len()
    }))
}

