#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteUtf8TextFileInput {
    path: String,
    text: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteBase64FileInput {
    path: String,
    bytes_base64: String,
}

fn normalize_export_path(raw: &str) -> Result<PathBuf, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("导出路径不能为空".to_string());
    }
    if std::path::Path::new(trimmed)
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("导出路径不允许包含上级目录跳转".to_string());
    }
    Ok(PathBuf::from(trimmed))
}

fn ensure_share_export_parent_dir(path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)
                .map_err(|err| format!("创建导出目录失败: {err}"))?;
        }
    }
    Ok(())
}


