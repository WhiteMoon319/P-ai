# 原生 RPC 协议

P-AI Android 进程内 Rust↔Kotlin 通信协议。**唯一契约来源：`contracts/native-rpc/`**。

## 传输

- JSON-RPC 2.0 over JNI。
- 请求：Kotlin `PaiNative.call(requestJson)` → Rust `native_bridge` → 响应 JSON。
- 事件：Rust `push_native_delta_event` 进单一队列 → Kotlin `NativeEventPump` 轮询消费。
- 超时：普通查询 15s；长任务 600s 或任务状态机。

## 契约文件

| 文件 | 内容 |
|---|---|
| `contracts/native-rpc/methods.json` | 请求方法名（129 个，按类别） |
| `contracts/native-rpc/events.json` | 事件名（保活/通知/流式/迁移进度） |
| `crates/pai-protocol/` | 类型 + `include_str!` 契约 + 一致性测试 |

## 方法类别

- bridge: bridge.ping / frontend_ready_start_remote_im_services
- app: get_app_version / check_github_update / get_web_access_info / set_ui_language
- config: load/save/patch_config / load_agents / save_agents / api_config.*
- conversation: conversation.list / create / delete / rename / pin / compact / rewind / model.list
- chat: chat.send / chat.stop / get_prompt_preview
- archives: archives.list / summary / delete / unarchive
- migration: check_message_store_migration / run_message_store_migration
- memory: list_memories / delete_memory / search_memories_recall / search_memories_mixed / 绑定管理
- delegate: delegate.conversations.list / statuses / abort / delete / submit
- goal: goal.current / create / cancel
- task: task_list_tasks / create / update / delete / get / list_run_logs / optimize_draft / complete
- mcp: mcp_list_servers / save / remove / deploy / undeploy / validate / fix / list_server_tools / list_skills
- remote_im: remote_im_list_channels / list_contacts / get_channel_status / restart_channel / get_channel_logs / get_contact_logs / delete_contact / update_contact_* / weixin_oc_*
- workspace: get_android_workspace_status / init / repair / reset / import_rootfs / android_workspace.list / readText / writeText / move / delete / import / export / glob / grep
- storage: get_storage_usage_overview / refresh / cleanup / get_usage_overview
- media: read_chat_image_data_url / read_avatar_data_url / stt_transcribe
- runtime_log: list_recent_runtime_logs / list_runtime_logs_since / clear / append_probe / llm_round_logs

完整清单见 `contracts/native-rpc/methods.json`（代码生成自 `crates/pai-protocol/src/methods.rs` 的一致性测试断言）。

## 事件

| 事件 | 方向 | 说明 |
|---|---|---|
| app.keepAlive | Rust→Kotlin | 保活前台服务启停 |
| app.notification / app.notification.clear | Rust→Kotlin | live update 通知 |
| chat.assistantDelta | Rust→Kotlin | 流式正文/思考/工具/回合终态 |
| chat.roundFinished | Rust→Kotlin | 回合完成/失败终态 |
| messageStore.migration.progress | Rust→Kotlin | 迁移进度 |

## 修改规则

1. 新增方法：更新 `methods.json` + Rust dispatch + Kotlin service + `pai-protocol` 测试断言。
2. 新增事件：更新 `events.json` + Rust 推送点 + Kotlin handleNotification。
3. 一致性测试：`cargo test -p pai-protocol` 遍历契约，未注册方法失败。
4. 禁止字符串错误伪装成功；失败走 JSON-RPC error 结构。
