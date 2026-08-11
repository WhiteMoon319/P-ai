package com.whitemoon319.pai.bridge

import com.whitemoon319.pai.model.BridgeReady
import com.whitemoon319.pai.native.PaiNative
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

enum class ConnectionStatus { Connecting, Connected, Disconnected }

/**
 * 原生事件泵：轮询 PaiNative.pollEvents() 消费 Rust 下行事件，
 * 按顺序同步投递（禁止并发 emit，保证流式 delta 顺序）。
 *
 * 事件（chat.assistantDelta / chat.roundFinished / app.keepAlive / app.notification /
 * messageStore.migration.progress 等）由订阅方（AppViewModel.handleNotification）消费。
 */
class NativeEventPump(private val scope: CoroutineScope) {

    private val gson = Gson()
    private var pollJob: Job? = null

    val connectionState = MutableStateFlow(ConnectionStatus.Disconnected)
    val bridgeReady = MutableStateFlow<BridgeReady?>(null)
    private val _notifications = MutableSharedFlow<Pair<String, JsonElement?>>(extraBufferCapacity = 128)
    val notifications: SharedFlow<Pair<String, JsonElement?>> = _notifications

    /** 原生模式：初始化即视为已连接（JNI 同步调用，无握手）。 */
    fun connect() {
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
            // 顺序同步 emit：不能 launch 并发，否则流式 delta 顺序错乱（词序颠倒）。
            // SharedFlow.emit 在无订阅者时直接丢弃，有订阅者（notificationJob）时同步投递。
            _notifications.tryEmit(method to params)
        }
    }

    companion object {
        const val RECONNECT_INTERVAL_MS = 2000L
        private const val POLL_INTERVAL_MS = 300L
    }
}
