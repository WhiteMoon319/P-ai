package ai.easycall.app.model

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
)

/** 判别联合 MessagePart，顶层带 type 字段：Text/Image/Audio/Attachment。 */
data class MessagePart(
    val type: String,
    val text: String? = null,
    @SerializedName("reasoningContent") val reasoningContent: String? = null,
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
    val message: DeltaMessage? = null,
)

data class DeltaMessage(
    @SerializedName("conversationId") val conversationId: String? = null,
    @SerializedName("activationId") val activationId: String? = null,
    @SerializedName("requestId") val requestId: String? = null,
    @SerializedName("assistantText") val assistantText: String? = null,
    @SerializedName("archivedBeforeSend") val archivedBeforeSend: Boolean = false,
    @SerializedName("assistantMessage") val assistantMessage: ChatMessage? = null,
)