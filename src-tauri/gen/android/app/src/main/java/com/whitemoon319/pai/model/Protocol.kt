package com.whitemoon319.pai.model

import com.google.gson.JsonElement
import com.google.gson.annotations.SerializedName

/**
 * ws://127.0.0.1:8429/chat JSON-RPC 协议数据模型。
 * 字段名对应 Rust serde `rename_all = "camelCase"` 输出，仅 MainSessionState 使用 snake_case。
 * 可缺省字段统一用可空类型，缺失即 null。
 */

// ---------------- 信封 ----------------

data class RpcRequest(
    val jsonrpc: String = "2.0",
    @field:com.google.gson.annotations.SerializedName("id") var id: Long? = null,
    val method: String,
    val params: Any? = null,
)

data class RpcResponse(
    val jsonrpc: String,
    val id: Long?,
    val result: JsonElement? = null,
    val error: RpcError? = null,
)

data class RpcError(
    val code: Int,
    val message: String,
)

/** 下行通知（无 id）：bridge.ready / chat.assistantDelta / chat.roundFinished 等。 */
data class RpcNotification(
    val jsonrpc: String,
    val method: String,
    val params: JsonElement? = null,
)

// ---------------- 连接握手 ----------------

data class BridgeReady(
    val path: String,
    val authRequired: Boolean,
    val authMode: String?,
    val attachmentTransfer: BridgeAttachmentTransfer?,
)

data class BridgeAttachmentTransfer(
    val version: Int?,
    val chunkSize: Long?,
    val maxBytes: Long?,
)

// ---------------- 会话列表 ----------------

data class ConversationListResult(
    val conversations: List<ConversationSummary> = emptyList(),
    val unarchivedConversations: List<ConversationSummary> = emptyList(),
    val viewerId: String? = null,
)

data class ConversationSummary(
    var conversationId: String,
    var title: String? = null,
    @SerializedName("summaryTitle") var summaryTitle: String? = null,
    @SerializedName("updatedAt") var updatedAt: String? = null,
    @SerializedName("lastMessageAt") var lastMessageAt: String? = null,
    @SerializedName("messageCount") var messageCount: Int = 0,
    @SerializedName("bodyMessageCount") var bodyMessageCount: Int = 0,
    @SerializedName("bodyTextLength") var bodyTextLength: Int = 0,
    @SerializedName("hasAssistantReply") var hasAssistantReply: Boolean = false,
    @SerializedName("unreadCount") var unreadCount: Int = 0,
    @SerializedName("agentId") var agentId: String? = null,
    @SerializedName("departmentId") var departmentId: String? = null,
    @SerializedName("departmentName") var departmentName: String? = null,
    @SerializedName("conversationKind") var conversationKind: String? = null,
    @SerializedName("isActive") var isActive: Boolean = false,
    @SerializedName("isPinned") var isPinned: Boolean = false,
    @SerializedName("runtimeState") var runtimeState: String? = null,
    @SerializedName("planModeEnabled") var planModeEnabled: Boolean = false,
    @SerializedName("workspaceLabel") var workspaceLabel: String? = null,
    @SerializedName("previewMessages") var previewMessages: List<ConversationPreviewMessage>? = null,
    var state: ConversationListItemState? = null,
)

data class ConversationListItemState(
    val activity: String? = null,
    @SerializedName("runtimeState") val runtimeState: String? = null,
    @SerializedName("unreadCount") val unreadCount: Int = 0,
    @SerializedName("openState") val openState: String? = null,
    @SerializedName("openViewerId") val openViewerId: String? = null,
    @SerializedName("currentViewerId") val currentViewerId: String? = null,
)

data class ConversationPreviewMessage(
    @SerializedName("messageId") val messageId: String? = null,
    val role: String? = null,
    @SerializedName("speakerAgentId") val speakerAgentId: String? = null,
    @SerializedName("createdAt") val createdAt: String? = null,
    @SerializedName("textPreview") val textPreview: String? = null,
    @SerializedName("hasImage") val hasImage: Boolean = false,
    @SerializedName("hasPdf") val hasPdf: Boolean = false,
    @SerializedName("hasAudio") val hasAudio: Boolean = false,
)

