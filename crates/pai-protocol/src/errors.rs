//! JSON-RPC 标准错误码。

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

/// 原生桥层错误（Kotlin 侧超时/解析/后端未就绪）。
pub const NATIVE_CALL_TIMEOUT: i64 = -32001;
pub const NATIVE_RESPONSE_PARSE: i64 = -32003;
pub const NATIVE_RUNTIME_NOT_READY: i64 = -32000;
