package ai.easycall.app.viewmodel

import ai.easycall.app.model.ChatMessage
import ai.easycall.app.model.ConversationSummary
import ai.easycall.app.model.DeltaNotification
import ai.easycall.app.ws.ChatService
import ai.easycall.app.ws.ConnectionStatus
import ai.easycall.app.ws.PaiWsClient
import android.content.Context
import com.google.gson.Gson
import com.google.gson.JsonElement
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.launch
import kotlinx.coroutines.withContext
import kotlinx.coroutines.flow.collectLatest
import kotlinx.coroutines.flow.filter
import kotlinx.coroutines.flow.onEach

/**
 * 应用级 ViewModel：连接管理、会话列表、消息流式接收。
 */
class AppViewModel(
    context: Context,
    private val scope: CoroutineScope,
) {
    private val prefs = context.getSharedPreferences("pai_chat_cache", Context.MODE_PRIVATE)
    private val client = PaiWsClient(scope)
    private val service = ChatService(client)
    private val gson = Gson()
    private val conversationListType = object : TypeToken<List<ConversationSummary>>() {}.type
    private var notificationJob: Job? = null
    private var connectJob: Job? = null
    private companion object {
        const val KEY_CONVERSATIONS = "conversations"
    }

    val connectionState: StateFlow<ConnectionStatus> = client.connectionState
    val conversations = MutableStateFlow<List<ConversationSummary>>(emptyList())
    val currentConversationId = MutableStateFlow<String?>(null)
    val messages = MutableStateFlow<List<ChatMessage>>(emptyList())
    val streamingText = MutableStateFlow("")
    /** 流式思考过程（activity_reasoning_delta），UI 折叠展示，非正文。 */
    val reasoningText = MutableStateFlow("")
    /** 流式工具调用列表（assistant_tool_event/result 聚合后的显示文本）。 */
    val toolEvents = MutableStateFlow<List<String>>(emptyList())
    val isStreaming = MutableStateFlow(false)
    val loading = MutableStateFlow(false)
    val error = MutableStateFlow<String?>(null)

    fun start() {
        loadCachedConversations()
        client.connect()
        notificationJob = scope.launch(Dispatchers.IO) {
            client.notifications.collectLatest { (method, params) ->
                handleNotification(method, params)
            }
        }
        // 后端连接建立后自动加载本地会话列表，避免用户手动刷新才看到对话
        connectJob = scope.launch(Dispatchers.IO) {
            client.connectionState
                .filter { it == ConnectionStatus.Connected }
                .onEach { refreshConversations() }
                .collectLatest { }
        }
    }

    fun stop() {
        notificationJob?.cancel()
        connectJob?.cancel()
        client.disconnect()
    }

    fun consumeError() {
        error.value = null
    }

    suspend fun refreshConversations() {
        withContext(Dispatchers.IO) {
            try {
                val result = service.listConversations()
                conversations.value = result.conversations
                saveConversationCache()
            } catch (e: Exception) {
                // 连接未就绪或后端未启动时不打断，保留本地缓存；提示仅用于用户主动刷新失败
                if (connectionState.value == ConnectionStatus.Connected) {
                    error.value = "刷新会话失败: ${e.message}"
                }
            }
        }
    }

    private fun loadCachedConversations() {
        val raw = prefs.getString(KEY_CONVERSATIONS, null) ?: return
        runCatching {
            gson.fromJson<MutableList<ConversationSummary>>(raw, conversationListType)
        }.getOrNull()?.let { conversations.value = it }
    }

    private fun saveConversationCache() {
        runCatching {
            prefs.edit().putString(KEY_CONVERSATIONS, gson.toJson(conversations.value)).apply()
        }
    }

    suspend fun createConversation(title: String? = null): String? {
        return withContext(Dispatchers.IO) {
            try {
                val result = service.createConversation(agentId = null, departmentId = null, title = title)
                val id = result.conversationId
                if (id != null) {
                    openConversation(id)
                    refreshConversations()
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
                service.resumeSubscription(conversationId)
                val page = service.blockPage(conversationId)
                currentConversationId.value = conversationId
                messages.value = page.messages
                streamingText.value = ""
                reasoningText.value = ""
                toolEvents.value = emptyList()
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
        reasoningText.value = ""
        toolEvents.value = emptyList()
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
        // 诊断：确认下行事件是否到达 Kotlin，及正文/思考/工具各自到达情况
        android.util.Log.d("PaiNotify", "method=$method convId=${params?.asJsonObject?.get("conversationId")} curr=${currentConversationId.value}")
        if (method == "chat.assistantDelta") {
            val diagEvent = params?.asJsonObject?.get("event")?.asJsonObject
            val diagKind = diagEvent?.get("kind")?.takeIf { !it.isJsonNull }?.asString ?: "null"
            val diagDelta = diagEvent?.get("delta")?.takeIf { !it.isJsonNull }?.asString ?: ""
            val diagMsg = diagEvent?.get("message")?.takeIf { !it.isJsonNull }?.asString ?: ""
            android.util.Log.d("PaiNotify", "assistantDelta kind=$diagKind deltaLen=${diagDelta.length} msgLen=${diagMsg.length}")
        }
        when (method) {
            "chat.assistantDelta" -> {
                val notif = params?.let { gson.fromJson(it, DeltaNotification::class.java) }
                val convId = notif?.conversationId
                if (convId != null && convId == currentConversationId.value) {
                    val event = notif.event
                    when (event?.kind) {
                        null, "", "stream" -> {
                            val delta = event?.delta ?: ""
                            if (delta.isNotEmpty()) {
                                streamingText.value = streamingText.value + delta
                            }
                        }
                        "activity_reasoning_delta" -> {
                            val r = event?.delta ?: ""
                            if (r.isNotEmpty()) {
                                reasoningText.value = reasoningText.value + r
                            }
                        }
                        "assistant_tool_event", "assistant_tool_result", "tool_status" -> {
                            // 工具调用过程：解析 message 中的工具名/状态，聚合为展示条目
                            val toolMsg = event?.message ?: ""
                            val toolName = event?.toolName?.takeIf { it.isNotBlank() }
                            val status = event?.toolStatus?.takeIf { it.isNotBlank() }
                            val label = when {
                                !toolName.isNullOrBlank() && !status.isNullOrBlank() -> "$toolName ($status)"
                                !toolName.isNullOrBlank() -> toolName
                                toolMsg.isNotBlank() -> toolMsg
                                else -> "工具"
                            }
                            val list = toolEvents.value.toMutableList()
                            if (list.lastOrNull() != label) list.add(label)
                            toolEvents.value = list
                        }
                        "round_completed" -> {
                            val msgJson = event?.message
                            val m = msgJson?.let {
                                runCatching { gson.fromJson(it, ai.easycall.app.model.DeltaMessage::class.java) }.getOrNull()
                            }
                            // 优先落带完整 parts（含 reasoningContent 的 assistantMessage），
                            // 避免仅用 assistantText 构造纯文本使思考在重进后丢失。
                            if (m != null && m.assistantMessage != null) {
                                commitAssistant("", m.assistantMessage!!)
                            } else if (m != null) {
                                val finalText = m.assistantText
                                if (finalText.isNullOrEmpty()) {
                                    finalizeStreaming()
                                } else {
                                    commitAssistant(finalText, null)
                                }
                            } else {
                                finalizeStreaming()
                            }
                            reasoningText.value = ""
                            toolEvents.value = emptyList()
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