// ---------------- 新建会话 ----------------

data class CreateConversationInput(
    @SerializedName("agentId") val agentId: String? = null,
    @SerializedName("departmentId") val departmentId: String? = null,
    val title: String? = null,
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
)

data class CreateConversationResult(
    @SerializedName("conversationId") val conversationId: String? = null,
)

/** conversation.createOptions 返回的部门/人格可选项。 */
data class CreateConversationOptions(
    val departments: List<CreateConversationOptionItem> = emptyList(),
    @SerializedName("defaultDepartmentId") val defaultDepartmentId: String? = null,
    @SerializedName("defaultAgentId") val defaultAgentId: String? = null,
)

data class CreateConversationOptionItem(
    val id: String? = null,
    @SerializedName("departmentId") val departmentId: String? = null,
    @SerializedName("agentId") val agentId: String? = null,
    @SerializedName("departmentName") val departmentName: String? = null,
    @SerializedName("agentName") val agentName: String? = null,
    @SerializedName("label") val label: String? = null,
)

// ---------------- 激活会话 ----------------

data class SetActiveInput(
    @SerializedName("conversationId") val conversationId: String? = null,
    @SerializedName("agentId") val agentId: String? = null,
)

data class SetActiveResult(
    @SerializedName("conversationId") val conversationId: String? = null,
)

// ---------------- 消息分页（blockPage / messageById） ----------------

data class BlockPageInput(
    @SerializedName("conversationId") val conversationId: String,
    @SerializedName("blockId") val blockId: Int? = null,
)

data class BlockPageResult(
    val blocks: List<BlockMeta> = emptyList(),
    @SerializedName("selectedBlockId") val selectedBlockId: Long? = null,
    val messages: List<ChatMessage> = emptyList(),
    @SerializedName("hasPrevBlock") val hasPrevBlock: Boolean = false,
    @SerializedName("hasNextBlock") val hasNextBlock: Boolean = false,
)

data class BlockMeta(
    @SerializedName("blockId") val blockId: Int? = null,
    @SerializedName("messageCount") val messageCount: Int = 0,
    @SerializedName("firstMessageId") val firstMessageId: String? = null,
    @SerializedName("lastMessageId") val lastMessageId: String? = null,
    @SerializedName("firstCreatedAt") val firstCreatedAt: String? = null,
    @SerializedName("lastCreatedAt") val lastCreatedAt: String? = null,
    @SerializedName("isLatest") val isLatest: Boolean = false,
)

// ---------------- 消息 ----------------

data class ChatMessage(
    val id: String,
    val role: String,
    @SerializedName("createdAt") val createdAt: String? = null,
    @SerializedName("speakerAgentId") val speakerAgentId: String? = null,
    val parts: List<MessagePart> = emptyList(),
    @SerializedName("extraTextBlocks") val extraTextBlocks: List<String> = emptyList(),
    @SerializedName("providerMeta") val providerMeta: JsonElement? = null,
    /** 落盘 assistant 消息的工具历史事件数组（role=tool/assistant 的 tool_calls）。 */
    @SerializedName("toolCall") val toolCall: List<ToolHistoryEvent>? = null,
)

/** 判别联合 MessagePart，顶层带 type 字段：Text/Image/Audio/Attachment。 */
data class MessagePart(
    val type: String,
    val text: String? = null,
    @SerializedName(value = "reasoningContent", alternate = ["reasoning_content"]) val reasoningContent: String? = null,
    val mime: String? = null,
    @SerializedName("bytesBase64") val bytesBase64: String? = null,
    val name: String? = null,
    val path: String? = null,
    val compressed: Boolean = false,
) {
    val displayText: String
        get() = when (type) {
            "Text" -> text.orEmpty()
            else -> text?.takeIf { it.isNotBlank() } ?: "[$type]"
        }
}

