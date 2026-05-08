fn memory_store_list_memories_visible_for_agent(
    data_path: &PathBuf,
    agent_id: &str,
    private_memory_enabled: bool,
) -> Result<Vec<MemoryEntry>, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return memory_store_list_memories(data_path);
    }
    let all = memory_store_list_memories(data_path)?;
    let filtered = all
        .into_iter()
        .filter(|m| match m.owner_agent_id.as_deref() {
            None => true,
            Some(owner) => private_memory_enabled && owner == target,
        })
        .collect::<Vec<_>>();
    Ok(filtered)
}

fn memory_store_list_memories_by_ids_visible_for_agent(
    data_path: &PathBuf,
    memory_ids: &[String],
    agent_id: &str,
    private_memory_enabled: bool,
) -> Result<Vec<MemoryEntry>, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return memory_store_list_memories_by_ids(data_path, memory_ids);
    }
    let filtered = memory_store_list_memories_by_ids(data_path, memory_ids)?
        .into_iter()
        .filter(|m| match m.owner_agent_id.as_deref() {
            None => true,
            Some(owner) => private_memory_enabled && owner == target,
        })
        .collect::<Vec<_>>();
    Ok(filtered)
}

fn memory_store_list_private_memories_by_agent(
    data_path: &PathBuf,
    agent_id: &str,
) -> Result<Vec<MemoryEntry>, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return Ok(Vec::new());
    }
    let all = memory_store_list_memories(data_path)?;
    Ok(all
        .into_iter()
        .filter(|m| m.owner_agent_id.as_deref() == Some(target))
        .collect::<Vec<_>>())
}

fn memory_store_count_private_memories_by_agent(
    data_path: &PathBuf,
    agent_id: &str,
) -> Result<usize, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return Ok(0);
    }
    let conn = memory_store_open(data_path)?;
    let count = conn
        .query_row(
            "SELECT COUNT(1) FROM memory_record WHERE owner_agent_id=?1",
            params![target],
            |row| row.get::<_, i64>(0),
        )
        .map_err(|err| format!("Count private memories by agent failed: {err}"))?;
    Ok(count.max(0) as usize)
}

fn memory_store_delete_memories_by_owner_agent_id(
    data_path: &PathBuf,
    agent_id: &str,
) -> Result<usize, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return Ok(0);
    }
    let mut conn = memory_store_open(data_path)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Begin owner memory delete transaction failed: {err}"))?;

    tx.execute(
        "DELETE FROM memory_fts WHERE item_id IN (SELECT id FROM memory_record WHERE owner_agent_id=?1)",
        params![target],
    )
    .map_err(|err| format!("Delete owner memory_fts failed: {err}"))?;
    let deleted = tx
        .execute("DELETE FROM memory_record WHERE owner_agent_id=?1", params![target])
        .map_err(|err| format!("Delete owner memory_record failed: {err}"))?;

    tx.commit()
        .map_err(|err| format!("Commit owner memory delete transaction failed: {err}"))?;
    if deleted > 0 {
        invalidate_memory_matcher_cache();
    }
    Ok(deleted)
}

fn memory_entry_allowed_for_profile(memory: &MemoryEntry) -> bool {
    matches!(
        memory.memory_type.trim().to_ascii_lowercase().as_str(),
        "knowledge" | "skill" | "event"
    )
}

fn profile_user_id_tag(user_id: &str) -> Option<String> {
    let trimmed = user_id.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(format!("user_id:{}", trimmed))
    }
}

fn memory_has_profile_shape_tags(memory: &MemoryEntry) -> bool {
    let has_profile = memory.tags.iter().any(|tag| tag == "profile");
    let has_user_id = memory.tags.iter().any(|tag| {
        tag.strip_prefix("user_id:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
    });
    let has_profile_attr = memory.tags.iter().any(|tag| {
        tag.strip_prefix("profile_attr:")
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_some()
    });
    has_profile && has_user_id && has_profile_attr
}

fn memory_entry_is_profile_memory(memory: &MemoryEntry) -> bool {
    memory_entry_allowed_for_profile(memory) && memory_has_profile_shape_tags(memory)
}

fn legacy_profile_attr_tag_for_memory(memory: &MemoryEntry) -> &'static str {
    match memory.memory_type.trim().to_ascii_lowercase().as_str() {
        "skill" => "profile_attr:skill",
        _ => "profile_attr:fact",
    }
}

