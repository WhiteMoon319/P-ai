package ai.easycall.app.viewmodel

import ai.easycall.app.model.ChatMessage
import ai.easycall.app.model.ConversationSummary
import ai.easycall.app.model.DeltaNotification
import ai.easycall.app.ws.ChatService
import ai.easycall.app.ws.ConnectionStatus
import ai.easycall.app.ws.PaiWsClient
import com.google.gson.Gson
import com.google.gson.JsonElement
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.flow.collectLatest

/**
 * 应用级 ViewModel：连接管理、会话列表、消息流式接收。
 */
class AppViewModel(
    private val scope: CoroutineScope,
) {
    private val client = PaiWsClient(scope)
    private val service = ChatService(client)
    private val gson = Gson()
    private var notificationJob: Job? = null

    val connectionState: StateFlow<ConnectionStatus> = client.connectionState
    val conversations = MutableStateFlow<List<ConversationSummary>>(emptyList())
    val currentConversationId = MutableStateFlow<String?>(null)
    val messages = MutableStateFlow<List<ChatMessage>>(emptyList())
    val streamingText = MutableStateFlow("")
    val isStreaming = MutableStateFlow(false)
    val loading = MutableStateFlow(false)
    val error = MutableStateFlow<String?>(null)

    fun start() {
        client.connect()
        notificationJob = scope.launch(Dispatchers.IO) {
            client.notifications.collectLatest { (method, params) ->
                handleNotification(method, params)
            }
        }
    }

    fun stop() {
        notificationJob?.cancel()
        client.disconnect()
    }

    suspend fun refreshConversations() {
        withContext(Dispatchers.IO) {
            try {
                val result = service.listConversations()
                conversations.value = result.conversations
            } catch (e: Exception) {
                error.value = "刷新会话失败: ${e.message}"
            }
        }
    }

    suspend fun createConversation(title: String? = null): String? {
        return withContext(Dispatchers.IO) {
            try {
                val result = service.createConversation(agentId = null, departmentId = null, title = title)
                val id = result.conversationId
                if (id != null) {
                    openConversation(id)
                }
                id
            } catch (e: Exception) {
                error.value = "新建会话失败: ${e.message}"
                null
            }
        }
    }

    suspend fun openConversation(conversationId: String) {
        val agentId = conversations.value.firstOrNull { it.conversationId == conversationId }?.agentId
        withContext(Dispatchers.IO) {
            loading.value = true
            try {
                service.setActive(conversationId, agentId)
                val page = service.blockPage(conversationId)
                currentConversationId.value = conversationId
                messages.value = page.messages
                streamingText.value = ""
                isStreaming.value = false
            } catch (e: Exception) {
                error.value = "打开会话失败: ${e.message}"
            } finally {
                loading.value = false
            }
        }
    }

    suspend fun sendMessage(text: String) {
        val conversationId = currentConversationId.value ?: return
        val conv = conversations.value.firstOrNull { it.conversationId == conversationId }
        val agent = conv?.agentId
        val departmentId = conv?.departmentId
        val trimmed = text.trim()
        if (trimmed.isEmpty()) return
        // 乐观显示用户消息
        messages.value = messages.value.plus(
            ChatMessage(
                id = "local-${nextLocalId()}",
                role = "user",
                parts = listOf(ai.easycall.app.model.MessagePart(type = "Text", text = trimmed)),
            )
        )
        committedAssistantText = null
        streamingText.value = ""
        isStreaming.value = true
        withContext(Dispatchers.IO) {
            try {
                service.send(conversationId, departmentId, agent, trimmed)
            } catch (e: Exception) {
                error.value = "发送失败: ${e.message}"
                isStreaming.value = false
            }
        }
    }

    suspend fun stopStreaming() {
        val conversationId = currentConversationId.value ?: return
        val conv = conversations.value.firstOrNull { it.conversationId == conversationId }
        val agent = conv?.agentId
        val departmentId = conv?.departmentId
        withContext(Dispatchers.IO) {
            try {
                service.stop(conversationId, departmentId, agent)
            } catch (_: Exception) {
            }
            finalizeStreaming()
        }
    }

    private fun handleNotification(method: String, params: JsonElement?) {
        // 诊断：确认下行事件是否到达 Kotlin
        android.util.Log.d("PaiNotify", "method=$method convId=${params?.asJsonObject?.get("conversationId")} curr=${currentConversationId.value}")
        when (method) {
            "chat.assistantDelta" -> {
                val notif = params?.let { gson.fromJson(it, DeltaNotification::class.java) }
                val convId = notif?.conversationId
                if (convId != null && convId == currentConversationId.value) {
                    val event = notif.event
                    when (event?.kind) {
                        null, "", "stream", "activity_reasoning_delta" -> {
                            val delta = event?.delta ?: ""
                            if (delta.isNotEmpty()) {
                                streamingText.value = streamingText.value + delta
                            }
                        }
                        "round_completed" -> {
                            val msgJson = event?.message
                            val m = msgJson?.let {
                                runCatching { gson.fromJson(it, ai.easycall.app.model.DeltaMessage::class.java) }.getOrNull()
                            }
                            if (m != null) {
                                val finalText = m.assistantText
                                if (finalText.isNullOrEmpty()) {
                                    m.assistantMessage?.let { commitAssistant("", it) }
                                } else {
                                    commitAssistant(finalText, null)
                                }
                            }
                            finalizeStreaming()
                        }
                        "round_failed" -> {
                            val msgJson = event?.message
                            val errText = msgJson?.let {
                                runCatching { gson.fromJson(it, ai.easycall.app.model.DeltaMessage::class.java) }.getOrNull()?.assistantText
                            }
                            error.value = errText ?: "生成失败"
                            finalizeStreaming()
                        }
                    }
                }
            }
            "chat.roundFinished" -> {
                // params 顶层平铺 assistantText / assistantMessage；用 commitAssistant 去重
                val text = params?.asJsonObject?.get("assistantText")?.takeIf { !it.isJsonNull }?.asString
                if (!text.isNullOrEmpty()) {
                    commitAssistant(text, null)
                } else {
                    val msg = params?.asJsonObject?.get("assistantMessage")?.takeIf { !it.isJsonNull }
                    val parsed = msg?.let {
                        runCatching { gson.fromJson(it, ChatMessage::class.java) }.getOrNull()
                    }
                    if (parsed != null) commitAssistant("", parsed)
                }
                finalizeStreaming()
            }
        }
    }

    private val idSeq = java.util.concurrent.atomic.AtomicLong(0)
    private fun nextLocalId() = "${System.currentTimeMillis()}-${idSeq.incrementAndGet()}"

    /** 记录当前回合已落地的 assistant 文本，避免 round_completed 与 roundFinished 重复落盘。 */
    private var committedAssistantText: String? = null

    private fun commitAssistant(text: String, message: ChatMessage?) {
        val trimmed = text?.trim().orEmpty()
        if (!trimmed.isEmpty()) {
            if (committedAssistantText == trimmed) {
                // 本回合已落过同意文本，只清理流式缓冲，不再重复落盘
                streamingText.value = ""
                return
            }
            committedAssistantText = trimmed
            messages.value = messages.value.plus(
                ChatMessage(
                    id = "assistant-${nextLocalId()}",
                    role = "assistant",
                    parts = listOf(ai.easycall.app.model.MessagePart(type = "Text", text = trimmed)),
                )
            )
            streamingText.value = ""
            return
        }
        if (message != null) {
            messages.value = messages.value.plus(message)
            streamingText.value = ""
        }
    }

    private fun finalizeStreaming() {
        // commit 正在流式输出的文本为正式消息（走统一去重 commit）
        val pendingText = streamingText.value
        if (pendingText.isNotEmpty()) {
            commitAssistant(pendingText, null)
        }
        streamingText.value = ""
        isStreaming.value = false
    }
}