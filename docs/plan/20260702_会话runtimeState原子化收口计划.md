# 会话 runtimeState 原子化收口计划

## 背景

会话列表偶发出现“会话仍被锁定、无法正确释放”的表现。现有链路中，`runtimeState` 是列表是否展示忙碌、整理中、不可点击的关键字段。

当前代码里存在两类不一致：

- 部分路径调用 `set_conversation_runtime_state(...)` 后没有广播 `conversation.runtimeStateUpdated`，前端列表可能长期停留在旧的 `organizing_context` 或 `assistant_streaming`。
- 前端存在基于 `done / failed / completed` 的推断式清理，但后端主会话运行态实际只有 `idle / assistant_streaming / organizing_context`，协议已经漂移。

## 目标

只收口 `runtimeState` 这一个字段，保证字段写入与字段广播原子化：

- 后端每次修改 `runtimeState`，都必须通过唯一入口完成。
- 唯一入口只负责写入 `runtimeState` 并广播这个字段。
- 广播 payload 只包含 `conversationId` 与 `runtimeState`，不夹带其他运行态字段。
- 前端收到 `conversation.runtimeStateUpdated` 后只覆盖 `runtimeState`，不推断或顺手修改其他字段。

## 非目标

本次不处理以下内容：

- 不重构 pending queue、processing claim、stream cache 等其他运行态字段。
- 不引入新的锁模型、owner、phase、reason 或事务对象。
- 不改变会话调度并发策略。
- 不重写会话列表 UI。
- 不把 `runtimeState` 扩展为更复杂的状态机。

## 现状调用点

后端直接调用 `set_conversation_runtime_state(...)` 的业务路径主要包括：

- 聊天调度开始与结束：`src-tauri/src/features/chat/scheduler.rs`
- guided queue 收尾：`src-tauri/src/features/chat/scheduler.rs`
- 压缩开始与结束：`src-tauri/src/features/system/commands/conversation_compaction.rs`
- 压缩后续调度开始：`src-tauri/src/features/system/commands/chat_and_runtime/core_send_inner.rs`
- 用户中断 / 停止：`src-tauri/src/features/system/commands/chat_and_runtime/core_commands.rs`
- Web / VS Code 侧边栏停止：`src-tauri/src/features/system/commands/ide_context.rs`
- 委托运行态清理：`src-tauri/src/features/delegate/runtime.rs`

前端直接消费 `runtimeState` 的关键位置：

- Web 侧边栏会话列表状态合并：`src/features/sidebar/App.vue`
- Web 侧边栏列表禁用与展示：`src/features/sidebar/views/ConversationListView.vue`
- 桌面聊天会话列表禁用与展示：`src/features/chat/components/ChatConversationListCard.vue`
- 桌面聊天侧栏禁用与展示：`src/features/chat/components/ChatConversationSidebar.vue`

## 实施方案

1. 新增后端唯一入口

新增 `set_conversation_runtime_state_and_emit(...)`，参数保持最小：

- `state`
- `conversation_id`
- `new_state`

函数内部只做两件事：

- 调用底层 `set_conversation_runtime_state(...)` 写入字段。
- 调用 `emit_conversation_runtime_state_updated_payload(...)` 广播同一个字段。

2. 降级底层写入函数

保留 `set_conversation_runtime_state(...)` 作为调度模块内的私有底层写入 helper，避免一次性搬动大量测试 fixture。

业务代码禁止新增直接调用；本次会替换非测试业务路径。测试中如只需要构造状态，可暂时保留直接调用。

3. 替换业务调用点

把业务路径中“写状态 + 手写广播”的组合替换为唯一入口。

把“只写状态不广播”的路径也替换为唯一入口，重点覆盖：

- guided queue 收尾
- 用户中断 / 停止
- Web / VS Code 侧边栏停止
- 委托运行态清理

4. 收敛前端消费逻辑

Web 侧边栏收到 `conversation.runtimeStateUpdated` 时只覆盖 `runtimeState`。

移除或弱化基于 `done / failed / completed` 的清理逻辑，避免用不存在于后端主会话协议的状态做判断。

5. 保持事件 payload 原子化

不修改 `ConversationRuntimeStateUpdatedPayload` 的字段结构。

事件继续保持：

- `conversationId`
- `runtimeState`

## 验收标准

- 全仓业务代码不再直接调用 `set_conversation_runtime_state(...)`。
- `set_conversation_runtime_state(...)` 不再对调度模块外暴露。
- 所有业务状态变更都通过 `set_conversation_runtime_state_and_emit(...)`。
- `runtimeState` 切到 `idle` 时，Web 侧边栏和桌面会话列表都能收到字段级更新。
- 前端不再依赖 `done / failed / completed` 来释放主会话运行态。
- 现有 `conversation.runtimeStateUpdated` 事件协议保持兼容。

## 验证计划

最小必要验证：

- 前端：`pnpm typecheck`
- 后端：优先运行和调度 / runtimeState 相关的最小 Cargo 测试。

如果 Cargo 验证被仓库既有无关编译错误阻塞，需在结果中明确记录阻塞点，不扩大修改范围。