fn memory_store_backfill_local_profile_tags_from_legacy_links(
    data_path: &PathBuf,
) -> Result<usize, String> {
    let conn = memory_store_open(data_path)?;
    let mut stmt = conn
        .prepare(
            "SELECT mr.id, mr.memory_type
             FROM profile_memory_link link
             JOIN memory_record mr ON mr.id = link.memory_id
             ORDER BY link.updated_at DESC, link.created_at DESC",
        )
        .map_err(|err| format!("Prepare backfill legacy profile links failed: {err}"))?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
            ))
        })
        .map_err(|err| format!("Query backfill legacy profile links failed: {err}"))?;
    let mut legacy_items = Vec::<(String, String)>::new();
    for row in rows {
        legacy_items.push(
            row.map_err(|err| format!("Read backfill legacy profile link row failed: {err}"))?,
        );
    }
    if legacy_items.is_empty() {
        return Ok(0);
    }

    let existing = memory_store_list_memories_by_ids(
        data_path,
        &legacy_items
            .iter()
            .map(|(memory_id, _)| memory_id.clone())
            .collect::<Vec<_>>(),
    )?
    .into_iter()
    .map(|memory| (memory.id.clone(), memory))
    .collect::<HashMap<String, MemoryEntry>>();

    let mut conn = memory_store_open(data_path)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Begin backfill legacy profile tags transaction failed: {err}"))?;
    let mut updated = 0usize;
    for (memory_id, memory_type) in legacy_items {
        let Some(memory) = existing.get(&memory_id) else {
            continue;
        };
        if !memory_entry_allowed_for_profile(memory) || memory_entry_is_profile_memory(memory) {
            continue;
        }
        let mut next_tags = memory.tags.clone();
        if !next_tags.iter().any(|tag| tag == "profile") {
            next_tags.push("profile".to_string());
        }
        if !next_tags.iter().any(|tag| tag.starts_with("user_id:")) {
            next_tags.push("user_id:0".to_string());
        }
        if !next_tags.iter().any(|tag| tag.starts_with("profile_attr:")) {
            let attr = match memory_type.trim().to_ascii_lowercase().as_str() {
                "skill" => "profile_attr:skill",
                _ => legacy_profile_attr_tag_for_memory(memory),
            };
            next_tags.push(attr.to_string());
        }
        let normalized = normalize_memory_keywords(&next_tags);
        memory_store_sync_tags(&tx, &memory_id, &normalized)?;
        memory_store_sync_memory_fts(&tx, &memory_id)?;
        updated += 1;
    }
    tx.commit()
        .map_err(|err| format!("Commit backfill legacy profile tags transaction failed: {err}"))?;
    if updated > 0 {
        invalidate_memory_matcher_cache();
    }
    Ok(updated)
}

