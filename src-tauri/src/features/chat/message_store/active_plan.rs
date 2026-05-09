const ACTIVE_PLAN_STATUS_IN_PROGRESS: &str = "in_progress";
const ACTIVE_PLAN_STATUS_COMPLETED: &str = "completed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ActivePlanRecord {
    plan_id: String,
    source_message_id: String,
    status: String,
    #[serde(default)]
    path: String,
    created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    completion_text: Option<String>,
}

fn encode_active_plan_record(record: &ActivePlanRecord) -> Result<String, String> {
    serde_json::to_string(record)
        .map(|value| format!("{value}\n"))
        .map_err(|err| format!("序列化执行中计划失败: {err}"))
}

fn read_active_plan_records(path: &PathBuf) -> Result<Vec<ActivePlanRecord>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(path).map_err(|err| {
        format!(
            "读取执行中计划文件失败，path={}，error={err}",
            path.display()
        )
    })?;
    let mut records = Vec::<ActivePlanRecord>::new();
    for (index, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let record = serde_json::from_str::<ActivePlanRecord>(trimmed).map_err(|err| {
            format!(
                "解析执行中计划失败，path={}，line={}，error={err}",
                path.display(),
                index + 1
            )
        })?;
        if record.path.trim().is_empty() {
            continue;
        }
        records.push(record);
    }
    Ok(records)
}

fn write_active_plan_records(path: &PathBuf, records: &[ActivePlanRecord]) -> Result<(), String> {
    let mut content = String::new();
    for record in records {
        content.push_str(&encode_active_plan_record(record)?);
    }
    write_message_store_text_atomic(path, "jsonl.tmp", &content, "执行中计划")
}

fn append_active_plan_record(path: &PathBuf, record: &ActivePlanRecord) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "创建执行中计划目录失败，path={}，error={err}",
                parent.display()
            )
        })?;
    }
    let line = encode_active_plan_record(record)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| {
            format!(
                "打开执行中计划文件失败，path={}，error={err}",
                path.display()
            )
        })?;
    use std::io::Write as _;
    file.write_all(line.as_bytes()).map_err(|err| {
        format!(
            "追加执行中计划失败，path={}，error={err}",
            path.display()
        )
    })
}

