pub(crate) fn ide_chat_jsonrpc_success(id: Option<Value>, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

pub(crate) fn ide_chat_jsonrpc_error(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": IdeChatJsonRpcError {
            code,
            message: message.into(),
        },
    })
}

pub(crate) fn ide_chat_parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value::<T>(params).map_err(|err| format!("invalid params: {err}"))
}

pub(crate) fn ide_chat_parse_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<T, String> {
    match params {
        Value::Object(mut map) => {
            if let Some(value) = map.remove(field) {
                return ide_chat_parse_params::<T>(value);
            }
            // 统一传输层在 Web 端会把 `{ input: ... }` 解包，
            // 因此同一个协议方法可能以“裸 input”进入 JSON-RPC。
            // 仅对 input 字段接受整对象回退，保留其它字段的严格校验。
            if field == "input" {
                return ide_chat_parse_params::<T>(Value::Object(map));
            }
            Err(format!("{field} is required"))
        }
        _ => Err(format!("{field} is required")),
    }
}

pub(crate) fn ide_chat_parse_optional_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<Option<T>, String> {
    match params {
        Value::Object(mut map) => map
            .remove(field)
            .map(ide_chat_parse_params::<T>)
            .transpose(),
        _ => Ok(None),
    }
}

pub(crate) fn ide_chat_serialize<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|err| format!("serialize result failed: {err}"))
}

include!("web_settings_methods.rs");

include!("chat_methods.rs");
