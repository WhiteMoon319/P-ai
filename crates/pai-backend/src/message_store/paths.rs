use uuid::Uuid;

use std::fs;
use std::path::{Path, PathBuf};

use super::*;

/// 数据布局目录常量（从 src-tauri app_data_layout.rs 迁入）。
pub const LAYOUT_DIR_CHAT: &str = "chat";
pub const LAYOUT_DIR_CHAT_CONVERSATIONS: &str = "conversations";
pub const CHAT_METADATA_DB_FILE_NAME: &str = "chat_metadata.db";

/// 应用根下 chat 目录（从 src-tauri app_data_layout.rs 迁入）。
pub fn app_layout_chat_dir(path: &Path) -> PathBuf {
    crate::memory::store::app_root_from_data_path(path).join(LAYOUT_DIR_CHAT)
}

/// 会话目录（从 src-tauri app_data_layout.rs 迁入）。
pub fn app_layout_chat_conversations_dir(path: &Path) -> PathBuf {
    app_layout_chat_dir(path).join(LAYOUT_DIR_CHAT_CONVERSATIONS)
}

/// 单会话 JSON 路径（从 src-tauri app_data_layout.rs 迁入）。
pub fn app_layout_chat_conversation_path(path: &Path, conversation_id: &str) -> PathBuf {
    app_layout_chat_conversations_dir(path).join(format!("{conversation_id}.json"))
}

// chat_metadata_store_db_path / chat_metadata_store_is_ready 由 sqlite.rs 提供（权威版）。

pub const MESSAGE_STORE_MANIFEST_FILE_NAME: &str = "manifest.json";
pub const MESSAGE_STORE_META_FILE_NAME: &str = "meta.json";
pub const MESSAGE_STORE_MESSAGES_FILE_NAME: &str = "messages.jsonl";
pub const MESSAGE_STORE_ACTIVE_PLANS_FILE_NAME: &str = "active_plans.jsonl";
pub const MESSAGE_STORE_INDEX_FILE_NAME: &str = "messages.idx.json";
pub const MESSAGE_STORE_BLOCKS_DIR_NAME: &str = "blocks";
pub const MESSAGE_STORE_BLOBS_DIR_NAME: &str = "blobs";

#[cfg(target_os = "windows")]
use std::os::windows::ffi::OsStrExt as _;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageStorePaths {
    pub data_path: PathBuf,
    pub conversation_id: String,
    pub legacy_conversation_file: PathBuf,
    pub shard_dir: PathBuf,
    pub manifest_file: PathBuf,
    pub meta_file: PathBuf,
    pub messages_file: PathBuf,
    pub active_plans_file: PathBuf,
    pub index_file: PathBuf,
    pub blocks_dir: PathBuf,
    pub blobs_dir: PathBuf,
}

impl MessageStorePaths {
    pub fn is_local_chat_conversation(&self) -> bool {
        self.shard_dir.parent() == Some(app_layout_chat_conversations_dir(&self.data_path).as_path())
    }

    pub fn is_v3_ready(&self) -> Result<bool, String> {
        Ok(self.is_local_chat_conversation() && chat_metadata_store_is_ready(&self.data_path)?)
    }
}

pub fn message_store_is_v3_ready(paths: &MessageStorePaths) -> Result<bool, String> {
    paths.is_v3_ready()
}

pub fn message_store_paths(
    data_path: &Path,
    conversation_id: &str,
) -> Result<MessageStorePaths, String> {
    if conversation_id.trim().is_empty() {
        return Err("会话 ID 为空，无法构造消息存储路径".to_string());
    }
    if conversation_id != conversation_id.trim() {
        return Err(format!(
            "会话 ID 不能包含首尾空白，conversation_id={conversation_id}"
        ));
    }
    validate_message_store_conversation_id(conversation_id)?;
    let shard_dir = app_layout_chat_conversations_dir(data_path).join(conversation_id);
    message_store_paths_for_shard_dir(
        data_path,
        conversation_id,
        shard_dir,
        app_layout_chat_conversation_path(data_path, conversation_id),
    )
}

