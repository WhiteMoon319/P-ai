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

impl TaskState {
    /// 从 JSON-RPC 字符串解析任务状态。
    ///
    /// 接受 `Pending` / `Running` / `Completed` / `Failed` / `Cancelled`
    /// （大小写不敏感；`Failed` 可附带 `,reason` 后缀承载错误信息）。
    pub fn from_str(value: &str) -> Result<TaskState, String> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err("task state is required.".to_string());
        }
        let (head, reason) = match trimmed.split_once(',') {
            Some((head, reason)) => (head.trim(), Some(reason.trim())),
            None => (trimmed, None),
        };
        match head.to_ascii_lowercase().as_str() {
            "pending" => Ok(TaskState::Pending),
            "running" => Ok(TaskState::Running),
            "completed" | "done" => Ok(TaskState::Completed),
            "failed" | "error" => Ok(TaskState::Failed(
                reason.map(str::to_string).unwrap_or_default(),
            )),
            "cancelled" | "canceled" => Ok(TaskState::Cancelled),
            other => Err(format!("unknown task state: {other}")),
        }
    }
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

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use pai_android_platform::event_queue::push_native_delta_event;

fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // ISO 8601（本地时间近似 UTC，Android 端仅作展示排序用）。
    let days = secs.div_euclid(86400);
    let secs_of_day = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        y,
        m,
        d,
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60,
        secs_of_day % 60
    )
}

/// 将 Unix epoch days 转换为 (年, 月, 日) 公历（Howard Hinnant 算法）。
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// 默认任务管理器：全局任务注册表 + 进度事件推送。
///
/// 实现 `TaskManager` trait，任务注册表为进程级单例（静态锁表），
/// 每次 `update_task` 会把 `TaskProgressEvent` 推入原生事件队列
/// （Kotlin 通过 pollEvents 轮询接收）。
#[derive(Debug, Default, Clone, Copy)]
pub struct DefaultTaskManager;

impl DefaultTaskManager {
    fn registry() -> &'static Mutex<HashMap<String, TaskHandle>> {
        static REGISTRY: OnceLock<Mutex<HashMap<String, TaskHandle>>> = OnceLock::new();
        REGISTRY.get_or_init(|| Mutex::new(HashMap::new()))
    }

    fn push_progress_event(task: &TaskHandle) {
        let event = TaskProgressEvent {
            task_id: task.task_id.clone(),
            state: task.state.clone(),
            progress: task.progress,
            message: task.message.clone(),
        };
        // Kotlin NativeEventPump 期望事件为 {method, params} 结构（与旧 ws 事件对齐）。
        let payload = serde_json::json!({
            "method": "task.progress",
            "params": event,
        });
        push_native_delta_event(payload);
    }
}

impl TaskManager for DefaultTaskManager {
    fn create_task(&self, task_id: &str) -> Result<TaskHandle, String> {
        let normalized = task_id.trim();
        if normalized.is_empty() {
            return Err("taskId is required.".to_string());
        }
        let now = now_rfc3339();
        let handle = TaskHandle {
            task_id: normalized.to_string(),
            state: TaskState::Pending,
            progress: 0.0,
            message: "pending".to_string(),
            created_at: now.clone(),
            updated_at: now,
        };
        let mut guard = Self::registry()
            .lock()
            .map_err(|_| "任务注册表锁污染".to_string())?;
        guard.insert(normalized.to_string(), handle.clone());
        Self::push_progress_event(&handle);
        Ok(handle)
    }

    fn update_task(
        &self,
        task_id: &str,
        state: TaskState,
        progress: f64,
        message: &str,
    ) -> Result<TaskHandle, String> {
        let normalized = task_id.trim();
        if normalized.is_empty() {
            return Err("taskId is required.".to_string());
        }
        let now = now_rfc3339();
        let mut guard = Self::registry()
            .lock()
            .map_err(|_| "任务注册表锁污染".to_string())?;
        let handle = guard.get_mut(normalized).ok_or_else(|| {
            format!("task not found: {normalized}")
        })?;
        handle.state = state;
        handle.progress = progress.clamp(0.0, 1.0);
        handle.message = message.to_string();
        handle.updated_at = now;
        let snapshot = handle.clone();
        Self::push_progress_event(&snapshot);
        Ok(snapshot)
    }

    fn get_task(&self, task_id: &str) -> Result<TaskHandle, String> {
        let normalized = task_id.trim();
        let guard = Self::registry()
            .lock()
            .map_err(|_| "任务注册表锁污染".to_string())?;
        guard
            .get(normalized)
            .cloned()
            .ok_or_else(|| format!("task not found: {normalized}"))
    }

    fn cancel_task(&self, task_id: &str) -> Result<TaskHandle, String> {
        self.update_task(task_id, TaskState::Cancelled, 0.0, "cancelled")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_task_manager_lifecycle() {
        let manager = DefaultTaskManager;
        let handle = manager.create_task("ws-init-1").unwrap();
        assert_eq!(handle.state, TaskState::Pending);
        assert_eq!(handle.progress, 0.0);

        let running = manager
            .update_task("ws-init-1", TaskState::Running, 0.5, "extracting rootfs")
            .unwrap();
        assert_eq!(running.state, TaskState::Running);
        assert_eq!(running.progress, 0.5);
        assert_eq!(running.message, "extracting rootfs");

        let completed = manager
            .update_task("ws-init-1", TaskState::Completed, 1.0, "ready")
            .unwrap();
        assert_eq!(completed.state, TaskState::Completed);

        let query = manager.get_task("ws-init-1").unwrap();
        assert_eq!(query.state, TaskState::Completed);

        // cancel 不存在任务应报错
        assert!(manager.cancel_task("nope").is_err());
    }

    #[test]
    fn default_task_manager_progress_clamped() {
        let manager = DefaultTaskManager;
        manager.create_task("clamp-1").unwrap();
        let handle = manager
            .update_task("clamp-1", TaskState::Running, 3.5, "over")
            .unwrap();
        assert_eq!(handle.progress, 1.0);
        let handle = manager
            .update_task("clamp-1", TaskState::Running, -1.0, "under")
            .unwrap();
        assert_eq!(handle.progress, 0.0);
    }

    #[test]
    fn default_task_manager_missing_task_errors() {
        let manager = DefaultTaskManager;
        assert!(manager.get_task("missing").is_err());
        assert!(manager
            .update_task("missing", TaskState::Running, 0.5, "x")
            .is_err());
        // 空 taskId 拒绝创建
        assert!(manager.create_task("  ").is_err());
    }

    #[test]
    fn task_state_from_str_parses_all_variants() {
        assert_eq!(TaskState::from_str("Pending").unwrap(), TaskState::Pending);
        assert_eq!(TaskState::from_str("running").unwrap(), TaskState::Running);
        assert_eq!(
            TaskState::from_str("completed").unwrap(),
            TaskState::Completed
        );
        assert_eq!(
            TaskState::from_str("done").unwrap(),
            TaskState::Completed
        );
        assert_eq!(
            TaskState::from_str("Failed,rootfs download timeout").unwrap(),
            TaskState::Failed("rootfs download timeout".to_string())
        );
        assert_eq!(
            TaskState::from_str("cancelled").unwrap(),
            TaskState::Cancelled
        );
        assert_eq!(
            TaskState::from_str("canceled").unwrap(),
            TaskState::Cancelled
        );
        assert!(TaskState::from_str("").is_err());
        assert!(TaskState::from_str("weird").is_err());
    }
}