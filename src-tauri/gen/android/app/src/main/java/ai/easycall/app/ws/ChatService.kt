package ai.easycall.app.ws

import ai.easycall.app.model.BlockPageInput
import ai.easycall.app.model.BlockPageResult
import ai.easycall.app.model.ConversationListResult
import ai.easycall.app.model.CreateConversationInput
import ai.easycall.app.model.CreateConversationResult
import ai.easycall.app.model.SendChatRequest
import ai.easycall.app.model.SessionSelector
import ai.easycall.app.model.SetActiveInput
import ai.easycall.app.model.SetActiveResult
import ai.easycall.app.model.SubmitChatResult

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

    suspend fun setActive(conversationId: String, agentId: String?): SetActiveResult {
        val input = SetActiveInput(conversationId = conversationId, agentId = agentId)
        return client.request("conversation.setActive", mapOf("input" to input), SetActiveResult::class.java)
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
            payload = ai.easycall.app.model.ChatInputPayload(text = text),
            session = SessionSelector(departmentId = departmentId, agentId = agentId ?: "agent", conversationId = conversationId),
        )
        return client.request("chat.send", request, SubmitChatResult::class.java)
    }

    suspend fun stop(conversationId: String, departmentId: String?, agentId: String?) {
        val req = ai.easycall.app.model.StopChatRequest(
            session = SessionSelector(departmentId = departmentId, agentId = agentId ?: "agent", conversationId = conversationId),
        )
        client.sendOneWay("chat.stop", req)
    }
}