use super::*;

// ==================== 远程前端模式通知命令 ====================
// 远程模式下手机 PAI 壳层把 iframe 转发的电脑 PAI 聊天事件转成 Android 通知。
// 平台实现（含通知构建）在 features/chat/scheduler/live_update.rs 的
// remote_live_update_notify_android，此处只做命令包装。
// 注意：命令函数保持私有（不加 pub），避免 tauri::command 宏为 pub 函数生成
// #[macro_export] 宏，在 include! 文本展开场景下与其他命令的宏命名冲突。

