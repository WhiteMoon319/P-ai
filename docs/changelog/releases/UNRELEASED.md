# 未发布

## 功能

## 修复

- 修复（runtime-state-atomic-broadcast）：会话 `runtimeState` 写入收口为后端单字段原子入口，所有业务路径写入后统一广播 `conversation.runtimeStateUpdated`；侧边栏移除基于 `done/failed/completed` 的旧推断清理，避免会话列表因漏推送或协议漂移长期停留在锁定态。