pub fn message_store_paths_for_shard_dir(
    data_path: &Path,
    conversation_id: &str,
    shard_dir: PathBuf,
    legacy_conversation_file: PathBuf,
) -> Result<MessageStorePaths, String> {
    if conversation_id.trim().is_empty() {
        return Err("会话 ID 为空，无法构造消息存储路径".to_string());
    }
    if conversation_id != conversation_id.trim() {
        return Err(format!(
            "会话 ID 不能包含首尾空白，conversation_id={conversation_id}"
        ));
    }
    validate_message_store_conversation_id(conversation_id)?;
    Ok(MessageStorePaths {
        data_path: data_path.to_path_buf(),
        conversation_id: conversation_id.to_string(),
        legacy_conversation_file,
        manifest_file: shard_dir.join(MESSAGE_STORE_MANIFEST_FILE_NAME),
        meta_file: shard_dir.join(MESSAGE_STORE_META_FILE_NAME),
        messages_file: shard_dir.join(MESSAGE_STORE_MESSAGES_FILE_NAME),
        active_plans_file: shard_dir.join(MESSAGE_STORE_ACTIVE_PLANS_FILE_NAME),
        index_file: shard_dir.join(MESSAGE_STORE_INDEX_FILE_NAME),
        blocks_dir: shard_dir.join(MESSAGE_STORE_BLOCKS_DIR_NAME),
        blobs_dir: shard_dir.join(MESSAGE_STORE_BLOBS_DIR_NAME),
        shard_dir,
    })
}

pub fn validate_message_store_conversation_id(conversation_id: &str) -> Result<(), String> {
    if conversation_id.contains('/') || conversation_id.contains('\\') {
        return Err(format!(
            "会话 ID 不能包含路径分隔符，conversation_id={conversation_id}"
        ));
    }
    if conversation_id
        .chars()
        .any(|ch| ch.is_control() || matches!(ch, '<' | '>' | ':' | '"' | '|' | '?' | '*'))
    {
        return Err(format!(
            "会话 ID 不能包含 Windows 文件名非法字符，conversation_id={conversation_id}"
        ));
    }
    if conversation_id.ends_with([' ', '.']) {
        return Err(format!(
            "会话 ID 不能以 Windows 不稳定文件名字符结尾，conversation_id={conversation_id}"
        ));
    }
    let mut components = std::path::Path::new(conversation_id).components();
    let Some(component) = components.next() else {
        return Err("会话 ID 为空，无法构造消息存储路径".to_string());
    };
    if components.next().is_some() || !matches!(component, std::path::Component::Normal(_)) {
        return Err(format!(
            "会话 ID 不能包含路径组件，conversation_id={conversation_id}"
        ));
    }
    let upper = conversation_id.to_ascii_uppercase();
    let base_name = upper.split('.').next().unwrap_or(upper.as_str());
    let reserved = matches!(base_name, "CON" | "PRN" | "AUX" | "NUL")
        || base_name
            .strip_prefix("COM")
            .and_then(|suffix| suffix.parse::<u8>().ok())
            .is_some_and(|value| (1..=9).contains(&value))
        || base_name
            .strip_prefix("LPT")
            .and_then(|suffix| suffix.parse::<u8>().ok())
            .is_some_and(|value| (1..=9).contains(&value));
    if reserved {
        return Err(format!(
            "会话 ID 不能使用 Windows 保留文件名，conversation_id={conversation_id}"
        ));
    }
    Ok(())
}

pub fn path_modified_time(path: &Path) -> Option<std::time::SystemTime> {
    fs::metadata(path).and_then(|metadata| metadata.modified()).ok()
}

pub fn directory_children_modified_time(path: &Path) -> Option<std::time::SystemTime> {
    let Ok(entries) = fs::read_dir(path) else {
        return None;
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.metadata().ok())
        .filter_map(|metadata| metadata.modified().ok())
        .max()
}

pub fn message_store_shard_modified_time(
    paths: &MessageStorePaths,
) -> Option<std::time::SystemTime> {
    if paths.is_v3_ready().unwrap_or(false) {
        return path_modified_time(&chat_metadata_store_db_path(&paths.data_path));
    }
    [
        &paths.legacy_conversation_file,
        &paths.manifest_file,
        &paths.meta_file,
        &paths.index_file,
        &paths.blocks_dir,
    ]
    .into_iter()
    .filter_map(|p| path_modified_time(p))
    .chain(std::iter::once(directory_children_modified_time(&paths.blocks_dir)).flatten())
    .max()
}

pub fn write_message_store_text_atomic(
    path: &PathBuf,
    tmp_extension: &str,
    content: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "创建{label}目录失败，path={}，error={err}",
                parent.display()
            )
        })?;
    }
    let tmp_path = path.with_extension(format!("{tmp_extension}.{}", Uuid::new_v4()));
    fs::write(&tmp_path, content).map_err(|err| {
        format!(
            "写入{label}临时文件失败，path={}，error={err}",
            tmp_path.display()
        )
    })?;
    replace_message_store_file_atomic(&tmp_path, path, label)
}

