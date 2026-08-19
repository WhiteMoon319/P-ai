package com.whitemoon319.pai.service

import com.whitemoon319.pai.model.TaskHandle
import com.whitemoon319.pai.model.RpcTaskState
import com.whitemoon319.pai.bridge.NativeRpcClient

/**
 * 原生任务状态机门面：封装 task.* JSON-RPC 方法。
 *
 * 长任务（workspace 初始化、rootfs 导入、migration 等）通过此接口
 * 创建/更新/查询/取消任务，任务进度事件通过 NativeEventPump 的
 * "task.progress" 通知下发。
 */
class TaskService(private val client: NativeRpcClient) {

    /**
     * 创建一个新任务。返回任务句柄（含 taskId/状态/时间戳）。
     * 参数：taskId 唯一标识一次长任务操作的键。
     */
    suspend fun createTask(taskId: String): TaskHandle =
        client.request("task.create", mapOf("taskId" to taskId), TaskHandle::class.java)

    /**
     * 更新任务状态与进度。每次更新会推送 TaskProgressEvent 到事件队列。
     *
     * @param taskId 任务 ID
     * @param state 新状态（Pending / Running / Completed / Failed / Cancelled）
     * @param progress 进度 0.0~1.0
     * @param message 可读描述
     */
    suspend fun updateTask(
        taskId: String,
        state: RpcTaskState,
        progress: Double,
        message: String,
    ): TaskHandle = client.request(
        "task.update",
        mapOf("taskId" to taskId, "state" to state.name, "progress" to progress, "message" to message),
        TaskHandle::class.java,
    )

    /**
     * 查询任务当前状态。
     */
    suspend fun getTask(taskId: String): TaskHandle =
        client.request("task.get", mapOf("taskId" to taskId), TaskHandle::class.java)

    /**
     * 取消一个任务。
     */
    suspend fun cancelTask(taskId: String): TaskHandle =
        client.request("task.cancel", mapOf("taskId" to taskId), TaskHandle::class.java)
}