use super::*;

fn insert_test_session(
    transfer_id: &str,
    owner: &str,
    staging_path: PathBuf,
    declared_size: u64,
) -> AttachmentTransferEntry {
    let entry = AttachmentTransferEntry {
        owner: owner.to_string(),
        session: Arc::new(tokio::sync::Mutex::new(AttachmentTransferSession {
            file_name: "test.bin".to_string(),
            mime: "application/octet-stream".to_string(),
            declared_size,
            received_size: 0,
            staging_path,
            updated_at: std::time::Instant::now(),
            closed: false,
        })),
    };
    attachment_transfer_runtime()
        .sessions
        .lock()
        .expect("lock sessions")
        .insert(transfer_id.to_string(), entry.clone());
    entry
}

#[test]
fn attachment_files_equal_should_compare_without_loading_whole_file() {
    let root = std::env::temp_dir().join(format!("pai-attachment-equal-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create temp dir");
    let left = root.join("left.bin");
    let right = root.join("right.bin");
    std::fs::write(&left, b"same-content").expect("write left");
    std::fs::write(&right, b"same-content").expect("write right");
    assert!(attachment_files_equal(&left, &right).expect("compare equal"));
    std::fs::write(&right, b"different").expect("rewrite right");
    assert!(!attachment_files_equal(&left, &right).expect("compare different"));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn attachment_transfer_should_not_expire_closed_session() {
    let session = AttachmentTransferSession {
        file_name: "test.bin".to_string(),
        mime: "application/octet-stream".to_string(),
        declared_size: 6,
        received_size: 6,
        staging_path: std::env::temp_dir().join("unused.part"),
        updated_at: std::time::Instant::now() - std::time::Duration::from_secs(ATTACHMENT_TRANSFER_IDLE_TIMEOUT_SECS + 1),
        closed: true,
    };
    assert!(!attachment_transfer_session_should_expire(&session));
}

#[tokio::test]
async fn attachment_chunk_should_enforce_owner_and_sequential_offset() {
    let root = std::env::temp_dir().join(format!("pai-attachment-chunk-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("create temp dir");
    let staging_path = root.join("upload.part");
    std::fs::write(&staging_path, []).expect("create staging file");
    let transfer_id = Uuid::new_v4().to_string();
    let entry = insert_test_session(&transfer_id, "owner-a", staging_path.clone(), 6);

    let owner_error =
        attachment_transfer_append_chunk_inner(&transfer_id, "owner-b", 0, b"abc".to_vec())
            .await
            .expect_err("reject wrong owner");
    assert!(owner_error.starts_with("TRANSFER_OWNER_MISMATCH:"));

    let first = attachment_transfer_append_chunk_inner(&transfer_id, "owner-a", 0, b"abc".to_vec())
        .await
        .expect("append first chunk");
    assert_eq!(first.next_offset, 3);

    let duplicate =
        attachment_transfer_append_chunk_inner(&transfer_id, "owner-a", 0, b"abc".to_vec())
            .await
            .expect("ack duplicate chunk");
    assert_eq!(duplicate.next_offset, 3);

    let duplicate_mismatch =
        attachment_transfer_append_chunk_inner(&transfer_id, "owner-a", 0, b"xyz".to_vec())
            .await
            .expect_err("reject mismatched duplicate chunk");
    assert!(duplicate_mismatch.starts_with("TRANSFER_OFFSET_MISMATCH:"));

    let offset_error =
        attachment_transfer_append_chunk_inner(&transfer_id, "owner-a", 2, b"de".to_vec())
            .await
            .expect_err("reject overlapping chunk");
    assert!(offset_error.starts_with("TRANSFER_OFFSET_MISMATCH:"));

    let second =
        attachment_transfer_append_chunk_inner(&transfer_id, "owner-a", 3, b"def".to_vec())
            .await
            .expect("append second chunk");
    assert_eq!(second.next_offset, 6);
    assert_eq!(
        std::fs::read(&staging_path).expect("read staging"),
        b"abcdef"
    );

    attachment_transfer_remove_entry(&transfer_id, &entry);
    let _ = std::fs::remove_dir_all(root);
}
