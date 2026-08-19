//! 原生长任务状态机：workspace/rootfs/migration 等长时间操作的进度追踪。
//!
//! 设计目标：
//! - `workspace.task.start → taskId → progress events → query status → complete/fail/cancel`
//! - 任务在后台异步执行，进度通过事件队列推送给 Kotlin
//! - 调用方通过 taskId 查询状态或取消任务
//!
//! 本模块定义类型与 trait，具体实现在 src-tauri 侧。

use serde::{Deserialize, Serialize};

/// 任务状态。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TaskState {
    /// 等待执行
    Pending,
    /// 执行中
    Running,
    /// 已完成
    Completed,
    /// 已失败（含错误信息）
    Failed(String),
    /// 已取消
    Cancelled,
}

/// 任务句柄，唯一标识一个后台长任务。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskHandle {
    /// 任务唯一 ID
    pub task_id: String,
    /// 当前状态
    pub state: TaskState,
    /// 进度 0.0 ~ 1.0
    pub progress: f64,
    /// 可读描述
    pub message: String,
    /// 创建时间（ISO 8601）
    pub created_at: String,
    /// 最后更新时间（ISO 8601）
    pub updated_at: String,
}

/// 任务进度事件，通过事件队列推送给 Kotlin。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskProgressEvent {
    pub task_id: String,
    pub state: TaskState,
    pub progress: f64,
    pub message: String,
}

/// 任务管理器 trait，由 src-tauri 侧实现。
pub trait TaskManager: Send + Sync {
    /// 创建任务并返回句柄。
    fn create_task(&self, task_id: &str) -> Result<TaskHandle, String>;

    /// 更新任务状态与进度。
    fn update_task(
        &self,
        task_id: &str,
        state: TaskState,
        progress: f64,
        message: &str,
    ) -> Result<TaskHandle, String>;

    /// 查询任务状态。
    fn get_task(&self, task_id: &str) -> Result<TaskHandle, String>;

    /// 取消任务。
    fn cancel_task(&self, task_id: &str) -> Result<TaskHandle, String>;
}