/**
 * 落盘工具历史事件（后端 tool_call 数组项，含 assistant 的 tool_calls 与 tool 的结果）。
 * 与流式 assistant_tool_event/result 的 message JSON 同构，可复用同一解析。
 */
data class ToolHistoryEvent(
    val role: String? = null,
    val content: String? = null,
    @SerializedName("tool_call_id") val toolCallId: String? = null,
    @SerializedName("tool_calls") val toolCalls: List<ToolCallInfo>? = null,
    @SerializedName("reasoning_content") val reasoningContent: String? = null,
)

data class ToolCallInfo(
    val id: String? = null,
    @SerializedName("call_id") val callId: String? = null,
    val function: ToolFunctionInfo? = null,
    val type: String? = null,
)

data class ToolFunctionInfo(
    val name: String? = null,
    val arguments: String? = null,
)

/**
 * UI 层一条可折叠的活动步骤：思考（reasoning）或工具（tool）。
 * 思考与工具同属一个 "thinking" 大类，大类与单个步骤都支持折叠。
 */
sealed class ActivityStep {
    /** 一段思考过程文本。 */
    data class Reasoning(val text: String) : ActivityStep()

    /** 一次工具调用（可能带工具级思考与结果）。 */
    data class Tool(
        val toolCallId: String?,
        val name: String?,
        val argsText: String?,
        val resultText: String?,
        val status: String?,
        /** 工具级思考，展开时为思考块，折叠时并入 tool 头部。 */
        val reasoning: String?,
    ) : ActivityStep()
}

/** 从一条 assistant 消息的 parts + toolCall 构建有序活动步骤（思考/工具交错，保持到达顺序）。 */
fun buildActivityStepsFromMessage(message: ChatMessage): List<ActivityStep> = buildList {
    // parts 里的 reasoning_content 是整轮思考的累计文本，作为独立思考块
    val reasoning = message.parts
        .mapNotNull { it.reasoningContent?.takeIf { r -> r.isNotBlank() } }
        .joinToString("\n")
    if (reasoning.isNotBlank()) {
        add(ActivityStep.Reasoning(reasoning))
    }
    // toolCall 数组里每条带有 tool_calls 的 assistant 事件拆成多个工具步骤
    message.toolCall.orEmpty().forEach { event ->
        if (event.role.orEmpty().trim().equals("assistant", ignoreCase = true)) {
            val eventReasoning = event.reasoningContent?.takeIf { it.isNotBlank() }
            val calls = event.toolCalls.orEmpty()
            calls.forEachIndexed { index, call ->
                val name = call.function?.name?.takeIf { it.isNotBlank() }
                val args = call.function?.arguments?.takeIf { it.isNotBlank() }
                // 工具级思考挂到该事件的第一个工具调用
                val toolReasoning = if (index == 0) eventReasoning else null
                add(
                    ActivityStep.Tool(
                        toolCallId = call.id ?: call.callId,
                        name = name,
                        argsText = args,
                        resultText = null,
                        status = "done",
                        reasoning = toolReasoning,
                    )
                )
            }
        } else if (event.role.orEmpty().trim().equals("tool", ignoreCase = true)) {
            // 工具结果追加到最近一个未闭合的工具步骤
            val result = event.content?.takeIf { it.isNotBlank() }
            if (result != null) {
                val steps = this
                val lastTool = steps.indexOfLast { it is ActivityStep.Tool }
                if (lastTool >= 0) {
                    val prev = steps[lastTool] as ActivityStep.Tool
                    steps[lastTool] = prev.copy(resultText = result)
                }
            }
        }
    }
}

/**
 * 用例：文本落盘时把当前流式 activitySteps（思考+工具）并入消息，保证思考在正文本体上方。
 * 把活动步骤恢复成 message 的 parts + toolCall 结构，供 MessageBubble 复用 buildActivityStepsFromMessage。
 * @param assistantText 正文本体（放入 Text part）
 * @param steps 活动步骤（思考拼进 reasoningContent，工具还原为 tool history event）
 */