fn active_plan_records_in_progress(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Vec<ActivePlanRecord>, String> {
    let paths = message_store_paths(data_path, conversation_id)?;
    Ok(read_active_plan_records(&paths.active_plans_file)?
        .into_iter()
        .rev()
        .filter(|record| record.status.trim() == ACTIVE_PLAN_STATUS_IN_PROGRESS)
        .collect())
}

pub(super) fn active_plan_append_in_progress(
    data_path: &PathBuf,
    conversation_id: &str,
    source_message_id: &str,
    path: &str,
) -> Result<(), String> {
    let paths = message_store_paths(data_path, conversation_id)?;
    let record = ActivePlanRecord {
        plan_id: Uuid::new_v4().to_string(),
        source_message_id: source_message_id.trim().to_string(),
        status: ACTIVE_PLAN_STATUS_IN_PROGRESS.to_string(),
        path: path.trim().to_string(),
        created_at: now_iso(),
        completed_at: None,
        completion_text: None,
    };
    if record.source_message_id.is_empty() {
        return Err("sourceMessageId 为空，无法写入执行中计划。".to_string());
    }
    if record.path.is_empty() {
        return Err("计划路径为空，无法写入执行中计划。".to_string());
    }
    append_active_plan_record(&paths.active_plans_file, &record)?;
    Ok(())
}

pub(super) fn active_plan_complete_by_path(
    data_path: &PathBuf,
    conversation_id: &str,
    path: &str,
    completion_text: Option<&str>,
) -> Result<bool, String> {
    let normalized_path = path.trim();
    if normalized_path.is_empty() {
        return Err("计划路径为空，无法完成执行中计划。".to_string());
    }
    let paths = message_store_paths(data_path, conversation_id)?;
    let mut records = read_active_plan_records(&paths.active_plans_file)?;
    let Some(index) = records
        .iter()
        .rposition(|record| {
            record.status.trim() == ACTIVE_PLAN_STATUS_IN_PROGRESS
                && record.path.trim().eq_ignore_ascii_case(normalized_path)
        })
    else {
        return Ok(false);
    };
    records[index].status = ACTIVE_PLAN_STATUS_COMPLETED.to_string();
    records[index].completed_at = Some(now_iso());
    records[index].completion_text = completion_text
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    write_active_plan_records(&paths.active_plans_file, &records)?;
    Ok(true)
}

pub(super) fn active_plan_prompt_block(
    data_path: &PathBuf,
    conversation_id: &str,
) -> Result<Option<String>, String> {
    let records = active_plan_records_in_progress(data_path, conversation_id)?;
    if records.is_empty() {
        return Ok(None);
    }
    let mut lines = Vec::<String>::new();
    lines.push("<active_plans>".to_string());
    lines.push("以下为用户已同意且正在执行的计划文件。它们必须持续纳入上下文；完成后调用 plan(action=complete) 并传入对应 path。".to_string());
    for (index, record) in records.iter().enumerate() {
        lines.push(format!("<active_plan index=\"{}\">", index + 1));
        lines.push(record.path.trim().to_string());
        lines.push("</active_plan>".to_string());
    }
    lines.push("</active_plans>".to_string());
    Ok(Some(lines.join("\n")))
}

#[cfg(test)]
#[test]
fn read_active_plan_records_should_skip_legacy_record_without_path() {
    let root = std::env::temp_dir().join(format!("eca-active-plan-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).expect("create temp dir");
    let file = root.join("active_plans.jsonl");
    fs::write(
        &file,
        concat!(
            "{\"planId\":\"legacy\",\"sourceMessageId\":\"msg-1\",\"status\":\"in_progress\",\"createdAt\":\"2026-01-01T00:00:00Z\"}\n",
            "{\"planId\":\"valid\",\"sourceMessageId\":\"msg-2\",\"status\":\"in_progress\",\"path\":\"C:/plan.md\",\"createdAt\":\"2026-01-01T00:00:00Z\"}\n"
        ),
    )
    .expect("write active plans");

    let records = read_active_plan_records(&file).expect("read active plans");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].plan_id, "valid");
    assert_eq!(records[0].path, "C:/plan.md");

    let _ = fs::remove_dir_all(root);
}

#[cfg(test)]
#[test]
fn active_plan_records_in_progress_should_return_newest_first() {
    let root = std::env::temp_dir().join(format!("eca-active-plan-order-{}", Uuid::new_v4()));
    let conversation_id = "conv-active-plan-order";
    let paths = message_store_paths(&root, conversation_id).expect("message store paths");
    fs::create_dir_all(paths.active_plans_file.parent().expect("active plans dir"))
        .expect("create active plans dir");
    fs::write(
        &paths.active_plans_file,
        concat!(
            "{\"planId\":\"old\",\"sourceMessageId\":\"msg-1\",\"status\":\"in_progress\",\"path\":\"C:/old.md\",\"createdAt\":\"2026-01-01T00:00:00Z\"}\n",
            "{\"planId\":\"done\",\"sourceMessageId\":\"msg-2\",\"status\":\"completed\",\"path\":\"C:/done.md\",\"createdAt\":\"2026-01-01T00:00:01Z\"}\n",
            "{\"planId\":\"new\",\"sourceMessageId\":\"msg-3\",\"status\":\"in_progress\",\"path\":\"C:/new.md\",\"createdAt\":\"2026-01-01T00:00:02Z\"}\n"
        ),
    )
    .expect("write active plans");

    let records =
        active_plan_records_in_progress(&root, conversation_id).expect("read active plans");
    assert_eq!(records.len(), 2);
    assert_eq!(records[0].plan_id, "new");
    assert_eq!(records[1].plan_id, "old");

    let _ = fs::remove_dir_all(root);
}
