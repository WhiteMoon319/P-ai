package com.whitemoon319.pai.model

import com.google.gson.annotations.SerializedName

/**
 * 原生任务状态（对齐 Rust pai_android_bridge::task::TaskState）。
 */
enum class RpcTaskState {
    @SerializedName("Pending") Pending,
    @SerializedName("Running") Running,
    @SerializedName("Completed") Completed,
    @SerializedName("Failed") Failed,
    @SerializedName("Cancelled") Cancelled,
}

/**
 * 任务句柄（对齐 Rust pai_android_bridge::task::TaskHandle）。
 */
data class TaskHandle(
    @SerializedName("task_id") val taskId: String,
    val state: RpcTaskState,
    val progress: Double,
    val message: String,
    @SerializedName("created_at") val createdAt: String,
    @SerializedName("updated_at") val updatedAt: String,
)

/**
 * 任务进度事件（对齐 Rust pai_android_bridge::task::TaskProgressEvent）。
 * 通过 native 事件队列推送，method = "task.progress"。
 */
data class TaskProgressEvent(
    @SerializedName("task_id") val taskId: String,
    val state: RpcTaskState,
    val progress: Double,
    val message: String,
)