fun buildChatMessageFromActivitySteps(
    id: String,
    role: String,
    assistantText: String,
    steps: List<ActivityStep>,
): ChatMessage {
    // 思考全文：所有 Reasoning 步骤 + 工具级思考，拼成 reasoningContent
    val reasoning = buildList {
        steps.forEach { step ->
            when (step) {
                is ActivityStep.Reasoning -> add(step.text)
                is ActivityStep.Tool -> step.reasoning?.takeIf { it.isNotBlank() }?.let { add(it) }
            }
        }
    }.joinToString("\n")

    // 工具历史事件还原：每条 assistant 的事件聚合 tool_calls
    val toolCall = buildList {
        val toolSteps = steps.filterIsInstance<ActivityStep.Tool>()
        if (toolSteps.isNotEmpty()) {
            add(
                ToolHistoryEvent(
                    role = "assistant",
                    toolCalls = toolSteps.map { step ->
                        ToolCallInfo(
                            id = step.toolCallId,
                            function = ToolFunctionInfo(name = step.name, arguments = step.argsText),
                        )
                    },
                    reasoningContent = null,
                )
            )
            // 工具结果：有 resultText 的工具步骤追加 tool role 事件
            toolSteps.filter { !it.resultText.isNullOrBlank() }.forEach { step ->
                add(
                    ToolHistoryEvent(
                        role = "tool",
                        toolCallId = step.toolCallId,
                        content = step.resultText,
                    )
                )
            }
        }
    }

    return ChatMessage(
        id = id,
        role = role,
        parts = listOf(MessagePart(type = "Text", text = assistantText, reasoningContent = reasoning)),
        toolCall = toolCall,
    )
}

// ---------------- 发送/停止 ----------------

data class SessionSelector(
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
    @SerializedName("departmentId") val departmentId: String? = null,
    @SerializedName("agentId") val agentId: String,
    @SerializedName("conversationId") val conversationId: String? = null,
)

data class ChatInputPayload(
    val text: String? = null,
    @SerializedName("displayText") val displayText: String? = null,
    val parts: List<ChatIngressPart>? = null,
    val images: List<BinaryPart>? = null,
    val audios: List<BinaryPart>? = null,
    val attachments: List<AttachmentMeta>? = null,
    val model: String? = null,
    @SerializedName("extraTextBlocks") val extraTextBlocks: List<String>? = null,
    val mentions: List<Mention>? = null,
    @SerializedName("providerMeta") val providerMeta: JsonElement? = null,
)

data class ChatIngressPart(
    val type: String,
    val text: String? = null,
    val path: String? = null,
    @SerializedName("bytesBase64") val bytesBase64: String? = null,
    val mime: String? = null,
    val name: String? = null,
)

data class BinaryPart(
    val mime: String,
    @SerializedName("bytesBase64") val bytesBase64: String,
    @SerializedName("savedPath") val savedPath: String? = null,
)

data class AttachmentMeta(
    @SerializedName("fileName") val fileName: String,
    val path: String,
    val mime: String? = null,
)

/** 附件本地路径摄取输入（attachment.ingestLocalPath）。 */
data class AttachmentIngestLocalPathInput(
    val path: String,
    @SerializedName("fileName") val fileName: String? = null,
    val mime: String? = null,
)

/** 附件摄取结果（AttachmentReceipt）。 */
data class AttachmentReceipt(
    val id: String,
    @SerializedName("fileName") val fileName: String,
    val mime: String,
    val size: Long,
    val path: String,
    @SerializedName("attachAsMedia") val attachAsMedia: Boolean = false,
    @SerializedName("textNotice") val textNotice: String? = null,
    @SerializedName("previewDataUrl") val previewDataUrl: String? = null,
)

data class Mention(
    @SerializedName("agentId") val agentId: String? = null,
    @SerializedName("agentName") val agentName: String? = null,
    @SerializedName("departmentId") val departmentId: String? = null,
    @SerializedName("departmentName") val departmentName: String? = null,
)

