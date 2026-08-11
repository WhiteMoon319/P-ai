# native-rpc 协议契约

P-AI Android 进程内 Rust↔Kotlin 通信的唯一协议来源。

## 传输

- JSON-RPC 2.0 over JNI：Kotlin `PaiNative.call(requestJson)` → Rust `native_bridge` dispatch → 响应 JSON。
- 事件推送：Rust `push_native_delta_event` 进单一队列 → Kotlin `pollEvents` 顺序消费。
- 请求超时：普通查询 15s；长任务（migration / workspace init/repair/reset / rootfs 导入导出）600s 独立超时或走任务状态机。

## 文件

| 文件 | 内容 |
|---|---|
| `methods.json` | 请求方法名唯一清单（按类别分组）。Rust dispatcher 与 Kotlin ChatService 必须一致。 |
| `events.json` | Rust→Kotlin 事件清单（保活 / 通知 / 流式 delta / 迁移进度）。 |

## 修改规则

1. 新增 RPC 方法：同时更新 `methods.json`、Rust `native_bridge`/`jsonrpc_dispatch` 分支、Kotlin service 方法。
2. 新增事件：同时更新 `events.json`、Rust 推送点、Kotlin `handleNotification` 分支。
3. 一致性测试：遍历 `methods.json`，断言 Rust dispatch 存在对应分支；未注册方法在测试阶段失败。
4. 禁止用字符串错误伪装成功；失败必须走 JSON-RPC error 结构。