fn memory_store_list_profile_memories_by_user_id_visible_for_agent(
    data_path: &PathBuf,
    user_id: &str,
    agent_id: &str,
    private_memory_enabled: bool,
    limit: usize,
) -> Result<Vec<MemoryEntry>, String> {
    let started_at = std::time::Instant::now();
    if user_id.trim() == "0" {
        let _ = memory_store_backfill_local_profile_tags_from_legacy_links(data_path);
    }
    let Some(user_id_tag) = profile_user_id_tag(user_id) else {
        runtime_log_info(format!(
            "[用户画像] 跳过，任务=fast_profile_lookup，user_id={}，agent_id={}，private_memory_enabled={}，requested_limit={}，reason=empty_user_id，elapsed_ms={}",
            user_id.trim(),
            agent_id.trim(),
            private_memory_enabled,
            limit,
            started_at.elapsed().as_millis()
        ));
        return Ok(Vec::new());
    };
    let conn = memory_store_open(data_path)?;
    let max_items = if limit == 0 { i64::MAX } else { limit as i64 };
    let target = agent_id.trim();
    let use_private_scope = if target.is_empty() || !private_memory_enabled {
        0i64
    } else {
        1i64
    };
    let sql = if target.is_empty() {
        "SELECT mr.id
         FROM memory_record mr
         WHERE EXISTS (
             SELECT 1
             FROM memory_tag_rel rel
             JOIN global_tag tag ON tag.id = rel.tag_id
             WHERE rel.memory_id = mr.id AND tag.name = 'profile'
         )
         AND EXISTS (
             SELECT 1
             FROM memory_tag_rel rel
             JOIN global_tag tag ON tag.id = rel.tag_id
             WHERE rel.memory_id = mr.id AND tag.name = ?1
         )
         AND EXISTS (
             SELECT 1
             FROM memory_tag_rel rel
             JOIN global_tag tag ON tag.id = rel.tag_id
             WHERE rel.memory_id = mr.id AND tag.name LIKE 'profile_attr:%'
         )
         ORDER BY mr.updated_at DESC
         LIMIT ?2"
    } else {
        "SELECT mr.id
         FROM memory_record mr
         WHERE EXISTS (
             SELECT 1
             FROM memory_tag_rel rel
             JOIN global_tag tag ON tag.id = rel.tag_id
             WHERE rel.memory_id = mr.id AND tag.name = 'profile'
         )
         AND EXISTS (
             SELECT 1
             FROM memory_tag_rel rel
             JOIN global_tag tag ON tag.id = rel.tag_id
             WHERE rel.memory_id = mr.id AND tag.name = ?1
         )
         AND EXISTS (
             SELECT 1
             FROM memory_tag_rel rel
             JOIN global_tag tag ON tag.id = rel.tag_id
             WHERE rel.memory_id = mr.id AND tag.name LIKE 'profile_attr:%'
         )
         AND (
             mr.owner_agent_id IS NULL
             OR (?2 = 1 AND mr.owner_agent_id = ?3)
         )
         ORDER BY mr.updated_at DESC
         LIMIT ?4"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|err| format!("Prepare list profile memories by user_id failed: {err}"))?;
    let mut ids = Vec::<String>::new();
    if target.is_empty() {
        let rows = stmt
            .query_map(params![user_id_tag, max_items], |row| row.get::<_, String>(0))
            .map_err(|err| format!("Query list profile memories by user_id failed: {err}"))?;
        for row in rows {
            ids.push(
                row.map_err(|err| {
                    format!("Read profile memory by user_id row failed: {err}")
                })?,
            );
        }
    } else {
        let rows = stmt
            .query_map(params![user_id_tag, use_private_scope, target, max_items], |row| {
                row.get::<_, String>(0)
            })
            .map_err(|err| format!("Query list profile memories by user_id failed: {err}"))?;
        for row in rows {
            ids.push(
                row.map_err(|err| {
                    format!("Read profile memory by user_id row failed: {err}")
                })?,
            );
        }
    }
    if ids.is_empty() {
        runtime_log_info(format!(
            "[用户画像] 完成，任务=fast_profile_lookup，user_id={}，agent_id={}，private_memory_enabled={}，requested_limit={}，result_count=0，elapsed_ms={}",
            user_id.trim(),
            agent_id.trim(),
            private_memory_enabled,
            limit,
            started_at.elapsed().as_millis()
        ));
        return Ok(Vec::new());
    }
    let mut memories =
        memory_store_list_memories_by_ids_visible_for_agent(data_path, &ids, agent_id, private_memory_enabled)?;
    memories.retain(memory_entry_is_profile_memory);
    let order = ids
        .into_iter()
        .enumerate()
        .map(|(idx, id)| (id, idx))
        .collect::<HashMap<String, usize>>();
    memories.sort_by_key(|memory| {
        order
            .get(memory.id.as_str())
            .copied()
            .unwrap_or(usize::MAX)
    });
    if limit != 0 && memories.len() > limit {
        memories.truncate(limit);
    }
    runtime_log_info(format!(
        "[用户画像] 完成，任务=fast_profile_lookup，user_id={}，agent_id={}，private_memory_enabled={}，requested_limit={}，result_count={}，elapsed_ms={}",
        user_id.trim(),
        agent_id.trim(),
        private_memory_enabled,
        limit,
        memories.len(),
        started_at.elapsed().as_millis()
    ));
    Ok(memories)
}

#[allow(dead_code)]
fn memory_store_upsert_profile_memory_links(
    data_path: &PathBuf,
    memory_ids: &[String],
    source: &str,
) -> Result<usize, String> {
    let mut conn = memory_store_open(data_path)?;
    let tx = conn
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|err| format!("Begin profile memory link transaction failed: {err}"))?;
    let now = now_iso();
    let normalized_source = if source.trim().eq_ignore_ascii_case("manual") {
        "manual"
    } else {
        "auto"
    };
    let mut linked_count = 0usize;
    for memory_id in memory_ids
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
    {
        let exists = tx
            .query_row(
                "SELECT 1 FROM memory_record WHERE id=?1 LIMIT 1",
                params![memory_id],
                |_| Ok(1i64),
            )
            .optional()
            .map_err(|err| format!("Check profile memory existence failed: {err}"))?
            .is_some();
        if !exists {
            continue;
        }
        let changed = tx
            .execute(
                "INSERT INTO profile_memory_link(id, memory_id, source, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(memory_id) DO UPDATE SET source=excluded.source, updated_at=excluded.updated_at",
                params![Uuid::new_v4().to_string(), memory_id, normalized_source, now, now],
            )
            .map_err(|err| format!("Upsert profile_memory_link failed: {err}"))?;
        if changed > 0 {
            linked_count += 1;
        }
    }
    tx.commit()
        .map_err(|err| format!("Commit profile memory link transaction failed: {err}"))?;
    Ok(linked_count)
}

