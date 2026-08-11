package com.whitemoon319.pai.model

import com.google.gson.JsonElement
import com.google.gson.annotations.SerializedName

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
