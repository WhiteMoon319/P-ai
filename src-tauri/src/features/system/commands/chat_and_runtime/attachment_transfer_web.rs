async fn ide_attachment_transfer_begin(
    state: &AppState,
    client_id: &str,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_params::<AttachmentTransferBeginInput>(params)?;
    ide_chat_serialize(attachment_transfer_begin_inner(input, state, client_id.trim(), true).await?)
}

async fn ide_attachment_transfer_complete(
    state: &AppState,
    client_id: &str,
    params: Value,
) -> Result<Value, String> {
    let input = ide_chat_parse_params::<AttachmentTransferIdInput>(params)?;
    ide_chat_serialize(
        attachment_transfer_complete_inner(&input.transfer_id, state, client_id.trim()).await?,
    )
}

async fn ide_attachment_transfer_abort(client_id: &str, params: Value) -> Result<Value, String> {
    let input = ide_chat_parse_params::<AttachmentTransferIdInput>(params)?;
    attachment_transfer_abort_inner(&input.transfer_id, client_id.trim()).await
}

fn attachment_transfer_decode_websocket_frame(
    data: &[u8],
) -> Result<(String, u64, Vec<u8>), String> {
    const HEADER_BYTES: usize = 1 + 16 + 8 + 4;
    if data.len() < HEADER_BYTES {
        return Err(attachment_transfer_error(
            "INVALID_BINARY_FRAME",
            "附件二进制帧头不完整",
        ));
    }
    if data.len() > HEADER_BYTES + ATTACHMENT_TRANSFER_CHUNK_BYTES {
        return Err(attachment_transfer_error(
            "INVALID_BINARY_FRAME",
            format!("附件二进制帧过大：{}", data.len()),
        ));
    }
    if data[0] != 1 {
        return Err(attachment_transfer_error(
            "INVALID_BINARY_FRAME",
            format!("附件二进制帧版本不支持：{}", data[0]),
        ));
    }
    let transfer_id = Uuid::from_slice(&data[1..17])
        .map_err(|err| {
            attachment_transfer_error("INVALID_BINARY_FRAME", format!("transferId 无效：{err}"))
        })?
        .to_string();
    let mut offset_bytes = [0u8; 8];
    offset_bytes.copy_from_slice(&data[17..25]);
    let offset = u64::from_be_bytes(offset_bytes);
    let mut length_bytes = [0u8; 4];
    length_bytes.copy_from_slice(&data[25..29]);
    let payload_len = u32::from_be_bytes(length_bytes) as usize;
    if payload_len == 0 || payload_len > ATTACHMENT_TRANSFER_CHUNK_BYTES {
        return Err(attachment_transfer_error(
            "INVALID_BINARY_FRAME",
            format!("附件二进制分块大小无效：{payload_len}"),
        ));
    }
    if data.len() != HEADER_BYTES + payload_len {
        return Err(attachment_transfer_error(
            "INVALID_BINARY_FRAME",
            format!(
                "附件二进制帧长度不匹配：header_payload={payload_len}，actual={}",
                data.len() - HEADER_BYTES
            ),
        ));
    }
    Ok((transfer_id, offset, data[HEADER_BYTES..].to_vec()))
}

async fn ide_attachment_transfer_binary_chunk(
    client_id: &str,
    data: &[u8],
) -> Result<Value, String> {
    let (transfer_id, offset, payload) = attachment_transfer_decode_websocket_frame(data)?;
    let output =
        attachment_transfer_append_chunk_inner(&transfer_id, client_id.trim(), offset, payload)
            .await?;
    Ok(serde_json::json!({
        "jsonrpc": "2.0",
        "method": "attachment.chunkAck",
        "params": output,
    }))
}

#[cfg(test)]
mod attachment_transfer_web_tests {
    use super::*;

    fn websocket_frame(transfer_id: Uuid, offset: u64, payload: &[u8]) -> Vec<u8> {
        let mut frame = Vec::with_capacity(29 + payload.len());
        frame.push(1);
        frame.extend_from_slice(transfer_id.as_bytes());
        frame.extend_from_slice(&offset.to_be_bytes());
        frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        frame.extend_from_slice(payload);
        frame
    }

    #[test]
    fn websocket_frame_should_decode_fixed_envelope() {
        let transfer_id = Uuid::new_v4();
        let frame = websocket_frame(transfer_id, 42, b"payload");
        let decoded = attachment_transfer_decode_websocket_frame(&frame).expect("decode frame");
        assert_eq!(decoded.0, transfer_id.to_string());
        assert_eq!(decoded.1, 42);
        assert_eq!(decoded.2, b"payload");
    }

    #[test]
    fn websocket_frame_should_reject_length_mismatch() {
        let transfer_id = Uuid::new_v4();
        let mut frame = websocket_frame(transfer_id, 0, b"payload");
        frame.pop();
        let error = attachment_transfer_decode_websocket_frame(&frame).expect_err("reject frame");
        assert!(error.starts_with("INVALID_BINARY_FRAME:"));
    }

    #[test]
    fn websocket_frame_should_reject_oversized_payload_before_copying() {
        let transfer_id = Uuid::new_v4();
        let frame = websocket_frame(
            transfer_id,
            0,
            &vec![0u8; ATTACHMENT_TRANSFER_CHUNK_BYTES + 1],
        );
        let error =
            attachment_transfer_decode_websocket_frame(&frame).expect_err("reject oversized frame");
        assert!(error.starts_with("INVALID_BINARY_FRAME:"));
    }
}
