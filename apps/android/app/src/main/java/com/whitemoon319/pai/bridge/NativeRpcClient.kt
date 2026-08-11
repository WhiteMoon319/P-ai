package com.whitemoon319.pai.bridge

import com.whitemoon319.pai.model.RpcError
import com.whitemoon319.pai.model.RpcRequest
import com.whitemoon319.pai.model.RpcResponse
import com.whitemoon319.pai.native.PaiNative
import com.google.gson.Gson
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.coroutines.withTimeoutOrNull
import java.util.concurrent.atomic.AtomicLong

/**
 * 原生桥 JSON-RPC 客户端（JNI 同步调用，不依赖 WS/WebView）。
 *
 * 封装请求-响应：call / callLong / request / requestLong / sendOneWay。
 * 超时语义：普通查询默认 15s；长任务（migration / workspace / rootfs）走 callLong 600s。
 * 下行事件（流式 delta 等）由 [NativeEventPump] 轮询分发，本类不处理。
 */
class NativeRpcClient {

    private val gson = Gson()
    private val idCounter = AtomicLong(1)

    /** 默认短超时：普通查询（会话/配置/文件列表等）防后端未就绪时永久挂起。 */
    private val defaultTimeoutMs = 15_000L

    /** 长任务超时：聊天发送、迁移、工作区初始化/修复/重置、rootfs 导入等。 */
    private val longTimeoutMs = 600_000L

    /** 执行 JSON-RPC 请求：JNI 同步调用 + 超时兜底（防后端未就绪时永久挂起）。 */
    suspend fun call(method: String, params: Any? = null, timeoutMs: Long = defaultTimeoutMs): RpcResponse {
        val id = idCounter.getAndIncrement()
        val body = RpcRequest(jsonrpc = "2.0", id = id, method = method, params = params ?: emptyMap<String, Any?>())
        return withContext(Dispatchers.IO) {
            val raw = withTimeoutOrNull(timeoutMs) {
                runCatching { PaiNative.call(gson.toJson(body)) }.getOrNull()
            }
            if (raw == null || raw.isBlank()) {
                RpcResponse(jsonrpc = "2.0", id = id, error = RpcError(-32001, "native call 超时（${timeoutMs}ms）或失败（后端未就绪?） $method"))
            } else {
                val resp = try {
                    gson.fromJson(raw, RpcResponse::class.java)
                } catch (e: Exception) {
                    RpcResponse(jsonrpc = "2.0", id = id, error = RpcError(-32003, "解析 native 响应失败: ${e.message}"))
                }
                if (resp.error != null && resp.id == null) {
                    resp.copy(id = id)
                } else {
                    resp
                }
            }
        }
    }

    /** 长任务请求：聊天发送、迁移、工作区初始化等可能远超 15s 的操作。 */
    suspend fun callLong(method: String, params: Any? = null): RpcResponse =
        call(method, params, longTimeoutMs)

    /** 长任务请求并解析到指定类型；失败抛异常（独立超时，不占普通查询短超时）。 */
    suspend fun <T> requestLong(method: String, params: Any?, clazz: Class<T>): T {
        val resp = callLong(method, params)
        if (resp.error != null) {
            throw NativeRpcException("$method 失败: ${resp.error.code} ${resp.error.message}")
        }
        val result = resp.result ?: throw NativeRpcException("$method 无 result")
        return gson.fromJson(result, clazz)
    }

    /** 请求并解析到指定类型；失败抛异常。 */
    suspend fun <T> request(method: String, params: Any?, clazz: Class<T>): T {
        val resp = call(method, params)
        if (resp.error != null) {
            throw NativeRpcException("$method 失败: ${resp.error.code} ${resp.error.message}")
        }
        val result = resp.result ?: throw NativeRpcException("$method 无 result")
        return gson.fromJson(result, clazz)
    }

    /** 请求并用 TypeToken 解析（如 List<T>）；失败抛异常。 */
    suspend fun <T> request(method: String, params: Any?, type: java.lang.reflect.Type): T {
        val resp = call(method, params)
        if (resp.error != null) {
            throw NativeRpcException("$method 失败: ${resp.error.code} ${resp.error.message}")
        }
        val result = resp.result ?: throw NativeRpcException("$method 无 result")
        return gson.fromJson(result, type)
    }

    /** 单向请求（不等待响应）：原生桥下等价普通 call，语义兼容。 */
    fun sendOneWay(method: String, params: Any? = null) {
        val id = idCounter.getAndIncrement()
        val body = RpcRequest(jsonrpc = "2.0", id = id, method = method, params = params ?: emptyMap<String, Any?>())
        runCatching {
            val raw = PaiNative.call(gson.toJson(body))
            val resp = gson.fromJson(raw, RpcResponse::class.java)
            if (resp.error != null) {
                android.util.Log.w("PaiNative", "sendOneWay $method 失败: ${resp.error.code} ${resp.error.message}")
            }
        }
    }
}
