#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextFileBom {
    None,
    Utf8,
    Utf16Le,
    Utf16Be,
}

#[derive(Debug, Clone)]
struct DecodedTextFile {
    text: String,
    encoding: &'static encoding_rs::Encoding,
    bom: TextFileBom,
}

impl DecodedTextFile {
    fn encode_like_original(&self, text: &str) -> Result<Vec<u8>, String> {
        encode_text_with_detected_encoding(text, self.encoding, self.bom)
    }
}

fn decode_text_file_bytes(bytes: &[u8]) -> Result<DecodedTextFile, String> {
    if bytes.is_empty() {
        return Ok(DecodedTextFile {
            text: String::new(),
            encoding: encoding_rs::UTF_8,
            bom: TextFileBom::None,
        });
    }

    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return decode_with_encoding(&bytes[3..], encoding_rs::UTF_8, TextFileBom::Utf8);
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        return decode_with_encoding(&bytes[2..], encoding_rs::UTF_16LE, TextFileBom::Utf16Le);
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        return decode_with_encoding(&bytes[2..], encoding_rs::UTF_16BE, TextFileBom::Utf16Be);
    }

    if let Ok(text) = std::str::from_utf8(bytes) {
        return Ok(DecodedTextFile {
            text: text.to_string(),
            encoding: encoding_rs::UTF_8,
            bom: TextFileBom::None,
        });
    }

    let mut detector = chardetng::EncodingDetector::new();
    detector.feed(bytes, true);
    let (guessed, _) = detector.guess_assess(None, true);
    if let Ok(decoded) = decode_with_encoding(bytes, guessed, TextFileBom::None) {
        return Ok(decoded);
    }

    #[cfg(target_os = "windows")]
    if let Ok(decoded) = decode_with_encoding(bytes, encoding_rs::GBK, TextFileBom::None) {
        return Ok(decoded);
    }

    Err("无法识别文本编码，已拒绝按有损方式读取".to_string())
}

fn decode_text_file_from_path(path: &std::path::Path) -> Result<DecodedTextFile, String> {
    let bytes =
        std::fs::read(path).map_err(|err| format!("读取文件失败（{}）：{err}", terminal_path_for_user(path)))?;
    decode_text_file_bytes(&bytes).map_err(|err| {
        format!(
            "{}：{}",
            err,
            terminal_path_for_user(path)
        )
    })
}

fn decode_with_encoding(
    bytes: &[u8],
    encoding: &'static encoding_rs::Encoding,
    bom: TextFileBom,
) -> Result<DecodedTextFile, String> {
    let (decoded, _, had_errors) = encoding.decode(bytes);
    if had_errors {
        return Err(format!("{} 解码失败", encoding.name()));
    }
    Ok(DecodedTextFile {
        text: decoded.into_owned(),
        encoding,
        bom,
    })
}

fn encode_text_with_detected_encoding(
    text: &str,
    encoding: &'static encoding_rs::Encoding,
    bom: TextFileBom,
) -> Result<Vec<u8>, String> {
    if encoding == encoding_rs::UTF_16LE {
        let mut out = Vec::with_capacity(text.len().saturating_mul(2).saturating_add(2));
        if bom == TextFileBom::Utf16Le {
            out.extend_from_slice(&[0xff, 0xfe]);
        }
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_le_bytes());
        }
        return Ok(out);
    }
    if encoding == encoding_rs::UTF_16BE {
        let mut out = Vec::with_capacity(text.len().saturating_mul(2).saturating_add(2));
        if bom == TextFileBom::Utf16Be {
            out.extend_from_slice(&[0xfe, 0xff]);
        }
        for unit in text.encode_utf16() {
            out.extend_from_slice(&unit.to_be_bytes());
        }
        return Ok(out);
    }

    let mut encoder = encoding.new_encoder();
    let capacity = encoder
        .max_buffer_length_from_utf8_without_replacement(text.len())
        .ok_or_else(|| format!("{} 编码输出过大", encoding.name()))?;
    let mut bytes = Vec::with_capacity(capacity);
    let (result, read) = encoder.encode_from_utf8_to_vec_without_replacement(text, &mut bytes, true);
    match result {
        encoding_rs::EncoderResult::InputEmpty if read == text.len() => {}
        encoding_rs::EncoderResult::Unmappable(ch) => {
            return Err(format!(
                "字符 '{}' 无法用原文件编码 {} 表示，已拒绝写入",
                ch,
                encoding.name()
            ));
        }
        encoding_rs::EncoderResult::OutputFull => {
            return Err(format!("{} 编码缓冲区不足，已拒绝写入", encoding.name()));
        }
        encoding_rs::EncoderResult::InputEmpty => {
            return Err(format!("{} 编码未完整消费输入，已拒绝写入", encoding.name()));
        }
    }

    let mut out = Vec::with_capacity(bytes.len() + 3);
    match bom {
        TextFileBom::None => {}
        TextFileBom::Utf8 => out.extend_from_slice(&[0xef, 0xbb, 0xbf]),
        TextFileBom::Utf16Le => out.extend_from_slice(&[0xff, 0xfe]),
        TextFileBom::Utf16Be => out.extend_from_slice(&[0xfe, 0xff]),
    }
    out.extend_from_slice(&bytes);
    Ok(out)
}

#[cfg(test)]
mod text_codec_tests {
    use super::*;

    #[test]
    fn decode_should_prefer_strict_utf8_for_ascii() {
        let decoded = decode_text_file_bytes(b"hello").expect("decode ascii");
        assert_eq!(decoded.text, "hello");
        assert_eq!(decoded.encoding, encoding_rs::UTF_8);
    }

    #[test]
    fn decode_should_strip_and_preserve_utf8_bom() {
        let decoded = decode_text_file_bytes(&[0xef, 0xbb, 0xbf, b'a']).expect("decode bom");
        assert_eq!(decoded.text, "a");
        assert_eq!(decoded.bom, TextFileBom::Utf8);
        assert_eq!(decoded.encode_like_original("b").expect("encode"), vec![0xef, 0xbb, 0xbf, b'b']);
    }

    #[test]
    fn decode_should_support_utf16le_bom() {
        let decoded = decode_text_file_bytes(&[0xff, 0xfe, 0x2d, 0x4e, 0x87, 0x65])
            .expect("decode utf16le");
        assert_eq!(decoded.text, "中文");
        assert_eq!(decoded.bom, TextFileBom::Utf16Le);
        assert_eq!(
            decoded.encode_like_original("中").expect("encode"),
            vec![0xff, 0xfe, 0x2d, 0x4e]
        );
    }

    #[test]
    fn decode_should_support_gbk_chinese() {
        let decoded = decode_text_file_bytes(&[0xd6, 0xd0, 0xce, 0xc4]).expect("decode gbk");
        assert_eq!(decoded.text, "中文");
        let encoded = decoded.encode_like_original("中文").expect("encode gbk");
        assert_eq!(encoded, vec![0xd6, 0xd0, 0xce, 0xc4]);
    }

    #[test]
    fn encode_should_reject_unmappable_character() {
        let decoded = DecodedTextFile {
            text: "中文".to_string(),
            encoding: encoding_rs::GBK,
            bom: TextFileBom::None,
        };
        let err = decoded
            .encode_like_original("中文🚀")
            .expect_err("emoji should not be encodable as gbk");
        assert!(err.contains("无法用原文件编码"));
    }
}