data class SendChatRequest(
    val payload: ChatInputPayload? = null,
    val session: SessionSelector? = null,
    @SerializedName("speakerAgentId") val speakerAgentId: String? = null,
    @SerializedName("traceId") val traceId: String? = null,
    @SerializedName("assistantMessageId") val assistantMessageId: String? = null,
    @SerializedName("oldestQueueCreatedAt") val oldestQueueCreatedAt: String? = null,
    @SerializedName("remoteImActivationSources") val remoteImActivationSources: List<JsonElement>? = null,
    @SerializedName("runtimeContext") val runtimeContext: JsonElement? = null,
    @SerializedName("triggerOnly") val triggerOnly: Boolean = false,
)

/** chat.stop 请求。 */
data class StopChatRequest(
    val session: SessionSelector? = null,
    @SerializedName("partialAssistantText") val partialAssistantText: String? = null,
    @SerializedName("partialStreamBlocks") val partialStreamBlocks: List<JsonElement>? = null,
)

/** chat.send 快速回执（正文走下行 delta）。 */
data class SubmitChatResult(
    val accepted: Boolean = false,
    val duplicate: Boolean = false,
    @SerializedName("eventId") val eventId: String? = null,
    @SerializedName("conversationId") val conversationId: String? = null,
    @SerializedName("traceId") val traceId: String? = null,
    val ingress: String? = null,
    @SerializedName("userMessageId") val userMessageId: String? = null,
    @SerializedName("assistantMessageId") val assistantMessageId: String? = null,
)

// ---------------- 下行推送 ----------------

/** chat.assistantDelta 的 params。 */
data class DeltaNotification(
    @SerializedName("conversationId") val conversationId: String? = null,
    val event: DeltaEvent? = null,
    @SerializedName("conversationTitle") val conversationTitle: String? = null,
)

data class DeltaEvent(
    val delta: String? = null,
    val kind: String? = null,
    @SerializedName("requestId") val requestId: String? = null,
    @SerializedName("toolName") val toolName: String? = null,
    @SerializedName("toolStatus") val toolStatus: String? = null,
    /** 后端把 DeltaMessage 整体序列化成 JSON 字符串下发，需二次解析。 */
    val message: String? = null,
)

data class DeltaMessage(
    @SerializedName("conversationId") val conversationId: String? = null,
    @SerializedName("activationId") val activationId: String? = null,
    @SerializedName("requestId") val requestId: String? = null,
    @SerializedName("assistantText") val assistantText: String? = null,
    @SerializedName("archivedBeforeSend") val archivedBeforeSend: Boolean = false,
    @SerializedName("assistantMessage") val assistantMessage: ChatMessage? = null,
)

// ---------------- 设置（模型与供应商 / 聊天设置 / 工具 / 关于） ----------------

/** load_config 返回的 AppConfig（只解析 Android 设置页所需子集）。 */
data class AppConfig(
    @SerializedName("apiConfigs") val apiConfigs: List<ApiConfig> = emptyList(),
    @SerializedName("assistantDepartmentApiConfigId") val assistantDepartmentApiConfigId: String? = null,
    @SerializedName("selectedApiConfigId") val selectedApiConfigId: String? = null,
    @SerializedName("webAccessEnabled") val webAccessEnabled: Boolean? = null,
    @SerializedName("webAccessPort") val webAccessPort: Int? = null,
    @SerializedName("webAccessPassword") val webAccessPassword: String? = null,
    @SerializedName("sttApiConfigId") val sttApiConfigId: String? = null,
    @SerializedName("sttAutoSend") val sttAutoSend: Boolean? = null,
    @SerializedName("messageNotificationEnabled") val messageNotificationEnabled: Boolean? = null,
    @SerializedName("messageNotificationSoundEnabled") val messageNotificationSoundEnabled: Boolean? = null,
    @SerializedName("desktopOperationNoticeEnabled") val desktopOperationNoticeEnabled: Boolean? = null,
    @SerializedName("uiLanguage") val uiLanguage: String? = null,
    @SerializedName("uiSizeScale") val uiSizeScale: Int? = null,
)

