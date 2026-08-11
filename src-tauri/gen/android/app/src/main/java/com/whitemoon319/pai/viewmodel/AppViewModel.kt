package com.whitemoon319.pai.viewmodel

import com.whitemoon319.pai.model.ActivityStep
import com.whitemoon319.pai.model.ChatMessage
import com.whitemoon319.pai.model.buildChatMessageFromActivitySteps
import com.whitemoon319.pai.model.ConversationSummary
import com.whitemoon319.pai.model.CreateConversationOptions
import com.whitemoon319.pai.model.DeltaNotification
import com.whitemoon319.pai.model.ToolHistoryEvent
import com.whitemoon319.pai.model.DeltaEvent
import com.whitemoon319.pai.ws.ChatService
import com.whitemoon319.pai.ws.ConnectionStatus
import com.whitemoon319.pai.ws.PaiWsClient
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue

/**
 * 应用级 ViewModel：连接管理、会话列表、消息流式接收。
 */
class AppViewModel(
    context: Context,
    private val scope: CoroutineScope,
) {
    private val appContext: Context = context.applicationContext
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
    /**
     * 流式活动步骤（思考与工具交错的有序列表），UI 按 rikkahub 语义：同一大类可分
     * 别折叠，但思考/工具各自又是一个可独立展开的步骤。正文仍在 [streamingText]。
     */
    val activitySteps = MutableStateFlow<List<ActivityStep>>(emptyList())
    val isStreaming = MutableStateFlow(false)
    val isRecording = MutableStateFlow(false)
    val recognizedText = MutableStateFlow<String?>(null)
    /** 当前待发送的附件（摄取后的 receipt）。 */
    val pendingAttachment = MutableStateFlow<com.whitemoon319.pai.model.AttachmentReceipt?>(null)
    val attaching = MutableStateFlow(false)
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
        // 后端连接建立后自动加载本地会话列表、补刷工具/工作区状态
        // （冷启动直接进工具页时 ws 未连，工具页进入时的拉取会失败，需连接后重试）
        connectJob = scope.launch(Dispatchers.IO) {
            client.connectionState
                .filter { it == ConnectionStatus.Connected }
                .collect {
                    withContext(Dispatchers.IO) { refreshConversations() }
                    refreshConnectedState()
                }
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

    /** 连接建立后补刷：工具状态 + 工作区状态（冷启动直接进工具页时 ws 未连，需连接后重试）。 */
    private suspend fun refreshConnectedState() {
        val agentId = currentConversationId.value?.let { id ->
            conversations.value.firstOrNull { it.conversationId == id }?.agentId
        }
        runCatching { toolStatus.value = service.checkToolsStatus(agentId) }
        runCatching { workspaceStatus.value = service.getAndroidWorkspaceStatus() }
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
                // 后端强制要求 departmentId+agentId（不含会报"新建会话必须选择部门/人格"）。
                // 先取 createOptions 的默认值兜底，保证点了新建有反馈、不静默失败。
                var agentId: String? = null
                var departmentId: String? = null
                runCatching {
                    val options = service.createConversationOptions()
                    departmentId = options.defaultDepartmentId?.takeIf { it.isNotBlank() }
                        ?: options.departments.firstOrNull()?.departmentId
                    agentId = options.defaultAgentId?.takeIf { it.isNotBlank() }
                        ?: options.departments.firstOrNull()?.agentId
                }
                if (departmentId.isNullOrBlank() || agentId.isNullOrBlank()) {
                    // createOptions 不可用时，退回到最近一个有部门/人格的会话
                    val conv = conversations.value.lastOrNull {
                        !it.departmentId.isNullOrBlank() && !it.agentId.isNullOrBlank()
                    }
                    departmentId = conv?.departmentId
                    agentId = conv?.agentId
                }
                createConversation(title, departmentId, agentId)
            } catch (e: Exception) {
                error.value = "新建会话失败: ${e.message}"
                null
            }
        }
    }

    /** 拉取新建会话可选的部门/人格及默认值，供自选用。失败返回空对象。 */
    suspend fun fetchCreateOptionsFull(): CreateConversationOptions {
        return withContext(Dispatchers.IO) {
            runCatching { service.createConversationOptions() }
                .getOrDefault(CreateConversationOptions())
        }
    }

    /** 用显式指定的部门/人格新建会话（自选入口）；不抛差归并到 [error]。 */
    suspend fun createConversation(
        title: String?,
        departmentId: String?,
        agentId: String?,
    ): String? {
        return withContext(Dispatchers.IO) {
            try {
                if (departmentId.isNullOrBlank() || agentId.isNullOrBlank()) {
                    error.value = "新建会话失败：未指定部门/人格"
                    return@withContext null
                }
                val result = service.createConversation(agentId = agentId, departmentId = departmentId, title = title)
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
                activitySteps.value = emptyList()
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
        val attachment = pendingAttachment.value
        if (trimmed.isEmpty() && attachment == null) return
        // 乐观显示用户消息
        val textPart = if (trimmed.isNotEmpty()) listOf(com.whitemoon319.pai.model.MessagePart(type = "Text", text = trimmed)) else emptyList()
        val attachPart = attachment?.let {
            listOf(
                com.whitemoon319.pai.model.MessagePart(
                    type = "Attachment",
                    text = it.fileName,
                    path = it.path,
                    mime = it.mime,
                )
            )
        } ?: emptyList()
        messages.value = messages.value.plus(
            ChatMessage(
                id = "local-${nextLocalId()}",
                role = "user",
                parts = textPart + attachPart,
            )
        )
        committedAssistantText = null
        streamingText.value = ""
        activitySteps.value = emptyList()
        isStreaming.value = true
        withContext(Dispatchers.IO) {
            try {
                if (attachment != null) {
                    service.sendWithAttachments(
                        conversationId, departmentId, agent, trimmed,
                        listOf(com.whitemoon319.pai.model.AttachmentMeta(
                            fileName = attachment.fileName,
                            path = attachment.path,
                            mime = attachment.mime,
                        )),
                    )
                } else {
                    service.send(conversationId, departmentId, agent, trimmed)
                }
            } catch (e: Exception) {
                error.value = "发送失败: ${e.message}"
                isStreaming.value = false
            }
        }
        pendingAttachment.value = null
    }

    /** 把复制到沙盒的附件文件摄取进后端，成功后存为待发送附件。 */
    suspend fun attachLocalFile(path: String, fileName: String?, mime: String?) {
        attaching.value = true
        try {
            val receipt = withContext(Dispatchers.IO) {
                service.ingestAttachment(path, fileName, mime)
            }
            pendingAttachment.value = receipt
        } catch (e: Exception) {
            error.value = "添加附件失败: ${e.message}"
        } finally {
            attaching.value = false
        }
    }

    fun clearPendingAttachment() {
        pendingAttachment.value = null
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

    /** 重命名会话。 */
    suspend fun renameConversation(conversationId: String, title: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.renameConversation(conversationId, title.trim())
                if (ok) refreshConversations()
                ok
            } catch (e: Exception) {
                error.value = "重命名失败: ${e.message}"
                false
            }
        }
    }

    /** 固定/取消固定会话。 */
    suspend fun toggleConversationPin(conversationId: String, pinned: Boolean): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.toggleConversationPin(conversationId, pinned)
                if (ok) refreshConversations()
                ok
            } catch (e: Exception) {
                error.value = "操作失败: ${e.message}"
                false
            }
        }
    }

    /** 删除会话。 */
    suspend fun deleteConversation(conversationId: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.deleteConversation(conversationId)
                if (ok) {
                    if (currentConversationId.value == conversationId) {
                        currentConversationId.value = null
                        messages.value = emptyList()
                        streamingText.value = ""
                        isStreaming.value = false
                    }
                    refreshConversations()
                }
                ok
            } catch (e: Exception) {
                error.value = "删除失败: ${e.message}"
                false
            }
        }
    }

    /** 回退到指定消息并重新生成（rewind → 清空其后续消息）。 */
    suspend fun rewindToMessage(messageId: String): Boolean {
        val conversationId = currentConversationId.value ?: return false
        val conv = conversations.value.firstOrNull { it.conversationId == conversationId }
        return withContext(Dispatchers.IO) {
            try {
                val result = service.rewindConversation(conversationId, conv?.departmentId, conv?.agentId, messageId)
                // 回退后刷新消息：移除 messageId 之后的所有消息
                val msgs = messages.value
                val targetIndex = msgs.indexOfFirst { it.id == messageId }
                if (targetIndex >= 0) {
                    messages.value = msgs.take(targetIndex + 1)
                } else {
                    refreshMessages()
                }
                streamingText.value = ""
                activitySteps.value = emptyList()
                committedAssistantText = null
                isStreaming.value = false
                true
            } catch (e: Exception) {
                error.value = "重新生成失败: ${e.message}"
                false
            }
        }
    }

    /** 重新加载当前会话消息（blockPage 最新页）。 */
    suspend fun refreshMessages() {
        val conversationId = currentConversationId.value ?: return
        withContext(Dispatchers.IO) {
            try {
                val page = service.blockPage(conversationId)
                messages.value = page.messages
            } catch (e: Exception) {
                error.value = "刷新消息失败: ${e.message}"
            }
        }
    }

    /** 归档会话。 */
    suspend fun archiveConversation(conversationId: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.batchArchiveConversations(listOf(conversationId))
                if (ok) {
                    if (currentConversationId.value == conversationId) {
                        currentConversationId.value = null
                        messages.value = emptyList()
                        streamingText.value = ""
                        isStreaming.value = false
                    }
                    refreshConversations()
                }
                ok
            } catch (e: Exception) {
                error.value = "归档失败: ${e.message}"
                false
            }
        }
    }

    // ---------------- 归档会话管理 ----------------

    val archives = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val archivesLoading = MutableStateFlow(false)

    suspend fun loadArchives() {
        withContext(Dispatchers.IO) {
            archivesLoading.value = true
            try {
                archives.value = service.listArchives()
            } catch (e: Exception) {
                error.value = "读取归档失败: ${e.message}"
            } finally {
                archivesLoading.value = false
            }
        }
    }

    /** 从归档恢复会话（unarchive 后刷新列表）。 */
    suspend fun unarchive(archiveId: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.unarchiveArchive(archiveId)
                if (ok) {
                    loadArchives()
                    refreshConversations()
                }
                ok
            } catch (e: Exception) {
                error.value = "恢复归档失败: ${e.message}"
                false
            }
        }
    }

    /** 删除归档。 */
    suspend fun deleteArchive(archiveId: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.deleteArchive(archiveId)
                if (ok) loadArchives()
                ok
            } catch (e: Exception) {
                error.value = "删除归档失败: ${e.message}"
                false
            }
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
                                appendReasoningDelta(r)
                            }
                        }
                        "assistant_tool_event" -> {
                            // 工具调用开始：解析 message 中的工具名/参数/工具级思考，追加或更新工具步骤
                            val toolMsg = event?.message
                            android.util.Log.d("PaiNotify", "TOOL_EVENT kind=${event?.kind} msg=$toolMsg")
                            if (toolMsg.isNullOrBlank()) return
                            upsertToolStep(toolMsg, event)
                        }
                        "assistant_tool_result" -> {
                            val toolMsg = event?.message
                            android.util.Log.d("PaiNotify", "TOOL_RESULT kind=${event?.kind} msg=$toolMsg")
                            if (toolMsg.isNullOrBlank()) return
                            // 工具结果：更新最近一次工具步骤的 resultText
                            updateToolResult(toolMsg, event)
                        }
                        "tool_status" -> {
                            // 阶段提示（正在准备调度/处理附件/进入模型请求）无具体工具名，不在前台气泡展示。
                            // 仅当带 tool_name 的真实工具状态出现时才落到工具步骤上。
                            val toolName = event?.toolName?.takeIf { it.isNotBlank() }
                            if (!toolName.isNullOrBlank()) {
                                upsertToolStep("", event, fallbackName = toolName)
                            }
                        }
                        "round_completed" -> {
                            val msgJson = event?.message
                            val m = msgJson?.let {
                                runCatching { gson.fromJson(it, com.whitemoon319.pai.model.DeltaMessage::class.java) }.getOrNull()
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
                            activitySteps.value = emptyList()
                            finalizeStreaming()
                        }
                        "round_failed" -> {
                            val msgJson = event?.message
                            val errText = msgJson?.let {
                                runCatching { gson.fromJson(it, com.whitemoon319.pai.model.DeltaMessage::class.java) }.getOrNull()?.assistantText
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
            "app.notification" -> {
                handleNativeNotification(params)
            }
            "app.notification.clear" -> {
                handleNativeNotificationClear(params)
            }
            "app.keepAlive" -> {
                handleNativeKeepAlive(params)
            }
        }
    }

    // ---------------- 原生通知（Rust live_update 事件队列） ----------------

    private fun handleNativeNotification(params: JsonElement?) {
        try {
            val obj = params?.asJsonObject ?: return
            val id = obj.get("id")?.takeIf { !it.isJsonNull }?.asInt ?: return
            val title = obj.get("title")?.takeIf { !it.isJsonNull }?.asString ?: return
            val body = obj.get("body")?.takeIf { !it.isJsonNull }?.asString ?: return
            val ongoing = obj.get("ongoing")?.takeIf { !it.isJsonNull }?.asBoolean ?: false
            val manager = appContext.getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            val channelId = "pai_live_updates"
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
                val channel = android.app.NotificationChannel(
                    channelId,
                    "PAI 实时更新",
                    android.app.NotificationManager.IMPORTANCE_DEFAULT
                ).apply { description = "会话回复与目标进度通知" }
                manager.createNotificationChannel(channel)
            }
            val builder = android.app.Notification.Builder(appContext, channelId)
                .setSmallIcon(android.R.drawable.ic_dialog_info)
                .setContentTitle(title)
                .setContentText(body)
                .setOngoing(ongoing)
                .setAutoCancel(!ongoing)
            val shortText = obj.get("shortText")?.takeIf { !it.isJsonNull }?.asString
            if (!shortText.isNullOrEmpty()) {
                builder.setSubText(shortText)
            }
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.M) {
                builder.setColor(0xFF4C6FFF.toInt())
            }
            if (android.os.Build.VERSION.SDK_INT >= android.os.Build.VERSION_CODES.O) {
                builder.setChannelId(channelId)
            }
            manager.notify(id, builder.build())
        } catch (e: Exception) {
            android.util.Log.w("PaiNotify", "原生通知发送失败: ${e.message}")
        }
    }

    private fun handleNativeNotificationClear(params: JsonElement?) {
        try {
            val obj = params?.asJsonObject ?: return
            val id = obj.get("id")?.takeIf { !it.isJsonNull }?.asInt ?: return
            val manager = appContext.getSystemService(Context.NOTIFICATION_SERVICE) as android.app.NotificationManager
            manager.cancel(id)
        } catch (_: Exception) {
        }
    }

    private fun handleNativeKeepAlive(params: JsonElement?) {
        // 保活通知：维持/移除前台服务感知的常驻通知。
        // 当前实现与普通通知共用通道；active=false 时由前端自行决定是否清空常驻通知。
        try {
            val obj = params?.asJsonObject ?: return
            val active = obj.get("active")?.takeIf { !it.isJsonNull }?.asBoolean ?: false
            android.util.Log.d("PaiNotify", "keepAlive active=$active")
        } catch (_: Exception) {
        }
    }

    // ---------------- 流式活动步骤累积 ----------------

    /** 追加一段思考过程：若上一步是 Reasoning 则续接，否则新建一个 Reasoning 步骤。 */
    private fun appendReasoningDelta(delta: String) {
        val list = activitySteps.value.toMutableList()
        val last = list.lastOrNull()
        if (last is ActivityStep.Reasoning) {
            list[list.size - 1] = last.copy(text = last.text + delta)
        } else {
            list.add(ActivityStep.Reasoning(delta))
        }
        activitySteps.value = list
    }

    /**
     * 追加或更新一个工具步骤。解析 message（与落盘 tool history event 同构）
     * 中的工具名/参数/工具级思考；已存在的同一 tool_call_id 只更新状态与结果。
     */
    private fun upsertToolStep(toolMsg: String, event: DeltaEvent?, fallbackName: String? = null) {
        var name = fallbackName
        var args: String? = null
        var toolCallId: String? = null
        var toolReasoning: String? = null
        if (toolMsg.isNotBlank()) {
            val parsed = runCatching { gson.fromJson(toolMsg, ToolHistoryEvent::class.java) }.getOrNull()
            val calls = parsed?.toolCalls.orEmpty()
            val first = calls.firstOrNull()
            if (calls.isNotEmpty()) {
                name = first?.function?.name?.takeIf { it.isNotBlank() } ?: name
                args = first?.function?.arguments?.takeIf { it.isNotBlank() }
                toolCallId = first?.id ?: first?.callId
            }
            // 工具级思考挂在事件上，仅首个工具调用持有
            toolReasoning = parsed?.reasoningContent?.takeIf { it.isNotBlank() }
        }
        val status = event?.toolStatus?.takeIf { it.isNotBlank() } ?: "doing"
        val list = activitySteps.value.toMutableList()
        // 同一 tool_call_id 的工具步骤存在则更新状态（结果的更新走 updateToolResult）
        val existingIndex = toolCallId?.let { id ->
            list.indexOfFirst { it is ActivityStep.Tool && it.toolCallId == id }
        }
        if (existingIndex != null && existingIndex >= 0) {
            val prev = list[existingIndex] as ActivityStep.Tool
            list[existingIndex] = prev.copy(name = name ?: prev.name, status = status)
        } else {
            // 新工具步骤：追加到最后一个工具步骤之后（保持与思考交错顺序）
            list.add(
                ActivityStep.Tool(
                    toolCallId = toolCallId,
                    name = name ?: "工具",
                    argsText = args,
                    resultText = null,
                    status = status,
                    reasoning = toolReasoning,
                )
            )
        }
        activitySteps.value = list
    }

    /** 工具结果：把 result 追加到最近一次工具步骤的 resultText。 */
    private fun updateToolResult(toolMsg: String, event: DeltaEvent?) {
        val parsed = runCatching { gson.fromJson(toolMsg, ToolHistoryEvent::class.java) }.getOrNull()
        val result = parsed?.content?.takeIf { it.isNotBlank() }
        if (result == null) return
        // result 事件（role=tool）的 id 在顶层 tool_call_id，而非 tool_calls 数组
        val toolCallId = parsed?.toolCallId?.takeIf { it.isNotBlank() }
        val list = activitySteps.value.toMutableList()
        var targetIndex = -1
        if (toolCallId != null) {
            targetIndex = list.indexOfLast { it is ActivityStep.Tool && it.toolCallId == toolCallId }
        }
        if (targetIndex < 0) {
            // 兜底：匹配最近一个状态不是 done 的工具步骤（正在执行中），避免回复到已完成步骤
            targetIndex = list.indexOfLast {
                it is ActivityStep.Tool && it.status != "done"
            }
        }
        if (targetIndex < 0) {
            targetIndex = list.indexOfLast { it is ActivityStep.Tool }
        }
        if (targetIndex >= 0) {
            val prev = list[targetIndex] as ActivityStep.Tool
            list[targetIndex] = prev.copy(resultText = result, status = "done")
        } else {
            val name = event?.toolName?.takeIf { it.isNotBlank() } ?: "工具"
            list.add(
                ActivityStep.Tool(
                    toolCallId = toolCallId,
                    name = name,
                    argsText = null,
                    resultText = result,
                    status = "done",
                    reasoning = null,
                )
            )
        }
        activitySteps.value = list
    }

    private val idSeq = java.util.concurrent.atomic.AtomicLong(0)
    private fun nextLocalId() = "${System.currentTimeMillis()}-${idSeq.incrementAndGet()}"

    /** 记录当前回合已落地的 assistant 文本，避免 round_completed 与 roundFinished 重复落盘。 */
    private var committedAssistantText: String? = null

    private fun commitAssistant(text: String, message: ChatMessage?) {
        val trimmed = text?.trim().orEmpty()
        val steps = activitySteps.value
        if (!trimmed.isEmpty()) {
            if (committedAssistantText == trimmed) {
                // 本回合已落过同意文本，只清理流式缓冲，不再重复落盘
                streamingText.value = ""
                activitySteps.value = emptyList()
                return
            }
            committedAssistantText = trimmed
            // 文本落盘时并入当前活动步骤（思考+工具），保证思考在正文本体上方、不残留到列表尾部
            val built = if (steps.isNotEmpty()) {
                buildChatMessageFromActivitySteps(
                    id = "assistant-${nextLocalId()}",
                    role = "assistant",
                    assistantText = trimmed,
                    steps = steps,
                )
            } else {
                ChatMessage(
                    id = "assistant-${nextLocalId()}",
                    role = "assistant",
                    parts = listOf(com.whitemoon319.pai.model.MessagePart(type = "Text", text = trimmed)),
                )
            }
            messages.value = messages.value.plus(built)
            streamingText.value = ""
            activitySteps.value = emptyList()
            return
        }
        if (message != null) {
            // message 分支去重：若本回合已落过相同正文（文本分支先到），直接跳过不再落盘
            val bodyText = message.parts.joinToString("\n") { it.displayText }.trim()
            if (!bodyText.isEmpty() && committedAssistantText == bodyText) {
                streamingText.value = ""
                activitySteps.value = emptyList()
                return
            }
            committedAssistantText = bodyText
            messages.value = messages.value.plus(message)
            streamingText.value = ""
            activitySteps.value = emptyList()
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

    // ==================== 设置 ====================

    val appConfig = MutableStateFlow<com.whitemoon319.pai.model.AppConfig?>(null)
    val chatSettings = MutableStateFlow<com.whitemoon319.pai.model.ChatSettings?>(null)
    val toolStatus = MutableStateFlow<List<com.whitemoon319.pai.model.ToolLoadStatus>>(emptyList())
    val bootstrap = MutableStateFlow<com.whitemoon319.pai.model.BootstrapSnapshot?>(null)
    val settingsLoading = MutableStateFlow(false)
    val settingsSaving = MutableStateFlow(false)
    /** Web 访问（远程连接）状态快照。 */
    val webAccessInfo = MutableStateFlow<Map<String, Any?>?>(null)
    val webAccessLoading = MutableStateFlow(false)

    /** 刷新 Web 访问状态（远程连接）。 */
    suspend fun refreshWebAccessInfo(forceRefresh: Boolean = false) {
        withContext(Dispatchers.IO) {
            webAccessLoading.value = true
            try {
                webAccessInfo.value = service.getWebAccessInfo(forceRefresh)
            } catch (e: Exception) {
                error.value = "读取远程连接状态失败: ${e.message}"
            } finally {
                webAccessLoading.value = false
            }
        }
    }

    /** 保存 Web 访问配置（开关/端口/密码），保存后自动重启服务。 */
    suspend fun saveWebAccess(enabled: Boolean, port: Int, password: String): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                val current = appConfig.value ?: service.loadConfig()
                val updated = current.copy(
                    webAccessEnabled = enabled,
                    webAccessPort = port,
                    webAccessPassword = password,
                )
                appConfig.value = service.saveConfig(updated)
                refreshWebAccessInfo(forceRefresh = true)
                true
            } catch (e: Exception) {
                error.value = "保存远程连接设置失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    /** 保存语音识别（STT）供应商选择。 */
    suspend fun saveSttApiConfig(sttApiConfigId: String?): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                val current = appConfig.value ?: service.loadConfig()
                val updated = current.copy(sttApiConfigId = sttApiConfigId)
                appConfig.value = service.saveConfig(updated)
                true
            } catch (e: Exception) {
                error.value = "保存语音供应商失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    // ---------------- Vue 设置页对齐：通知 / 外观 ----------------

    /** 保存通知与外观设置（patch 语义：仅覆盖可编辑字段，避免全量保存丢字段）。 */
    suspend fun saveNotificationAndAppearance(
        messageNotificationEnabled: Boolean? = null,
        messageNotificationSoundEnabled: Boolean? = null,
        desktopOperationNoticeEnabled: Boolean? = null,
        uiLanguage: String? = null,
        uiSizeScale: Int? = null,
    ): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                val current = appConfig.value ?: service.loadConfig()
                val updated = current.copy(
                    messageNotificationEnabled = messageNotificationEnabled ?: current.messageNotificationEnabled,
                    messageNotificationSoundEnabled = messageNotificationSoundEnabled ?: current.messageNotificationSoundEnabled,
                    desktopOperationNoticeEnabled = desktopOperationNoticeEnabled ?: current.desktopOperationNoticeEnabled,
                    uiLanguage = uiLanguage ?: current.uiLanguage,
                    uiSizeScale = uiSizeScale ?: current.uiSizeScale,
                )
                appConfig.value = service.saveConfig(updated)
                true
            } catch (e: Exception) {
                error.value = "保存设置失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    // ---------------- Vue 设置页对齐：记忆 / 日志 / 存储 / 用量 / MCP / 任务 / 远程IM ----------------

    val memories = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val memoryLoading = MutableStateFlow(false)

    // ---------------- 人设（Persona）管理 ----------------

    val agents = MutableStateFlow<List<com.whitemoon319.pai.model.AgentProfile>?>(null)
    val agentsLoading = MutableStateFlow(false)

    suspend fun loadAgents() {
        withContext(Dispatchers.IO) {
            agentsLoading.value = true
            try {
                agents.value = service.loadAgents()
            } catch (e: Exception) {
                error.value = "读取人设失败: ${e.message}"
            } finally {
                agentsLoading.value = false
            }
        }
    }

    /** 保存人设编辑（全量写回）。 */
    suspend fun saveAgents(agents: List<com.whitemoon319.pai.model.AgentProfile>): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                val ok = service.saveAgents(agents)
                if (ok) loadAgents()
                ok
            } catch (e: Exception) {
                error.value = "保存人设失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    suspend fun loadMemories() {
        withContext(Dispatchers.IO) {
            memoryLoading.value = true
            try {
                val result = service.listMemories()
                @Suppress("UNCHECKED_CAST")
                memories.value = (result["memories"] as? List<Map<String, Any?>>) ?: emptyList()
            } catch (e: Exception) {
                error.value = "读取记忆失败: ${e.message}"
            } finally {
                memoryLoading.value = false
            }
        }
    }

    suspend fun deleteMemory(memoryId: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                val ok = service.deleteMemory(memoryId)
                if (ok) loadMemories()
                ok
            } catch (e: Exception) {
                error.value = "删除记忆失败: ${e.message}"
                false
            }
        }
    }

    /** 记忆回忆搜索结果（搜索命中列表）。 */
    val memorySearchResults = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val memorySearching = MutableStateFlow(false)

    /** 记忆回忆搜索：优先当前会话 agent，否则用默认。 */
    suspend fun searchMemories(query: String) {
        if (query.isBlank()) {
            memorySearchResults.value = null
            return
        }
        withContext(Dispatchers.IO) {
            memorySearching.value = true
            try {
                val agentId = currentConversationId.value?.let { id ->
                    conversations.value.firstOrNull { it.conversationId == id }?.agentId
                } ?: "agent"
                val result = service.searchMemoriesRecall(agentId, query.trim())
                @Suppress("UNCHECKED_CAST")
                memorySearchResults.value = (result["memories"] as? List<Map<String, Any?>>) ?: emptyList()
            } catch (e: Exception) {
                error.value = "搜索记忆失败: ${e.message}"
            } finally {
                memorySearching.value = false
            }
        }
    }

    fun clearMemorySearch() {
        memorySearchResults.value = null
    }

    val runtimeLogs = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val runtimeLogsLoading = MutableStateFlow(false)

    suspend fun loadRuntimeLogs() {
        withContext(Dispatchers.IO) {
            runtimeLogsLoading.value = true
            try {
                runtimeLogs.value = service.listRecentRuntimeLogs()
            } catch (e: Exception) {
                error.value = "读取日志失败: ${e.message}"
            } finally {
                runtimeLogsLoading.value = false
            }
        }
    }

    val storageOverview = MutableStateFlow<Map<String, Any?>?>(null)
    val storageLoading = MutableStateFlow(false)

    suspend fun loadStorageOverview(refresh: Boolean = false) {
        withContext(Dispatchers.IO) {
            storageLoading.value = true
            try {
                storageOverview.value = if (refresh) service.refreshStorageUsageOverview() else service.getStorageUsageOverview()
            } catch (e: Exception) {
                error.value = "读取存储用量失败: ${e.message}"
            } finally {
                storageLoading.value = false
            }
        }
    }

    val usageOverview = MutableStateFlow<Map<String, Any?>?>(null)
    val usageLoading = MutableStateFlow(false)

    suspend fun loadUsageOverview() {
        withContext(Dispatchers.IO) {
            usageLoading.value = true
            try {
                usageOverview.value = service.getUsageOverview()
            } catch (e: Exception) {
                error.value = "读取用量失败: ${e.message}"
            } finally {
                usageLoading.value = false
            }
        }
    }

    val mcpServers = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val mcpLoading = MutableStateFlow(false)

    suspend fun loadMcpServers() {
        withContext(Dispatchers.IO) {
            mcpLoading.value = true
            try {
                mcpServers.value = service.mcpListServers()
            } catch (e: Exception) {
                error.value = "读取 MCP 服务器失败: ${e.message}"
            } finally {
                mcpLoading.value = false
            }
        }
    }

    val tasks = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val tasksLoading = MutableStateFlow(false)

    suspend fun loadTasks() {
        withContext(Dispatchers.IO) {
            tasksLoading.value = true
            try {
                tasks.value = service.taskListTasks()
            } catch (e: Exception) {
                error.value = "读取任务失败: ${e.message}"
            } finally {
                tasksLoading.value = false
            }
        }
    }

    val remoteImChannels = MutableStateFlow<List<Map<String, Any?>>?>(null)
    val remoteImLoading = MutableStateFlow(false)

    suspend fun loadRemoteImChannels() {
        withContext(Dispatchers.IO) {
            remoteImLoading.value = true
            try {
                remoteImChannels.value = service.remoteImListChannels()
            } catch (e: Exception) {
                error.value = "读取远程 IM 通道失败: ${e.message}"
            } finally {
                remoteImLoading.value = false
            }
        }
    }

    /** 加载设置页全部数据（配置/聊天设置/工具状态/关于）。agentId 用于工具状态。 */
    suspend fun loadSettings(agentId: String?) {
        withContext(Dispatchers.IO) {
            settingsLoading.value = true
            try {
                runCatching { appConfig.value = service.loadConfig() }
                runCatching { chatSettings.value = service.loadChatSettings() }
                runCatching { toolStatus.value = service.checkToolsStatus(agentId) }
                runCatching { bootstrap.value = service.bootstrapSnapshot() }
            } finally {
                settingsLoading.value = false
            }
        }
    }

    /** 切换部门主 API 配置（全局生效，不动其他配置）。 */
    suspend fun switchPrimaryApiConfig(apiConfigId: String): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                val departmentId = defaultDepartmentId()
                    ?: run {
                        error.value = "切换失败：无法确定部门"
                        return@withContext false
                    }
                val updated = service.setDepartmentPrimaryApiConfig(departmentId, apiConfigId)
                appConfig.value = updated
                true
            } catch (e: Exception) {
                error.value = "切换供应商失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    /** 保存聊天设置（patch 语义，只回传用户可改字段）。 */
    suspend fun saveChatSettings(
        alias: String?,
        responseStyleId: String?,
        pdfReadMode: String? = null,
        instructionPresets: List<com.whitemoon319.pai.model.PromptCommandPreset>? = null,
    ): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                val updated = service.saveChatSettings(
                    com.whitemoon319.pai.model.ChatSettings(
                        userAlias = alias?.takeIf { it.isNotBlank() },
                        responseStyleId = responseStyleId?.takeIf { it.isNotBlank() },
                        pdfReadMode = pdfReadMode?.takeIf { it.isNotBlank() },
                        instructionPresets = instructionPresets ?: emptyList(),
                    )
                )
                chatSettings.value = updated
                true
            } catch (e: Exception) {
                error.value = "保存聊天设置失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    private suspend fun defaultDepartmentId(): String? {
        return runCatching {
            service.createConversationOptions().defaultDepartmentId?.takeIf { it.isNotBlank() }
        }.getOrNull()
    }

    // ==================== 设置：供应商 CRUD / 工作区 / 关于 ====================

    /** 新增供应商并刷新配置。 */
    suspend fun createApiConfig(config: com.whitemoon319.pai.model.ApiConfig): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                appConfig.value = service.createApiConfig(config)
                true
            } catch (e: Exception) {
                error.value = "新增供应商失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    /** 更新供应商并刷新配置。 */
    suspend fun updateApiConfig(config: com.whitemoon319.pai.model.ApiConfig): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                appConfig.value = service.updateApiConfig(config)
                true
            } catch (e: Exception) {
                error.value = "更新供应商失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    /** 删除供应商并刷新配置。 */
    suspend fun deleteApiConfig(id: String): Boolean {
        return withContext(Dispatchers.IO) {
            settingsSaving.value = true
            try {
                appConfig.value = service.deleteApiConfig(id)
                true
            } catch (e: Exception) {
                error.value = "删除供应商失败: ${e.message}"
                false
            } finally {
                settingsSaving.value = false
            }
        }
    }

    /** 切换当前会话的首选模型（供应商）。 */
    suspend fun setConversationPreferredModel(conversationId: String, preferredApiConfigId: String?): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                service.setConversationPreferredModel(conversationId, preferredApiConfigId)
                true
            } catch (e: Exception) {
                error.value = "切换模型失败: ${e.message}"
                false
            }
        }
    }

    // ---------------- 语音输入 ----------------

    private var audioRecorder: android.media.MediaRecorder? = null
    private var audioFile: java.io.File? = null

    /** 开始录音（需先在 UI 层申请 RECORD_AUDIO 权限）。 */
    fun startRecording() {
        if (isRecording.value) return
        try {
            val dir = java.io.File(appContext.cacheDir, "stt").apply { mkdirs() }
            val file = java.io.File(dir, "rec_${System.currentTimeMillis()}.m4a")
            val recorder = android.media.MediaRecorder()
            recorder.setAudioSource(android.media.MediaRecorder.AudioSource.MIC)
            recorder.setOutputFormat(android.media.MediaRecorder.OutputFormat.MPEG_4)
            recorder.setAudioEncoder(android.media.MediaRecorder.AudioEncoder.AAC)
            recorder.setAudioSamplingRate(16000)
            recorder.setAudioEncodingBitRate(64000)
            recorder.setOutputFile(file.absolutePath)
            recorder.prepare()
            recorder.start()
            audioRecorder = recorder
            audioFile = file
            isRecording.value = true
        } catch (e: Exception) {
            error.value = "录音启动失败: ${e.message}"
        }
    }

    /** 停止录音并转文字，成功后把文本写入 error 状态供 UI 回填。 */
    fun stopAndTranscribe() {
        val recorder = audioRecorder ?: return
        val file = audioFile
        try {
            recorder.stop()
            recorder.release()
        } catch (e: Exception) {
            // MediaRecorder.stop 可能因时长过短抛异常，忽略
            error.value = "录音过短或失败: ${e.message}"
        } finally {
            audioRecorder = null
            audioFile = null
            isRecording.value = false
        }
        if (file == null || !file.exists() || file.length() == 0L) return
        scope.launch(Dispatchers.IO) {
            try {
                val bytes = file.readBytes()
                val b64 = android.util.Base64.encodeToString(bytes, android.util.Base64.NO_WRAP)
                val text = service.sttTranscribe("audio/mp4", b64, null)
                if (!text.isNullOrBlank()) {
                    withContext(Dispatchers.Main) { recognizedText.value = text }
                } else {
                    error.value = "语音识别无结果"
                }
            } catch (e: Exception) {
                error.value = "语音识别失败: ${e.message}"
            } finally {
                file.delete()
            }
        }
    }

    /** 连接测试：发一条极短请求验证供应商连通性，返回回复文本或错误。 */
    suspend fun testApiConfigConnection(config: com.whitemoon319.pai.model.ApiConfig): String? {
        return withContext(Dispatchers.IO) {
            try {
                service.testTextConnection(config)
            } catch (e: Exception) {
                error.value = "连接测试失败: ${e.message}"
                null
            }
        }
    }

    val workspaceStatus = MutableStateFlow<com.whitemoon319.pai.model.AndroidWorkspaceStatus?>(null)
    val workspaceBusy = MutableStateFlow(false)
    val workspaceFiles = MutableStateFlow<com.whitemoon319.pai.model.WorkspaceFileListResult?>(null)
    val workspaceDir = MutableStateFlow<String?>(null)

    suspend fun refreshWorkspaceStatus() {
        withContext(Dispatchers.IO) {
            runCatching { workspaceStatus.value = service.getAndroidWorkspaceStatus() }
                .onFailure { e ->
                    // 暴露真实错误便于排障（连接未就绪或后端方法失败）
                    error.value = "刷新工作区状态失败: ${e.message}"
                    android.util.Log.e("WS", "getAndroidWorkspaceStatus failed", e)
                }
        }
    }

    /** 列出工作区目录（path 为相对路径，null 表示根）。 */
    suspend fun listWorkspaceDir(path: String?) {
        withContext(Dispatchers.IO) {
            try {
                workspaceFiles.value = service.listWorkspaceFiles(path)
                workspaceDir.value = path
            } catch (e: Exception) {
                error.value = "读取工作区失败: ${e.message}"
            }
        }
    }

    suspend fun readWorkspaceFile(path: String): com.whitemoon319.pai.model.WorkspaceTextResult? {
        return withContext(Dispatchers.IO) {
            try {
                service.readWorkspaceText(path)
            } catch (e: Exception) {
                error.value = "读取文件失败: ${e.message}"
                null
            }
        }
    }

    suspend fun writeWorkspaceFile(path: String, text: String, overwrite: Boolean): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                service.writeWorkspaceText(path, text, overwrite)
                true
            } catch (e: Exception) {
                error.value = "写入文件失败: ${e.message}"
                false
            }
        }
    }

    suspend fun deleteWorkspaceFile(path: String): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                service.deleteWorkspaceFile(path)
                true
            } catch (e: Exception) {
                error.value = "删除文件失败: ${e.message}"
                false
            }
        }
    }

    suspend fun moveWorkspaceFile(source: String, target: String, overwrite: Boolean): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                service.moveWorkspaceFile(source, target, overwrite)
                true
            } catch (e: Exception) {
                error.value = "移动/重命名失败: ${e.message}"
                false
            }
        }
    }

    suspend fun importWorkspaceFile(fileName: String, dataBase64: String, targetPath: String?): Boolean {
        return withContext(Dispatchers.IO) {
            try {
                service.importWorkspaceFile(fileName, null, dataBase64, targetPath)
                true
            } catch (e: Exception) {
                error.value = "导入失败: ${e.message}"
                false
            }
        }
    }

    suspend fun exportWorkspaceFile(path: String): com.whitemoon319.pai.model.WorkspaceExportResult? {
        return withContext(Dispatchers.IO) {
            try {
                service.exportWorkspaceFile(path)
            } catch (e: Exception) {
                error.value = "导出失败: ${e.message}"
                null
            }
        }
    }

    suspend fun serviceGrep(query: String, path: String?): List<com.whitemoon319.pai.model.WorkspaceSearchMatch> {
        return withContext(Dispatchers.IO) {
            try {
                service.grepWorkspaceFiles(query, path, regex = false, ignoreCase = true, includeGlob = null).matches
            } catch (e: Exception) {
                error.value = "搜索失败: ${e.message}"
                emptyList()
            }
        }
    }

    private suspend fun runWorkspaceAction(
        action: suspend () -> com.whitemoon319.pai.model.AndroidWorkspaceStatus,
    ) {
        withContext(Dispatchers.IO) {
            workspaceBusy.value = true
            try {
                workspaceStatus.value = action()
            } catch (e: Exception) {
                error.value = "工作区操作失败: ${e.message}"
            } finally {
                workspaceBusy.value = false
            }
        }
    }

    suspend fun initWorkspace() = runWorkspaceAction { service.initAndroidWorkspace() }
    suspend fun repairWorkspace() = runWorkspaceAction { service.repairAndroidWorkspaceRuntime() }
    suspend fun resetWorkspaceRuntime() = runWorkspaceAction { service.resetAndroidWorkspaceRuntime() }
    suspend fun resetWorkspaceState() = runWorkspaceAction { service.resetAndroidWorkspaceState() }

    val appVersion = MutableStateFlow<String?>(null)
    val repoUrl = MutableStateFlow<String?>(null)
    val updateResult = MutableStateFlow<String?>(null)

    suspend fun loadAboutInfo() {
        withContext(Dispatchers.IO) {
            runCatching { appVersion.value = service.getAppVersion() }
            runCatching { repoUrl.value = service.getProjectRepositoryUrl() }
        }
    }

    suspend fun checkUpdate() {
        withContext(Dispatchers.IO) {
            updateResult.value = try {
                val info = service.checkGithubUpdate()
                info["message"]?.toString() ?: info.toString()
            } catch (e: Exception) {
                "检查更新失败: ${e.message}"
            }
        }
    }
}