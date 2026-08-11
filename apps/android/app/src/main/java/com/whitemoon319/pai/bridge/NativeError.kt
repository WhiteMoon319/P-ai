package com.whitemoon319.pai.bridge

/**
 * 原生桥 RPC 异常（请求失败 / 超时 / 解析失败 / 后端未就绪）。
 * 由 [NativeRpcClient] 在 error 响应或无 result 时抛出。
 */
class NativeRpcException(
    message: String,
    val code: Int = -32000,
    cause: Throwable? = null,
) : IllegalStateException(message, cause)

/** 原生桥错误码（与 crates/pai-protocol/src/errors.rs 对齐）。 */
object NativeRpcErrorCode {
    const val NATIVE_RUNTIME_NOT_READY = -32000
    const val NATIVE_CALL_TIMEOUT = -32001
    const val NATIVE_RESPONSE_PARSE = -32003
    const val PARSE_ERROR = -32700
    const val INVALID_REQUEST = -32600
    const val METHOD_NOT_FOUND = -32601
    const val INVALID_PARAMS = -32602
    const val INTERNAL_ERROR = -32603
}
