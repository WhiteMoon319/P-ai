fn ide_chat_jsonrpc_success(id: Option<Value>, result: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "result": result,
    })
}

fn ide_chat_jsonrpc_error(id: Option<Value>, code: i32, message: impl Into<String>) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": IdeChatJsonRpcError {
            code,
            message: message.into(),
        },
    })
}

fn ide_chat_parse_params<T: serde::de::DeserializeOwned>(params: Value) -> Result<T, String> {
    serde_json::from_value::<T>(params).map_err(|err| format!("invalid params: {err}"))
}

fn ide_chat_parse_param_field<T: serde::de::DeserializeOwned>(
    params: Value,
    field: &str,
) -> Result<T, String> {
    match params {
        Value::Object(mut map) => map
            .remove(field)
            .ok_or_else(|| format!("{field} is required"))
            .and_then(ide_chat_parse_params::<T>),
        _ => Err(format!("{field} is required")),
    }
}

fn ide_chat_parse_optional_param_field<T: serde::de::DeserializeOwned>(
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

fn ide_chat_serialize<T: Serialize>(value: T) -> Result<Value, String> {
    serde_json::to_value(value).map_err(|err| format!("serialize result failed: {err}"))
}

include!("web_settings_methods.rs");

include!("chat_methods.rs");
