package com.whitemoon319.pai.ws

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

/**
 * 聊天相关协议方法门面，直接封装 JSON-RPC 调用。
 * 方法名与 jsonrpc_dispatch.rs 的 ws method 一致。
 */
class ChatService(private val client: PaiWsClient) {

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

    suspend fun batchArchiveConversations(conversationIds: List<String>): Boolean {
        val input = mapOf("conversationIds" to conversationIds)
        return client.request("conversation.batchArchive", mapOf("input" to input), Boolean::class.java)
    }

    // ---------------- 设置 ----------------

    suspend fun loadConfig(): com.whitemoon319.pai.model.AppConfig =
        client.request("load_config", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AppConfig::class.java)

    /** 保存全局配置（patch 语义：传入的字段覆盖，未传保留）。 */
    suspend fun saveConfig(config: com.whitemoon319.pai.model.AppConfig): com.whitemoon319.pai.model.AppConfig =
        client.request("save_config", mapOf("config" to config), com.whitemoon319.pai.model.AppConfig::class.java)

    /** 查询 Web 访问（远程连接）状态：running/enabled/port/urls/password/connections。 */
    suspend fun getWebAccessInfo(forceRefresh: Boolean = false): Map<String, Any?> {
        val input = mapOf("forceRefresh" to forceRefresh)
        @Suppress("UNCHECKED_CAST")
        return client.request("get_web_access_info", mapOf("input" to input), Map::class.java) as Map<String, Any?>
    }

    // ---------------- Vue 设置页对齐：记忆 / 存储 / 用量 / 日志 / MCP / 任务 ----------------

    suspend fun listMemories(): Map<String, Any?> {
        @Suppress("UNCHECKED_CAST")
        return client.request("list_memories", emptyMap<String, Any?>(), Map::class.java) as Map<String, Any?>
    }

    suspend fun deleteMemory(memoryId: String): Boolean {
        val input = mapOf("memoryId" to memoryId)
        return client.request("delete_memory", mapOf("input" to input), Boolean::class.java)
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

    suspend fun mcpListServers(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("mcp_list_servers", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    suspend fun taskListTasks(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("task_list_tasks", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
    }

    suspend fun remoteImListChannels(): List<Map<String, Any?>> {
        @Suppress("UNCHECKED_CAST")
        return client.request("remote_im_list_channels", emptyMap<String, Any?>(), List::class.java) as List<Map<String, Any?>>
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
        client.request("init_android_workspace", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun repairAndroidWorkspaceRuntime(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        client.request("repair_android_workspace_runtime", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun resetAndroidWorkspaceRuntime(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        client.request("reset_android_workspace_runtime", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun resetAndroidWorkspaceState(): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        client.request("reset_android_workspace_state", emptyMap<String, Any?>(), com.whitemoon319.pai.model.AndroidWorkspaceStatus::class.java)

    suspend fun importAndroidWorkspaceRootfsArchive(fileName: String, dataBase64: String): com.whitemoon319.pai.model.AndroidWorkspaceStatus =
        client.request(
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