package ai.easycall.app.ws

import ai.easycall.app.model.BridgeReady
import ai.easycall.app.model.RpcError
import ai.easycall.app.model.RpcRequest
import ai.easycall.app.model.RpcResponse
import com.google.gson.Gson
import com.google.gson.JsonElement
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableSharedFlow
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharedFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.suspendCancellableCoroutine
import okhttp3.OkHttpClient
import okhttp3.Request
import okhttp3.Response
import okhttp3.WebSocket
import okhttp3.WebSocketListener
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.atomic.AtomicLong
import kotlin.coroutines.resume

enum class ConnectionStatus { Connecting, Connected, Disconnected }

/**
 * ws://127.0.0.1:8429/chat JSON-RPC WebSocket 客户端。
 * - 自动重连（Rust 后端延迟启动，需等待 run_deferred_setup 拉起 8429）。
 * - 请求-响应按 id 配对；下行通知分发到 [notifications] SharedFlow。
 * - loopback 免认证（后端已按 peer_is_local 判定）。
 */
class PaiWsClient(private val scope: CoroutineScope) {

    private val gson = Gson()
    private val httpClient = OkHttpClient()
    private val idCounter = AtomicLong(1)

    private var socket: WebSocket? = null
    private var connectJob: Job? = null
    private var closedByUser = false

    val connectionState = MutableStateFlow(ConnectionStatus.Disconnected)
    val bridgeReady = MutableStateFlow<BridgeReady?>(null)
    private val _notifications = MutableSharedFlow<Pair<String, JsonElement?>>(extraBufferCapacity = 128)
    val notifications: SharedFlow<Pair<String, JsonElement?>> = _notifications
    private val pending = ConcurrentHashMap<Long, (RpcResponse) -> Unit>()

    fun connect(host: String = "127.0.0.1", port: Int = 8429) {
        closedByUser = false
        connectJob?.cancel()
        connectJob = scope.launch(Dispatchers.IO) {
            while (!closedByUser) {
                val connected = tryOpen(host, port)
                if (!connected) {
                    connectionState.value = ConnectionStatus.Disconnected
                }
                delay(RECONNECT_INTERVAL_MS)
            }
        }
    }

    fun disconnect() {
        closedByUser = true
        connectJob?.cancel()
        socket?.close(4000, "client closed")
        socket = null
        connectionState.value = ConnectionStatus.Disconnected
    }

    /** 尝试建立一条连接；失败返回 false。 */
    private suspend fun tryOpen(host: String, port: Int): Boolean =
        suspendCancellableCoroutine { cont ->
            connectionState.value = ConnectionStatus.Connecting
            val url = "ws://$host:$port/chat"
            val request = Request.Builder().url(url).build()
            // 保证 continuation 只 resume 一次：okhttp 可能在 onOpen 后仍触发 onFailure（重连/重置），
            // 导致 Already resumed 崩溃。用 AtomicBoolean 丢弃后续 resume。
            val resumeOnce = java.util.concurrent.atomic.AtomicBoolean(false)
            fun complete(value: Boolean) {
                if (resumeOnce.compareAndSet(false, true)) {
                    if (!cont.isCancelled) cont.resume(value)
                }
            }
            val ws = httpClient.newWebSocket(request, object : WebSocketListener() {
                override fun onOpen(webSocket: WebSocket, response: Response) {
                    socket = webSocket
                    connectionState.value = ConnectionStatus.Connected
                    complete(true)
                }

                override fun onMessage(webSocket: WebSocket, text: String) {
                    handleMessage(text)
                }

                override fun onFailure(webSocket: WebSocket, t: Throwable, response: Response?) {
                    connectionState.value = ConnectionStatus.Disconnected
                    if (socket != webSocket) {
                        socket = null
                    }
                    complete(false)
                }

                override fun onClosed(webSocket: WebSocket, code: Int, reason: String) {
                    connectionState.value = ConnectionStatus.Disconnected
                }
            })
            cont.invokeOnCancellation {
                ws.cancel()
            }
        }

    private fun handleMessage(text: String) {
        val el = try {
            gson.fromJson(text, JsonElement::class.java)
        } catch (_: Exception) {
            return
        }
        if (el == null || !el.isJsonObject) return
        val obj = el.asJsonObject

        // 下行通知（无 id 有 method）
        if (!obj.has("id") && obj.has("method")) {
            val method = obj.get("method").asString
            val params = try {
                obj.get("params")
            } catch (_: Exception) {
                null
            }
            handleNotification(method, params)
            return
        }

        // 响应
        if (obj.has("id") && !obj.get("id").isJsonNull) {
            val id = obj.get("id").asLong
            val cb = pending.remove(id)
            if (cb != null) {
                val resp = if (obj.has("error")) {
                    RpcResponse(
                        jsonrpc = "2.0", id = id,
                        error = gson.fromJson(obj.get("error"), RpcError::class.java),
                    )
                } else {
                    RpcResponse(
                        jsonrpc = "2.0", id = id,
                        result = try {
                            obj.get("result")
                        } catch (_: Exception) {
                            null
                        },
                    )
                }
                cb.invoke(resp)
            }
        }
    }

    private fun handleNotification(method: String, params: JsonElement?) {
        if (method == "bridge.ready" && params != null && params.isJsonObject) {
            bridgeReady.value = gson.fromJson(params, BridgeReady::class.java)
        }
        scope.launch(Dispatchers.IO) {
            _notifications.emit(method to params)
        }
    }

    /** 执行请求并挂起等待响应。 */
    suspend fun call(method: String, params: Any? = null): RpcResponse {
        val id = idCounter.getAndIncrement()
        val body = RpcRequest(jsonrpc = "2.0", id = id, method = method, params = params ?: emptyMap<String, Any?>())
        return suspendCancellableCoroutine { cont ->
            val ws = socket
            if (ws == null || connectionState.value != ConnectionStatus.Connected) {
                cont.resume(RpcResponse(jsonrpc = "2.0", id = id, error = RpcError(-32000, "not connected")))
                return@suspendCancellableCoroutine
            }
            pending[id] = { resp ->
                if (!cont.isCancelled) cont.resume(resp)
            }
            cont.invokeOnCancellation {
                pending.remove(id)
            }
            ws.send(gson.toJson(body))
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

    fun sendOneWay(method: String, params: Any? = null) {
        val ws = socket ?: return
        if (connectionState.value != ConnectionStatus.Connected) return
        val id = idCounter.getAndIncrement()
        val body = RpcRequest(jsonrpc = "2.0", id = id, method = method, params = params ?: emptyMap<String, Any?>())
        ws.send(gson.toJson(body))
    }

    companion object {
        const val RECONNECT_INTERVAL_MS = 2000L
    }
}