#[allow(dead_code)]
fn memory_store_list_profile_memories_visible_for_agent(
    data_path: &PathBuf,
    agent_id: &str,
    private_memory_enabled: bool,
    limit: usize,
) -> Result<Vec<MemoryEntry>, String> {
    memory_store_list_profile_memories_by_user_id_visible_for_agent(
        data_path,
        "0",
        agent_id,
        private_memory_enabled,
        limit,
    )
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentPrivateMemoryExportResult {
    path: String,
    count: usize,
}

fn memory_store_export_agent_private_memories(
    data_path: &PathBuf,
    agent_id: &str,
) -> Result<AgentPrivateMemoryExportResult, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return Err("agent_id is required".to_string());
    }
    let memories = memory_store_list_private_memories_by_agent(data_path, target)?;
    let mut exported_memories = Vec::with_capacity(memories.len());
    for mut memory in memories {
        // Persona-private export is intentionally rewritten as global memories,
        // so users can import into global scope directly.
        memory.owner_agent_id = None;
        exported_memories.push(memory);
    }

    let ts = OffsetDateTime::now_utc().unix_timestamp().to_string();
    let export_dir = app_root_from_data_path(data_path).join("backups").join(ts);
    fs::create_dir_all(&export_dir)
        .map_err(|err| format!("Create private memory backup dir failed: {err}"))?;
    let safe_agent = target.replace('\\', "_").replace('/', "_").replace(':', "_");
    let file_name = format!("agent-{safe_agent}-private-memories.json");
    let path = export_dir.join(file_name);
    let payload = serde_json::json!({
        "version": 1,
        "exportedAt": now_iso(),
        "agentId": target,
        "memories": exported_memories,
    });
    let body = serde_json::to_string_pretty(&payload)
        .map_err(|err| format!("Serialize private memory export payload failed: {err}"))?;
    fs::write(&path, body)
        .map_err(|err| format!("Write private memory export file failed: {err}"))?;

    Ok(AgentPrivateMemoryExportResult {
        path: path.to_string_lossy().to_string(),
        count: payload
            .get("memories")
            .and_then(|v| v.as_array())
            .map(|v| v.len())
            .unwrap_or(0),
    })
}

fn memory_store_import_memories_for_agent(
    data_path: &PathBuf,
    incoming: &[MemoryEntry],
    agent_id: &str,
) -> Result<MemoryStoreImportStats, String> {
    let target = agent_id.trim();
    if target.is_empty() {
        return Err("agent_id is required".to_string());
    }
    let mut rewritten = Vec::<MemoryEntry>::with_capacity(incoming.len());
    for item in incoming {
        let mut next = item.clone();
        next.owner_agent_id = Some(target.to_string());
        rewritten.push(next);
    }
    memory_store_import_memories(data_path, &rewritten)
}
