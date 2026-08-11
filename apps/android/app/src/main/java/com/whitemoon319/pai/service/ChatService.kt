package com.whitemoon319.pai.service

import com.whitemoon319.pai.model.BlockPageInput
import com.whitemoon319.pai.model.BlockPageResult
import com.whitemoon319.pai.model.ConversationListResult
import com.whitemoon319.pai.model.CreateConversationInput
import com.whitemoon319.pai.model.CreateConversationOptions
import com.whitemoon319.pai.model.CreateConversationResult
import com.whitemoon319.pai.model.SendChatRequest
import com.whitemoon319.pai.model.SessionSelector
import com.whitemoon319.pai.model.SetActiveInput
import com.whitemoon319.pai.model.SetActiveResult
import com.whitemoon319.pai.model.SubmitChatResult
import com.whitemoon319.pai.bridge.NativeRpcClient

/**
 * 聊天相关协议方法门面，直接封装 JSON-RPC 调用。
 * 方法名与 jsonrpc_dispatch.rs 的 ws method 一致。
 *
 * 超时语义：普通查询走默认短超时（15s）；聊天发送、迁移、工作区初始化/
 * 修复/重置、rootfs 导入等长任务走 callLong（600s），避免大文件/深迁移
 * 在 15s 被误判失败。
 */
class ChatService(private val client: NativeRpcClient) {

    /** 长任务请求（独立超时），供迁移/工作区/rootfs 导入等使用。 */
    private suspend fun <T> requestLong(method: String, params: Any?, clazz: Class<T>): T =
        client.requestLong(method, params, clazz)

    suspend fun listConversations(): ConversationListResult =
        client.request("conversation.list", emptyMap<String, Any?>(), ConversationListResult::class.java)

    suspend fun createConversation(agentId: String?, departmentId: String?, title: String?): CreateConversationResult {
        val input = CreateConversationInput(agentId = agentId, departmentId = departmentId, title = title)
        return client.request("conversation.create", input, CreateConversationResult::class.java)
    }

    /** 取新建会话可选的部门/人格与默认值。 */
    suspend fun createConversationOptions(): CreateConversationOptions =
        client.request("conversation.createOptions", emptyMap<String, Any?>(), CreateConversationOptions::class.java)

    suspend fun setActive(conversationId: String, agentId: String?): SetActiveResult {
        val input = SetActiveInput(conversationId = conversationId, agentId = agentId)
        return client.request("conversation.setActive", mapOf("input" to input), SetActiveResult::class.java)
    }

    /** 标记会话已读（清零未读计数）。 */
    suspend fun markRead(conversationId: String): Boolean =
        client.request("conversation.markRead", mapOf("conversationId" to conversationId), Boolean::class.java)

    /** 切换会话首选模型（供应商）。 */
    suspend fun setConversationPreferredModel(conversationId: String, preferredApiConfigId: String?): Map<String, Any?> {
        val input = mapOf(
            "conversationId" to conversationId,
            "preferredApiConfigId" to preferredApiConfigId,
        )
        return client.request(
            "conversation.setPreferredModel",
            mapOf("input" to input),
            object : com.google.gson.reflect.TypeToken<Map<String, Any?>>() {}.type,
        )
    }

    /** 语音转文字（STT）。 */
    suspend fun sttTranscribe(mime: String, bytesBase64: String, sttApiConfigId: String? = null): String? {
        val input = mapOf(
            "mime" to mime,
            "bytesBase64" to bytesBase64,
            "sttApiConfigId" to sttApiConfigId,
        )
        val result: Map<String, Any?> = client.request(
            "stt_transcribe",
            input,
            object : com.google.gson.reflect.TypeToken<Map<String, Any?>>() {}.type,
        )
        return result["text"]?.toString()
    }

    /** 登记为当前会话的 sidebar 订阅者，使后端把流式 delta / 思考 / 工具事件广播到此连接。 */
    suspend fun resumeSubscription(conversationId: String) {
        client.sendOneWay(
            "conversation.resumeSubscription",
            mapOf("conversationId" to conversationId),
        )
    }