/** 会话级 API 设置（含 STT/视觉/工具审查供应商），save_conversation_api_settings 用。 */
data class ConversationApiSettings(
    @SerializedName("assistantDepartmentApiConfigId", alternate = ["chatApiConfigId"])
    val assistantDepartmentApiConfigId: String? = null,
    @SerializedName("visionApiConfigId") val visionApiConfigId: String? = null,
    @SerializedName("toolReviewApiConfigId") val toolReviewApiConfigId: String? = null,
    @SerializedName("sttApiConfigId") val sttApiConfigId: String? = null,
    @SerializedName("sttAutoSend") val sttAutoSend: Boolean = false,
)

/** 单个 API 配置（供应商）。 */
data class ApiConfig(
    val id: String? = null,
    val name: String? = null,
    @SerializedName("requestFormat") val requestFormat: String? = null,
    @SerializedName("allowConcurrentRequests") val allowConcurrentRequests: Boolean = true,
    @SerializedName("maxConcurrentRequests") val maxConcurrentRequests: Int? = null,
    @SerializedName("enableText") val enableText: Boolean = true,
    @SerializedName("enableImage") val enableImage: Boolean = false,
    @SerializedName("enableAudio") val enableAudio: Boolean = false,
    @SerializedName("enableVideo") val enableVideo: Boolean = false,
    @SerializedName("enableTools") val enableTools: Boolean = true,
    @SerializedName("baseUrl") val baseUrl: String? = null,
    @SerializedName("apiKey") val apiKey: String? = null,
    val model: String? = null,
    @SerializedName("reasoningEffort") val reasoningEffort: String? = null,
    val temperature: Double = 0.7,
    @SerializedName("customTemperatureEnabled") val customTemperatureEnabled: Boolean = false,
    @SerializedName("contextWindowTokens") val contextWindowTokens: Int = 128000,
    @SerializedName("maxOutputTokens") val maxOutputTokens: Int = 8192,
    @SerializedName("customMaxOutputTokensEnabled") val customMaxOutputTokensEnabled: Boolean = false,
    @SerializedName("failureRetryCount") val failureRetryCount: Int = 0,
)

/** load_chat_settings / save_chat_settings 的 ChatSettings。 */
data class ChatSettings(
    @SerializedName("assistantDepartmentAgentId") val assistantDepartmentAgentId: String? = null,
    @SerializedName("userAlias") val userAlias: String? = null,
    @SerializedName("responseStyleId") val responseStyleId: String? = null,
    @SerializedName("pdfReadMode") val pdfReadMode: String? = null,
    @SerializedName("instructionPresets") val instructionPresets: List<PromptCommandPreset> = emptyList(),
)

/** 人设/代理（AgentProfile），load_agents / save_agents 用。 */
data class AgentProfile(
    val id: String? = null,
    val name: String? = null,
    @SerializedName("systemPrompt") val systemPrompt: String? = null,
    @SerializedName("createdAt") val createdAt: String? = null,
    @SerializedName("updatedAt") val updatedAt: String? = null,
    @SerializedName("avatarPath") val avatarPath: String? = null,
    @SerializedName("isBuiltInUser") val isBuiltInUser: Boolean = false,
    @SerializedName("isBuiltInSystem") val isBuiltInSystem: Boolean = false,
    @SerializedName("privateMemoryEnabled") val privateMemoryEnabled: Boolean = false,
    @SerializedName("memoryRecallMode") val memoryRecallMode: String? = null,
    val source: String? = null,
    val scope: String? = null,
)

/** save_agents 输入。 */
data class SaveAgentsInput(
    val agents: List<AgentProfile> = emptyList(),
)

data class PromptCommandPreset(
    val id: String? = null,
    val name: String? = null,
    val prompt: String? = null,
)