pub fn replace_message_store_file_atomic(
    tmp_path: &PathBuf,
    path: &PathBuf,
    label: &str,
) -> Result<(), String> {
    if let Err(replace_err) = replace_message_store_file_atomic_inner(tmp_path, path) {
        if let Err(copy_err) = fs::copy(tmp_path, path) {
            let _ = fs::remove_file(tmp_path);
            return Err(format!(
                "替换{label}失败，tmp={}，target={}，replace_error={replace_err}，copy_error={copy_err}",
                tmp_path.display(),
                path.display()
            ));
        }
        fs::remove_file(tmp_path).map_err(|cleanup_err| {
            format!(
                "清理{label}临时文件失败，tmp={}，replace_error={replace_err}，cleanup_error={cleanup_err}",
                tmp_path.display()
            )
        })?;
    }
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn replace_message_store_file_atomic_inner(
    tmp_path: &PathBuf,
    path: &PathBuf,
) -> Result<(), std::io::Error> {
    let tmp_wide = path_to_windows_wide_null(tmp_path);
    let path_wide = path_to_windows_wide_null(path);
    let flags = windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
        | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH;
    let ok = unsafe {
        windows_sys::Win32::Storage::FileSystem::MoveFileExW(
            tmp_wide.as_ptr(),
            path_wide.as_ptr(),
            flags,
        )
    };
    if ok == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(target_os = "windows"))]
pub fn replace_message_store_file_atomic_inner(
    tmp_path: &PathBuf,
    path: &PathBuf,
) -> Result<(), std::io::Error> {
    fs::rename(tmp_path, path)
}

#[cfg(target_os = "windows")]
pub fn path_to_windows_wide_null(path: &Path) -> Vec<u16> {
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(test)]
pub mod message_store_atomic_write_tests {
    use super::*;

    #[test]
    fn message_store_paths_should_reject_path_like_conversation_id() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-paths-{}",
            Uuid::new_v4()
        ));
        let data_path = root.join("app_data.json");

        for invalid_id in [
            "../escape",
            "nested/id",
            r"nested\id",
            "bad:name",
            "bad*name",
            "bad\u{7}name",
            "CON",
            "con.txt",
            "com1",
            "LPT1",
            "name.",
            "name ",
            " name",
            ".",
            "",
        ] {
            let err = message_store_paths(&data_path, invalid_id)
                .expect_err("path-like conversation id should fail");
            assert!(err.contains("会话 ID"));
        }

        let paths = message_store_paths(&data_path, "conversation-safe").expect("safe id");
        assert_eq!(paths.conversation_id, "conversation-safe");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_atomic_write_should_create_parent_and_overwrite_target() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-atomic-{}",
            Uuid::new_v4()
        ));
        let target = root.join("nested").join("messages.jsonl");

        write_message_store_text_atomic(&target, "jsonl.tmp", "first", "测试文件")
            .expect("write first");
        write_message_store_text_atomic(&target, "jsonl.tmp", "second", "测试文件")
            .expect("overwrite second");

        assert_eq!(fs::read_to_string(&target).expect("read target"), "second");
        let remaining = fs::read_dir(target.parent().expect("target parent"))
            .expect("read parent")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![target.clone()]);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn message_store_atomic_write_should_use_distinct_temp_files_for_parallel_writers() {
        let root = std::env::temp_dir().join(format!(
            "easy-call-message-store-atomic-parallel-{}",
            Uuid::new_v4()
        ));
        let target = root.join("nested").join("messages.jsonl");
        let mut handles = Vec::new();

        for idx in 0..8 {
            let target = target.clone();
            handles.push(std::thread::spawn(move || {
                write_message_store_text_atomic(
                    &target,
                    "jsonl.tmp",
                    &format!("writer-{idx}"),
                    "并发测试文件",
                )
                .expect("parallel write");
            }));
        }
        for handle in handles {
            handle.join().expect("join writer");
        }

        let final_content = fs::read_to_string(&target).expect("read target");
        assert!(final_content.starts_with("writer-"));
        let remaining = fs::read_dir(target.parent().expect("target parent"))
            .expect("read parent")
            .map(|entry| entry.expect("entry").path())
            .collect::<Vec<_>>();
        assert_eq!(remaining, vec![target.clone()]);
        let _ = fs::remove_dir_all(root);
    }
}
