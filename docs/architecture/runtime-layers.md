# 运行时层

## 进程模型

单个 Android 进程：

```
Kotlin (Compose UI)
   │  JSON-RPC over JNI（PaiNative.call / pollEvents）
   ▼
Rust (.so: libeasy_call_ai_lib.so)
   ├─ Tokio runtime（自建 8MB 栈，多线程）
   ├─ AppState（配置/会话/记忆/运行时状态）
   └─ native_bridge（dispatch：请求分发 + 事件队列）
```

## 层职责

| 层 | 职责 | 禁止 |
|---|---|---|
| apps/android | Activity/Application、Compose、ViewModel、前台服务、通知、录音、SAF、FileProvider、JNI 请求与事件消费、权限 | 复制 Rust 业务逻辑 |
| pai-backend | 会话/消息/流式/工具/记忆/MCP/Skill/任务/Goal/Delegate/远程IM/配置/migration | 依赖 tauri/jni/Android/Kotlin/NativeAppHandle |
| pai-android-bridge | nativeInit/nativeCall、JSON-RPC 编解码、runtime 初始化、请求分发、事件队列、取消/任务句柄、JNI 错误转换 | 塞入业务实现 |
| pai-android-platform | workspace、rootfs 下载/校验/解压、proot、native libs 查找、Android TLS、沙盒路径、文件导入导出 | 依赖 UI |

## 数据流

### 请求（Kotlin → Rust）

```
Compose → ViewModel → ChatService → NativeRpcClient.call(method, params)
  → PaiNative.call(JSON) → native_bridge.dispatch → 共享 inner → 响应 JSON → RpcResponse
```

### 事件（Rust → Kotlin）

```
Rust push_native_delta_event(event) → NATIVE_DELTA_QUEUE
  → Kotlin NativeEventPump.pollEvents() 轮询 → notifications flow
  → AppViewModel.handleNotification（顺序消费，禁止并发 emit / collectLatest）
```

事件包括：`chat.assistantDelta`（流式正文/思考/工具/回合终态）、`chat.roundFinished`、
`app.keepAlive`、`app.notification(.clear)`、`messageStore.migration.progress`。

### 长任务（workspace/rootfs/migration）

```
start RPC → 同步返回 taskId / 或阻塞等待（callLong 600s）
  → 进度事件（workspace.status / migration.progress）
  → UI 轮询 get_android_workspace_status 或消费进度事件
  → complete / failed（失败带可读错误）
```

## 保活

`app.keepAlive {active}` → `PaiForegroundService.start/stop`（API 34+ specialUse 类型）。
active=true 于回复轮次/目标激活时推送，active=false 于完成/结束时推送。幂等（AtomicInteger 计数）。

## 关键约束

- 流式事件顺序由单一队列 + 顺序 tryEmit 保证；禁止重新引入并发 launch emit 或 collectLatest。
- 路径访问必须经过 Android 沙盒与 symlink 防护（rootfs 解压拒绝绝对路径/`..`/非法 link）。
- 禁止 unwrap/expect（测试除外）；失败必须返回可读错误，不返回假成功。