    suspend fun blockPage(conversationId: String, blockId: Int? = null): BlockPageResult {
        val input = BlockPageInput(conversationId = conversationId, blockId = blockId)
        return client.request("conversation.blockPage", input, BlockPageResult::class.java)
    }

    /** 加载更早消息（conversation.messagesBefore）。返回 (messages, 是否还有更多)。 */
    suspend fun messagesBefore(conversationId: String, beforeMessageId: String, limit: Int = 30): Pair<List<com.whitemoon319.pai.model.ChatMessage>, Boolean> {
        val input = mapOf(
            "conversationId" to conversationId,
            "beforeMessageId" to beforeMessageId,
            "limit" to limit,
        )
        @Suppress("UNCHECKED_CAST")
        val result = client.request("conversation.messagesBefore", mapOf("input" to input), Map::class.java) as Map<String, Any?>
        val messages = (result["messages"] as? List<Map<String, Any?>>).orEmpty().mapNotNull { raw ->
            runCatching {
                val gson = com.google.gson.Gson()
                gson.fromJson(gson.toJson(raw), com.whitemoon319.pai.model.ChatMessage::class.java)
            }.getOrNull()
        }
        val hasMore = (result["hasMore"] as? Boolean) ?: (messages.size >= limit)
        return messages to hasMore
    }

    suspend fun send(conversationId: String, departmentId: String?, agentId: String?, text: String): SubmitChatResult {
        val request = SendChatRequest(
            payload = com.whitemoon319.pai.model.ChatInputPayload(text = text),
            session = SessionSelector(departmentId = departmentId, agentId = agentId ?: "agent", conversationId = conversationId),
        )
        return client.request("chat.send", request, SubmitChatResult::class.java)
    }

    /** 发送消息并携带已摄取的附件（path 为 attachment.ingestLocalPath 返回的 savedPath）。 */
    suspend fun sendWithAttachments(
        conversationId: String,
        departmentId: String?,
        agentId: String?,
        text: String,
        attachments: List<com.whitemoon319.pai.model.AttachmentMeta>,
    ): SubmitChatResult {
        val request = SendChatRequest(
            payload = com.whitemoon319.pai.model.ChatInputPayload(
                text = text,
                attachments = attachments,
            ),
            session = SessionSelector(departmentId = departmentId, agentId = agentId ?: "agent", conversationId = conversationId),
        )
        return client.request("chat.send", request, SubmitChatResult::class.java)
    }

    /** 摄取本地附件（content URI 复制到沙盒后的绝对路径）→ 返回 AttachmentReceipt（含 savedPath）。 */
    suspend fun ingestAttachment(
        path: String,
        fileName: String?,
        mime: String?,
    ): com.whitemoon319.pai.model.AttachmentReceipt {
        val input = com.whitemoon319.pai.model.AttachmentIngestLocalPathInput(
            path = path,
            fileName = fileName,
            mime = mime,
        )
        return client.request(
            "attachment.ingestLocalPath",
            mapOf("input" to input),
            com.whitemoon319.pai.model.AttachmentReceipt::class.java,
        )
    }

    suspend fun stop(conversationId: String, departmentId: String?, agentId: String?) {
        val req = com.whitemoon319.pai.model.StopChatRequest(
            session = SessionSelector(departmentId = departmentId, agentId = agentId ?: "agent", conversationId = conversationId),
        )
        client.sendOneWay("chat.stop", req)
    }

    // ---------------- 会话管理（对齐 Vue 侧边栏操作） ----------------

    suspend fun renameConversation(conversationId: String, title: String): Boolean {
        val input = mapOf("conversationId" to conversationId, "title" to title)
        return client.request("conversation.rename", mapOf("input" to input), Boolean::class.java)
    }

    suspend fun toggleConversationPin(conversationId: String, pinned: Boolean): Boolean {
        val input = mapOf("conversationId" to conversationId, "pinned" to pinned)
        return client.request("conversation.pin", mapOf("input" to input), Boolean::class.java)
    }

    suspend fun deleteConversation(conversationId: String): Boolean {
        val input = mapOf("conversationId" to conversationId)
        return client.request("conversation.delete", mapOf("input" to input), Boolean::class.java)
    }