/** check_tools_status 的请求 input。 */
data class CheckToolsStatusInput(
    @SerializedName("agentId") val agentId: String? = null,
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
)

/** check_tools_status 返回的单条工具状态。 */
data class ToolLoadStatus(
    val id: String? = null,
    val status: String? = null,
    val detail: String? = null,
)

/** set_department_primary_api_config 的请求 input。 */
data class SetDepartmentPrimaryApiConfigInput(
    @SerializedName("departmentId") val departmentId: String? = null,
    @SerializedName("apiConfigId") val apiConfigId: String? = null,
)

/** app.bootstrapSnapshot 返回（只取设置页需要的部分）。 */
data class BootstrapSnapshot(
    val config: AppConfig? = null,
    val chatSettings: ChatSettings? = null,
)

/** Android 沙盒工作区状态。 */
data class AndroidWorkspaceStatus(
    val state: String? = null,
    @SerializedName("rootPath") val rootPath: String? = null,
    @SerializedName("llmWorkspaceRoot") val llmWorkspaceRoot: String? = null,
    @SerializedName("runtimeRoot") val runtimeRoot: String? = null,
    @SerializedName("initializedAt") val initializedAt: String? = null,
    @SerializedName("updatedAt") val updatedAt: String? = null,
    @SerializedName("lastError") val lastError: String? = null,
    val version: Int = 0,
    @SerializedName("runtimeVersion") val runtimeVersion: String? = null,
    @SerializedName("downloadBytes") val downloadBytes: Long? = null,
    @SerializedName("downloadTotalBytes") val downloadTotalBytes: Long? = null,
    @SerializedName("downloadStage") val downloadStage: String? = null,
) {
    // 后端枚举 AndroidWorkspaceStateKind 序列化为 snake_case（ready/downloading/not_downloaded）
    val isReady: Boolean get() = state == "ready" || state == "Ready"
    val isDownloading: Boolean get() = state == "downloading" || state == "Downloading"
    val isNotDownloaded: Boolean get() = state == "not_downloaded" || state == "NotDownloaded"
}

/** api_config.delete 的请求 input。 */
data class ApiConfigDeleteInput(
    val id: String? = null,
)

// ---------------- Android 工作区文件管理 ----------------

data class WorkspaceFileEntry(
    val name: String? = null,
    val path: String? = null,
    val kind: String? = null,
    val bytes: Long? = null,
) {
    val isDirectory: Boolean get() = kind == "directory"
}

data class WorkspaceFileListResult(
    @SerializedName("currentPath") val currentPath: String? = null,
    @SerializedName("parentPath") val parentPath: String? = null,
    val entries: List<WorkspaceFileEntry> = emptyList(),
)

data class WorkspaceTextResult(
    val path: String? = null,
    val text: String? = null,
    val bytes: Long = 0,
)

data class WorkspaceWriteResult(
    val entry: WorkspaceFileEntry? = null,
)

data class WorkspaceMoveResult(
    @SerializedName("sourcePath") val sourcePath: String? = null,
    val entry: WorkspaceFileEntry? = null,
)

data class WorkspaceGlobResult(
    val entries: List<WorkspaceFileEntry> = emptyList(),
)

data class WorkspaceSearchMatch(
    val path: String? = null,
    val line: Long = 0,
    val text: String? = null,
)

data class WorkspaceGrepResult(
    val matches: List<WorkspaceSearchMatch> = emptyList(),
)

data class WorkspaceDeleteResult(
    @SerializedName("deletedPath") val deletedPath: String? = null,
)

data class WorkspaceImportResult(
    val status: AndroidWorkspaceStatus? = null,
    @SerializedName("importedPath") val importedPath: String? = null,
    @SerializedName("fileName") val fileName: String? = null,
    val bytes: Long = 0,
)

data class WorkspaceExportResult(
    val path: String? = null,
    @SerializedName("fileName") val fileName: String? = null,
    val mime: String? = null,
    @SerializedName("dataBase64") val dataBase64: String? = null,
    val bytes: Long = 0,
)