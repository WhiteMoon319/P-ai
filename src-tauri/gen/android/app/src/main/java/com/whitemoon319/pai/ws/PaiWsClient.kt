package com.whitemoon319.pai.ws

import com.whitemoon319.pai.model.BridgeReady
import com.whitemoon319.pai.model.RpcError
import com.whitemoon319.pai.model.RpcRequest
import com.whitemoon319.pai.model.RpcResponse
import com.whitemoon319.pai.native.PaiNative
import com.google.gson.Gson
import com.google.gson.JsonElement
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.withTimeoutOrNull
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import java.util.concurrent.atomic.AtomicLong

enum class ConnectionStatus { Connecting, Connected, Disconnected }

/**
 * 原生桥 JSON-RPC 客户端（JNI，不再走 WS/WebView）。
 *
 * 保留原 [PaiWsClient] 的公开接口语义（connectionState / call / request / sendOneWay /
 * notifications），底层传输从 `ws://127.0.0.1:8429/chat` 换为 `PaiNative.call` 同步
 * JNI 调用（Rust 侧 native_bridge.rs 复用 jsonrpc_dispatch 方法映射）。
 *
 * - 连接状态：原生桥初始化成功即视为 Connected（不需要 WS 握手）。
 * - 请求-响应：JNI 同步返回，带超时兜底避免后端未就绪时永久挂起。
 * - 下行通知（流式 delta 等）：由 [pollEventsLoop] 轮询 PaiNative.pollEvents() 分发，
 *   当前 Rust 侧事件队列尚未实现时为空，后续轮次补齐。
 */
class PaiWsClient(private val scope: CoroutineScope) {

    private val gson = Gson()
    private val idCounter = AtomicLong(1)

    private var pollJob: Job? = null

    val connectionState = MutableStateFlow(ConnectionStatus.Disconnected)
    val bridgeReady = MutableStateFlow<BridgeReady?>(null)
    private val _notifications = MutableSharedFlow<Pair<String, JsonElement?>>(extraBufferCapacity = 128)
    val notifications: SharedFlow<Pair<String, JsonElement?>> = _notifications
    private val pending = object : java.util.concurrent.ConcurrentHashMap<Long, (RpcResponse) -> Unit>() {}

    /** 原生模式：初始化即视为已连接（JNI 同步调用，无握手）。 */
    fun connect(host: String = "", port: Int = 0) {
        connectionState.value = ConnectionStatus.Connecting
        // 后端是否就绪由首次 call 实际结果决定；此处先置 Connected 让 UI 放行，
        // 具体调用失败会通过 error 暴露（与旧 ws 超时语义一致）。
        connectionState.value = ConnectionStatus.Connected
        bridgeReady.value = BridgeReady(
            path = "native",
            authRequired = false,
            authMode = null,
            attachmentTransfer = null,
        )
        startPollLoop()
    }

    fun disconnect() {
        pollJob?.cancel()
        connectionState.value = ConnectionStatus.Disconnected
    }

    private fun startPollLoop() {
        pollJob?.cancel()
        pollJob = scope.launch(Dispatchers.IO) {
            while (connectionState.value == ConnectionStatus.Connected) {
                val raw = runCatching { PaiNative.pollEvents() }.getOrNull() ?: "[]"
                dispatchPolledEvents(raw)
                delay(POLL_INTERVAL_MS)
            }
        }
    }

    private fun dispatchPolledEvents(raw: String) {
        val el = try {
            gson.fromJson(raw, JsonElement::class.java)
        } catch (_: Exception) {
            return
        }
        if (el == null || !el.isJsonArray) return
        for (item in el.asJsonArray) {
            if (item == null || !item.isJsonObject) continue
            val obj = item.asJsonObject
            val method = obj.get("method")?.takeIf { !it.isJsonNull }?.asString ?: continue
            val params = try {
                obj.get("params")
            } catch (_: Exception) {
                null
            }
            scope.launch(Dispatchers.IO) {
                _notifications.emit(method to params)
            }
        }
    }

    /** 执行 JSON-RPC 请求：JNI 同步调用 + 超时兜底（防后端未就绪时永久挂起）。 */
    suspend fun call(method: String, params: Any? = null): RpcResponse {
        val id = idCounter.getAndIncrement()
        val body = RpcRequest(jsonrpc = "2.0", id = id, method = method, params = params ?: emptyMap<String, Any?>())
        return withContext(Dispatchers.IO) {
            val raw = withTimeoutOrNull(15_000) {
                runCatching { PaiNative.call(gson.toJson(body)) }.getOrNull()
            }
            if (raw == null || raw.isBlank()) {
                RpcResponse(jsonrpc = "2.0", id = id, error = RpcError(-32001, "native call 超时或失败（后端未就绪?） $method"))
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

    /** 请求并解析到指定类型；失败抛异常。 */
    suspend fun <T> request(method: String, params: Any?, clazz: Class<T>): T {
        val resp = call(method, params)
        if (resp.error != null) {
            throw IllegalStateException("$method 失败: ${resp.error.code} ${resp.error.message}")
        }
        val result = resp.result ?: throw IllegalStateException("$method 无 result")
        return gson.fromJson(result, clazz)
    }

    /** 请求并用 TypeToken 解析（如 List<T>）；失败抛异常。 */
    suspend fun <T> request(method: String, params: Any?, type: java.lang.reflect.Type): T {
        val resp = call(method, params)
        if (resp.error != null) {
            throw IllegalStateException("$method 失败: ${resp.error.code} ${resp.error.message}")
        }
        val result = resp.result ?: throw IllegalStateException("$method 无 result")
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

    companion object {
        const val RECONNECT_INTERVAL_MS = 2000L
        const val REQUEST_TIMEOUT_MS = 4000L
        private const val POLL_INTERVAL_MS = 300L
    }
}