    /** 回退到指定消息（rewind）：删除其后消息，可重新生成。 */
    suspend fun rewindConversation(conversationId: String, departmentId: String?, agentId: String?, messageId: String): Map<String, Any?> {
        val input = mapOf(
            "session" to mapOf(
                "departmentId" to departmentId,
                "agentId" to (agentId ?: "agent"),
                "conversationId" to conversationId,
            ),
            "messageId" to messageId,
            "undoApplyPatch" to false,
        )
        @Suppress("UNCHECKED_CAST")
        return client.request("conversation.rewind", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    /** 压缩会话（compact：汇总旧消息释放上下文）。 */
    suspend fun compactConversation(conversationId: String): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("conversation.compact", mapOf("conversationId" to conversationId), Map::class.java) as Map<String, Any?>
    }

    /** 导出会话分享（exportShare：返回 fileName + payloadJson）。 */
    suspend fun exportConversationShare(conversationId: String): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("conversation.exportShare", mapOf("input" to mapOf("conversationId" to conversationId)), Map::class.java) as Map<String, Any?>
    }

    /** 设置会话自动推送远程联系人（conversation.autoPush）。 */
    suspend fun setConversationAutoPush(conversationId: String, remoteContactId: String?): Map<String, Any?> {
        val input = mapOf("conversationId" to conversationId, "remoteContactId" to remoteContactId)
        @Suppress("UNCHECKED_CAST")
        return client.request("conversation.autoPush", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    // ---------------- 委托任务（delegate） ----------------

    /** 委托会话列表。 */
    suspend fun delegateConversationsList(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("delegate.conversations.list", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    /** 委托状态列表。 */
    suspend fun delegateStatuses(conversationId: String?): List<Map<String, Any?>> {
        val input = mapOf("conversationId" to conversationId)
        @Suppress("UNCHECKED_CAST")
        return client.request("delegate.statuses", mapOf("input" to input), List::class.java) as List<Map<String, Any?>>
    }

    /** 中止委托。 */
    suspend fun delegateAbort(conversationId: String, delegateId: String?): Boolean {
        val input = mapOf("conversationId" to conversationId, "delegateId" to delegateId)
        return client.request("delegate.abort", mapOf("input" to input), Boolean::class.java)
    }

    /** 删除委托会话。 */
    suspend fun delegateDelete(conversationId: String): Boolean {
        val input = mapOf("conversationId" to conversationId)
        return client.request("delegate.delete", mapOf("input" to input), Boolean::class.java)
    }

    /** 提交委托任务。 */
    suspend fun delegateSubmit(
        conversationId: String,
        targetDepartmentId: String,
        targetAgentId: String? = null,
        goal: String? = null,
        why: String? = null,
        todo: String? = null,
        background: String? = null,
        question: String? = null,
    ): Map<String, Any?> {
        val input = mutableMapOf<String, Any?>(
            "conversationId" to conversationId,
            "targetDepartmentId" to targetDepartmentId,
            "targetAgentId" to targetAgentId,
            "goal" to goal,
            "why" to why,
            "todo" to todo,
            "background" to background,
            "question" to question,
        )
        @Suppress("UNCHECKED_CAST")
        return client.request("delegate.submit", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    // ---------------- 长期目标（goal） ----------------

    /** 查询会话当前目标。 */
    suspend fun goalCurrent(conversationId: String): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("goal.current", mapOf("conversationId" to conversationId), Map::class.java) as Map<String, Any?>
    }

    /** 创建/更新会话目标。 */
    suspend fun goalCreate(conversationId: String, objective: String): Map<String, Any?> {
        val input = mapOf("conversationId" to conversationId, "objective" to objective)
        @Suppress("UNCHECKED_CAST")
        return client.request("goal.create", input, Map::class.java) as Map<String, Any?>
    }

    /** 取消会话目标。 */
    suspend fun goalCancel(conversationId: String): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("goal.cancel", mapOf("conversationId" to conversationId), Map::class.java) as Map<String, Any?>
    }

    /** 当前会话提示词预览（get_prompt_preview）。 */
    suspend fun promptPreview(
        conversationId: String,
        departmentId: String?,
        agentId: String?,
        previewMode: String? = null,
    ): Map<String, Any?> {
        val input = mapOf(
            "departmentId" to departmentId,
            "agentId" to (agentId ?: "agent"),
            "conversationId" to conversationId,
        )
        val params = mutableMapOf<String, Any?>("input" to input)
        if (previewMode != null) params["previewMode"] = previewMode
        @Suppress("UNCHECKED_CAST")
        return client.request("get_prompt_preview", params, Map::class.java) as Map<String, Any?>
    }

    /** 读取聊天图片为 data URL（read_chat_image_data_url）。 */
    suspend fun readChatImageDataUrl(mediaRef: String, mime: String): String? {
        val input = mapOf("mediaRef" to mediaRef, "mime" to mime)
        @Suppress("UNCHECKED_CAST")
        val result = client.request("read_chat_image_data_url", mapOf("input" to input), Map::class.java) as Map<String, Any?>
        return result["dataUrl"] as? String
    }

    /** 读取人设头像为 data URL（read_avatar_data_url）。 */
    suspend fun readAvatarDataUrl(path: String): String? {
        @Suppress("UNCHECKED_CAST")
        val result = client.request("read_avatar_data_url", mapOf("input" to mapOf("path" to path)), Map::class.java) as Map<String, Any?>
        return result["dataUrl"] as? String
    }

    /** 会话可用模型列表（model.list，Vue 模型切换语义）。返回 chatModelOptions 数组。 */
    suspend fun modelList(conversationId: String): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        val result = client.request("model.list", mapOf("conversationId" to conversationId), Map::class.java) as Map<String, Any?>
        return (result["chatModelOptions"] as? List<Map<String, Any?>>) ?: emptyList()
    }

    suspend fun batchArchiveConversations(conversationIds: List<String>, reflectionApiConfigId: String? = null): Boolean {
        val input = mutableMapOf<String, Any?>("conversationIds" to conversationIds)
        if (!reflectionApiConfigId.isNullOrBlank()) {
            input["reflectionApiConfigId"] = reflectionApiConfigId
        }
        return client.request("conversation.batchArchive", mapOf("input" to input), Boolean::class.java)
    }

    // ---------------- 归档会话管理（对齐 Vue ArchivesWindow） ----------------

    suspend fun listArchives(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("archives.list", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    suspend fun archiveBlockPage(archiveId: String, blockId: Int? = null): Map<String, Any?> {
        val input = mapOf("archiveId" to archiveId, "blockId" to blockId)
        @Suppress("UNCHECKED_CAST")
        return client.request("archives.blockPage", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    suspend fun archiveSummary(archiveId: String): String =
        client.request("archives.summary", mapOf("archiveId" to archiveId), String::class.java)

    suspend fun deleteArchive(archiveId: String): Boolean =
        client.request("archives.delete", mapOf("archiveId" to archiveId), Boolean::class.java)

    suspend fun unarchiveArchive(archiveId: String): Boolean =
        client.request("archives.unarchive", mapOf("archiveId" to archiveId), Boolean::class.java)

    // ---------------- 设置 ----------------

    suspend fun loadConfig(): com.whitemoon319.pai.model.AppConfig =
        client.request("load_config", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AppConfig::class.java)

    /** 保存全局配置（patch 语义：传入的字段覆盖，未传保留）。 */
    suspend fun saveConfig(config: com.whitemoon319.pai.model.AppConfig): com.whitemoon319.pai.model.AppConfig =
        client.request("save_config", mapOf("config" to config), com.whitemoon319.pai.model.AppConfig::class.java)

    /** 局部更新配置（patch_config：只更新传入字段，避免全量覆盖丢其他配置）。 */
    suspend fun patchConfig(input: Map<String, Any?>): com.whitemoon319.pai.model.AppConfig =
        client.request("patch_config", mapOf("input" to input), com.whitemoon319.pai.model.AppConfig::class.java)

    // ---------------- 消息存储迁移（启动门禁） ----------------

    /** 迁移预检：返回 {migrationRequired, totalConversations, canAutoMigrate, ...}。 */
    suspend fun checkMessageStoreMigration(): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("check_message_store_migration", emptyMap<String, Any?>(), Map::class.java) as Map<String, Any?>
    }

    /** 执行消息存储迁移；失败抛异常，进度事件走 native 事件队列（messageStore.migration.progress）。 */
    suspend fun runMessageStoreMigration(): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return requestLong("run_message_store_migration", emptyMap<String, Any?>(), Map::class.java) as Map<String, Any?>
    }

    /** 读取人设/代理列表（load_agents）。 */
    suspend fun loadAgents(): List<com.whitemoon319.pai.model.AgentProfile> =
        client.request(
            "load_agents",
            emptyMap<String, Any?>(),
            object : com.google.gson.reflect.TypeToken<List<com.whitemoon319.pai.model.AgentProfile>>() {}.type,
        )

    /** 保存人设/代理列表（save_agents）。 */
    suspend fun saveAgents(agents: List<com.whitemoon319.pai.model.AgentProfile>): Boolean {
        val input = com.whitemoon319.pai.model.SaveAgentsInput(agents = agents)
        return client.request("save_agents", mapOf("input" to input), Boolean::class.java)
    }

    /** 查询 Web 访问（远程连接）状态：running/enabled/port/urls/password/connections。 */
    suspend fun getWebAccessInfo(forceRefresh: Boolean = false): Map<String, Any?> {
        val input = mapOf("forceRefresh" to forceRefresh)
        @Suppress("UNCHECKED_CAST")
        return client.request("get_web_access_info", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    // ---------------- Vue 设置页对齐：记忆 / 存储 / 用量 / 日志 / MCP / 任务 ----------------

    suspend fun listMemories(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("list_memories", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    suspend fun deleteMemory(memoryId: String): Boolean {
        val input = mapOf("memoryId" to memoryId)
        return client.request("delete_memory", mapOf("input" to input), Boolean::class.java)
    }

    /** 记忆回忆搜索（search_memories_recall）。默认 rag 模式（对话召回同链路），后端仅接受 rag/tool。 */
    suspend fun searchMemoriesRecall(agentId: String, query: String, mode: String = "rag"): Map<String, Any?> {
        val input = mapOf("agentId" to agentId, "query" to query, "mode" to mode)
        @Suppress("UNCHECKED_CAST")
        return client.request("search_memories_recall", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    suspend fun getStorageUsageOverview(): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("get_storage_usage_overview", emptyMap<String, Any?>(), Map::class.java) as Map<String, Any?>
    }

    suspend fun refreshStorageUsageOverview(): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("refresh_storage_usage_overview", emptyMap<String, Any?>(), Map::class.java) as Map<String, Any?>
    }

    suspend fun getUsageOverview(): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("get_usage_overview", emptyMap<String, Any?>(), Map::class.java) as Map<String, Any?>
    }

    suspend fun listRecentRuntimeLogs(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("list_recent_runtime_logs", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    /** LLM 轮次日志（诊断）。 */
    suspend fun listRecentLlmRoundLogs(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("list_recent_llm_round_logs", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    /** 清空 LLM 轮次日志。 */
    suspend fun clearRecentLlmRoundLogs(): Boolean =
        client.request("clear_recent_llm_round_logs", emptyMap<String, Any?>(), Boolean::class.java)

    suspend fun mcpListServers(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("mcp_list_servers", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    /** 保存/更新 MCP 服务器。 */
    suspend fun mcpSaveServer(id: String, name: String, enabled: Boolean, definitionJson: String): Boolean {
        val input = mapOf(
            "id" to id,
            "name" to name,
            "enabled" to enabled,
            "definitionJson" to definitionJson,
        )
        return client.request("mcp_save_server", mapOf("input" to input), Boolean::class.java)
    }

    /** 删除 MCP 服务器。 */
    suspend fun mcpRemoveServer(serverId: String): Boolean {
        val input = mapOf("serverId" to serverId)
        return client.request("mcp_remove_server", mapOf("input" to input), Boolean::class.java)
    }

    suspend fun taskListTasks(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("task_list_tasks", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    /** 创建定时任务。 */
    suspend fun taskCreateTask(
        goal: String,
        why: String = "",
        todo: String = "",
        runAt: String? = null,
        cronExpression: String? = null,
        agentId: String? = null,
    ): Map<String, Any?> {
        val input = mapOf(
            "goal" to goal,
            "why" to why,
            "todo" to todo,
            "agentId" to agentId,
            "trigger" to mapOf(
                "runAt" to runAt,
                "cronExpression" to cronExpression,
            ),
        )
        @Suppress("UNCHECKED_CAST")
        return client.request("task_create_task", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    /** 删除任务。 */
    suspend fun taskDeleteTask(taskId: String): Boolean {
        val input = mapOf("taskId" to taskId)
        return client.request("task_delete_task", mapOf("input" to input), Boolean::class.java)
    }

    suspend fun remoteImListChannels(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("remote_im_list_channels", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    /** 获取远程 IM 通道状态。 */
    suspend fun remoteImChannelStatus(channelId: String): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("remote_im_get_channel_status", mapOf("channelId" to channelId), Map::class.java) as Map<String, Any?>
    }

    /** 重启远程 IM 通道。 */
    suspend fun remoteImRestartChannel(channelId: String): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("remote_im_restart_channel", mapOf("channelId" to channelId), Map::class.java) as Map<String, Any?>
    }

    /** 远程 IM 联系人列表。 */
    suspend fun remoteImListContacts(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("remote_im_list_contacts", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    suspend fun setDepartmentPrimaryApiConfig(departmentId: String, apiConfigId: String): com.whitemoon319.pai.model.AppConfig {
        val input = com.whitemoon319.pai.model.SetDepartmentPrimaryApiConfigInput(departmentId = departmentId, apiConfigId = apiConfigId)
        return client.request("set_department_primary_api_config", mapOf("input" to input), com.whitemoon319.pai.model.AppConfig::class.java)
    }

    suspend fun loadChatSettings(): com.whitemoon319.pai.model.ChatSettings =
        client.request("load_chat_settings", emptyMap<String, Any?>(), com.whitemoon319.pai.model.ChatSettings::class.java)

    suspend fun saveChatSettings(settings: com.whitemoon319.pai.model.ChatSettings): com.whitemoon319.pai.model.ChatSettings =
        client.request("save_chat_settings", mapOf("input" to settings), com.whitemoon319.pai.model.ChatSettings::class.java)

    suspend fun checkToolsStatus(agentId: String?): List<com.whitemoon319.pai.model.ToolLoadStatus> {
        val input = com.whitemoon319.pai.model.CheckToolsStatusInput(agentId = agentId)
        return client.request("check_tools_status", mapOf("input" to input), object : com.google.gson.reflect.TypeToken<List<com.whitemoon319.pai.model.ToolLoadStatus>>() {}.type)
    }

    suspend fun bootstrapSnapshot(): com.whitemoon319.pai.model.BootstrapSnapshot =
        client.request("app.bootstrapSnapshot", emptyMap<String, Any?>(), com.whitemoon319.pai.model.BootstrapSnapshot::class.java)

    // ---------------- 供应商 CRUD（Android 设置页专用，后端 api_config.* RPC） ----------------

    suspend fun createApiConfig(config: com.whitemoon319.pai.model.ApiConfig): com.whitemoon319.pai.model.AppConfig =
        client.request("api_config.create", mapOf("input" to config), com.whitemoon319.pai.model.AppConfig::class.java)

    suspend fun updateApiConfig(config: com.whitemoon319.pai.model.ApiConfig): com.whitemoon319.pai.model.AppConfig =
        client.request("api_config.update", mapOf("input" to config), com.whitemoon319.pai.model.AppConfig::class.java)

    suspend fun deleteApiConfig(id: String): com.whitemoon319.pai.model.AppConfig {
        val input = com.whitemoon319.pai.model.ApiConfigDeleteInput(id = id)
        return client.request("api_config.delete", mapOf("input" to input), com.whitemoon319.pai.model.AppConfig::class.java)
    }

    /** 文本连接测试：用 ApiConfig 字段发一条极短请求。返回模型回复文本。 */
    suspend fun testTextConnection(config: com.whitemoon319.pai.model.ApiConfig): String =
        client.request("test_text_connection", mapOf("input" to config), String::class.java)

    // ---------------- Android 沙盒工作区 ----------------

    suspend fun getAndroidWorkspaceStatus(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        client.request("get_android_workspace_status", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun initAndroidWorkspace(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        requestLong("init_android_workspace", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun repairAndroidWorkspaceRuntime(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        requestLong("repair_android_workspace_runtime", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun resetAndroidWorkspaceRuntime(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        requestLong("reset_android_workspace_runtime", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun resetAndroidWorkspaceState(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        requestLong("reset_android_workspace_state", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun importAndroidWorkspaceRootfsArchive(fileName: String, dataBase64: String): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        requestLong(
            "import_android_workspace_rootfs_archive",
            mapOf("fileName" to fileName, "dataBase64" to dataBase64),
            com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java,
        )

    // ---------------- Android 工作区文件管理 ----------------

    suspend fun listWorkspaceFiles(path: String?): com.whitemoon319.pai.model.WorkspaceFileListResult =
        client.request("android_workspace.list", mapOf("path" to path), com.whitemoon319.pai.model.WorkspaceFileListResult::class.java)

    suspend fun readWorkspaceText(path: String): com.whitemoon319.pai.model.WorkspaceTextResult =
        client.request("android_workspace.readText", mapOf("path" to path), com.whitemoon319.pai.model.WorkspaceTextResult::class.java)

    suspend fun writeWorkspaceText(path: String, text: String, overwrite: Boolean): com.whitemoon319.pai.model.WorkspaceWriteResult =
        client.request(
            "android_workspace.writeText",
            mapOf("path" to path, "text" to text, "overwrite" to overwrite),
            com.whitemoon319.pai.model.WorkspaceWriteResult::class.java,
        )

    suspend fun moveWorkspaceFile(source: String, target: String, overwrite: Boolean): com.whitemoon319.pai.model.WorkspaceMoveResult =
        client.request(
            "android_workspace.move",
            mapOf("source" to source, "target" to target, "overwrite" to overwrite),
            com.whitemoon319.pai.model.WorkspaceMoveResult::class.java,
        )

    suspend fun globWorkspaceFiles(pattern: String, path: String?): com.whitemoon319.pai.model.WorkspaceGlobResult =
        client.request("android_workspace.glob", mapOf("pattern" to pattern, "path" to path), com.whitemoon319.pai.model.WorkspaceGlobResult::class.java)

    suspend fun grepWorkspaceFiles(query: String, path: String?, regex: Boolean?, ignoreCase: Boolean?, includeGlob: String?): com.whitemoon319.pai.model.WorkspaceGrepResult =
        client.request(
            "android_workspace.grep",
            mapOf("query" to query, "path" to path, "regex" to regex, "ignoreCase" to ignoreCase, "includeGlob" to includeGlob),
            com.whitemoon319.pai.model.WorkspaceGrepResult::class.java,
        )

    suspend fun deleteWorkspaceFile(path: String): com.whitemoon319.pai.model.WorkspaceDeleteResult =
        client.request("android_workspace.delete", mapOf("path" to path), com.whitemoon319.pai.model.WorkspaceDeleteResult::class.java)

    /** 导入文件到工作区（base64）。 */
    suspend fun importWorkspaceFile(fileName: String, mime: String?, dataBase64: String, targetPath: String?): com.whitemoon319.pai.model.WorkspaceImportResult =
        client.request(
            "android_workspace.import",
            mapOf("fileName" to fileName, "mime" to mime, "dataBase64" to dataBase64, "targetPath" to targetPath),
            com.whitemoon319.pai.model.WorkspaceImportResult::class.java,
        )

    /** 导出工作区文件为 base64。 */
    suspend fun exportWorkspaceFile(path: String): com.whitemoon319.pai.model.WorkspaceExportResult =
        client.request("android_workspace.export", mapOf("path" to path), com.whitemoon319.pai.model.WorkspaceExportResult::class.java)

    // ---------------- 关于 / 更新 ----------------

    suspend fun getAppVersion(): String =
        client.request("get_app_version", emptyMap<String, Any?>(), String::class.java)

    suspend fun getProjectRepositoryUrl(): String =
        client.request("get_project_repository_url", emptyMap<String, Any?>(), String::class.java)

    suspend fun checkGithubUpdate(): Map<String, Any?> =
        client.request("check_github_update", emptyMap<String, Any?>(), object : com.google.gson.reflect.TypeToken<Map<String, Any?>>() {}.type)
}