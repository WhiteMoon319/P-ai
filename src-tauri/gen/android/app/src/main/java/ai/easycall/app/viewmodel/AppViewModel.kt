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
                id = "local-${System.currentTimeMillis()}",
                role = "user",
                parts = listOf(ai.easycall.app.model.MessagePart(type = "Text", text = trimmed)),
            )
        )
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
                                    m.assistantMessage?.let { appendAssistantMessage(it) }
                                } else {
                                    appendAssistantText(finalText)
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
                finalizeStreaming()
            }
        }
    }

    private fun appendAssistantText(text: String) {
        val base = messages.value.filterNot { it.id.startsWith("local-") }
        val compiled = base.plus(
            ChatMessage(
                id = "assistant-${System.currentTimeMillis()}",
                role = "assistant",
                parts = listOf(ai.easycall.app.model.MessagePart(type = "Text", text = text)),
            )
        )
        messages.value = compiled
        streamingText.value = ""
    }

    private fun appendAssistantMessage(message: ChatMessage) {
        val base = messages.value.filterNot { it.id.startsWith("local-") }
        messages.value = base.plus(message)
        streamingText.value = ""
    }

    private fun finalizeStreaming() {
        // commit 正在流式输出的文本为正式消息
        val pendingText = streamingText.value
        if (pendingText.isNotEmpty()) {
            appendAssistantText(pendingText)
        }
        streamingText.value = ""
        isStreaming.value = false
    }
}