# 前后端通信、聊天调度与前台恢复状态机

> 本文描述当前实现，不是理想化协议。代码变化后，本文中的方法名、事件名和状态含义必须同步核对。
>
> 最近核对时间：2026-07-17

## 1. 文档目的

这套聊天系统同时支持：

- 桌面 APP 的 Tauri 窗口；
- Web/移动浏览器；
- VS Code 侧边栏；
- 后台调度、远程 IM 激活和委托等非前台来源。

它们共享同一个后端会话和调度器，但不共享同一种传输介质，也不共享同一种前端生命周期。最容易出问题的地方不是“模型有没有返回”，而是下面几层状态在时间上发生了错位：

```text
持久化消息真相
    ↓
后端会话运行态 / 流式缓存
    ↓
传输通道或通知订阅
    ↓
前端当前投影
    ↓
用户可见气泡
```

因此，本文重点回答五个问题：

1. 一条消息从发送到完成，究竟经过哪些层？
2. APP Channel 与 Web WebSocket 的职责差异是什么？
3. 为什么“连接还活着”不等于“当前会话流还活着”？
4. 前台恢复时为什么优先恢复目标消息或订阅，而不是刷新页面？
5. 哪些边界必须保守地走完整会话兜底？

## 2. 先固定三个真相层

### 2.1 持久化消息真相层

消息真相由会话服务和消息存储负责。它回答：

- 这条 user/assistant 消息是否已经存在；
- 消息的正式 ID 是什么；
- 完成后的正文、工具调用、附件和 metadata 是什么；
- 会话最后一条正式消息是谁。

重要约束：assistant 流式轮次启动前，后端会先分配真实 `assistantMessageId`，调用 `bootstrap_streaming_assistant_message` 创建真实 assistant 消息，再开始发送 `chat.roundStarted` 和增量。因此，前端流式期间显示的 assistant 目标不是“等结束后才创建的草稿消息”。

普通输入提交期间前端可能暂存输入显示态，但这不是后台 assistant 消息草稿；assistant 目标始终使用后端已经分配的真实消息 ID。任何前端暂存态都不能替代持久化真相。

### 2.2 后端运行态层

调度器按 `conversation_id` 保存运行槽位。当前主要状态是：

| 状态 | 含义 | 是否表示仍可能产生 assistant 增量 |
| --- | --- | --- |
| `idle` | 当前会话没有正在执行的主助理轮次 | 否 |
| `organizing_context` | 正在整理上下文、归档、压缩或准备执行 | 是，可能尚未开始正文流式 |
| `assistant_streaming` | 主助理正在输出或执行工具循环 | 是 |

运行槽位同时保存：

- `isProcessing`：是否仍被处理声明占用；
- `hasPendingQueue` / `pendingQueueCount`：是否有待处理消息；
- `streamCache`：当前轮次的运行时投影缓存。

`conversation.runtimeSnapshot` 返回的是这个层的轻量快照，不读取整段历史。

### 2.3 前端投影层

前端需要把后端事实投影成：

- 当前消息列表；
- 当前 assistant 目标消息；
- `busy` / `chatting` / round phase；
- 文本、思维链块、工具状态；
- 当前传输绑定和 generation。

前端投影可以暂时落后于后端，也可以在浏览器休眠后丢失订阅，但不应反过来修改后端真相。恢复逻辑的任务是重新建立投影，而不是重新制造一轮回答。

## 3. 正常聊天执行链

### 3.1 发送与入队

正常路径可以概括为：

```text
前端 sendChat()
  → invokeTauri / Web JSON-RPC
  → 后端接收 ChatPendingEvent
  → 按 conversation_id 入队或直接处理
  → 处理同一会话的一批消息
  → 写入正式 user/任务/委托消息
  → 广播 historyFlushed
  → 如果 activate_assistant=true，启动主助理轮次
```

队列事件包括：

- 事件 ID；
- 会话 ID；
- 要写入的消息集合；
- 是否激活 assistant；
- session 信息；
- 可能的 assistant 消息 ID；
- 来源和远程上下文。

同一会话的批处理是调度边界。前端不能因为收到一条增量就假设所有排队消息已经落盘；必须以 `historyFlushed` 或轻量运行快照为依据。

### 3.2 启动真实 assistant 轮次

启动主助理时，后端按下面顺序执行：

1. 生成 `activationId`，当前实现中通常与本轮 trace/request 标识关联。
2. 确定 `requestId`。
3. 确定真实 `assistantMessageId`，没有传入时生成 UUID。
4. 调用 `bootstrap_streaming_assistant_message` 创建真实 assistant 消息。
5. 初始化会话的 `streamCache`，写入 activation/request/assistant message ID 和开始时间。
6. 发出 `chat.roundStarted`。
7. 将会话运行态切换为 `assistant_streaming`。
8. 开始调用模型和工具循环。

这一步是首条 assistant 气泡可靠性的核心：前端可以在收到 `chat.roundStarted` 后立即以真实 ID 建立投影，不需要猜测或等待最终消息。

### 3.3 增量事件

模型和工具循环产生 `AssistantDeltaEvent`。事件可能包含：

- `delta`：普通文本增量；
- `kind`：工具状态、工具事件、工具结果、思维链增量等；
- `activationId` / `requestId`；
- `phaseId`；
- `message`、工具名、工具调用 ID；
- `streamCache`：后端更新后的完整运行时投影快照。

后端在把事件交给前端前，会先更新自己的 `streamCache`。这样即使前端错过了若干增量，恢复时也可以拿到当前完整投影，而不是只能等待下一颗 delta。

### 3.4 完成与收口

模型轮次结束后，后端执行：

1. 将最终 assistant 消息写入正式历史；
2. 发出 `chat.roundFinished`，其中可带完整 `assistantMessage`；
3. 将运行态切回 `idle`；
4. 清理会话 `streamCache`；
5. 更新会话列表活动和已读相关标记。

前端收到 `chat.roundFinished` 时，应先合并正式 assistant 消息，再移除流式 metadata，最后清理本地 streaming 状态。这样可以避免“先删投影、后一帧才插正式消息”的闪烁或气泡短暂消失。

失败收口与完成收口使用同一条生命周期原则：运行态必须回到 `idle`，前端必须结束流式状态，错误信息只表达真实失败。

## 4. 两种传输模型

### 4.1 桌面 APP：Tauri Channel

桌面 APP 通过 Tauri invoke 调用 Rust command。流式增量通过 `tauri::ipc::Channel<AssistantDeltaEvent>` 发送。

### 4.1.1 发送轮次 Channel

每次发送可以带一个与本次提交绑定的 delta channel。后端会把事件 ID 到 channel 的映射保存在 `pending_chat_delta_channels`，因此本次提交可以收到自己的流式事件。

这条通道适合“发起这轮请求的前端实例”。它不应该被当作永久前台订阅：窗口隐藏、会话切换或前端 generation 变化后，旧 channel 可能仍会晚到事件。

### 4.1.2 活动会话绑定 Channel

前台窗口另外通过：

- `bind_active_chat_view_stream`；
- `unbind_active_chat_view_stream`；
- `probe_active_chat_view_stream`；

维护“窗口 label → 当前会话 ID → Channel”的绑定。

绑定层使用以下防护：

- `boundChannelSeq`：新绑定会使旧绑定失效；
- 会话 ID 检查：事件必须属于当前会话；
- generation 检查：旧轮次事件不能覆盖新轮次；
- 发送失败后移除失效 binding。

### 4.1.3 APP 探针

APP 探针不是只检查 command 是否返回。Rust 会向绑定的 Channel 发送特殊 `stream_probe` 事件，前端收到带有 `probeId` 的事件后才判定探针成功。

```text
前端 probeBoundChannel()
  → invoke probe_active_chat_view_stream
  → Rust 查找当前窗口/会话绑定
  → Rust 向绑定 Channel 发送 stream_probe
  → 前端收到相同 probeId
  → healthy
```

如果 command 返回成功但 Channel 没有收到回环事件，探针仍然失败。这避免把“RPC 可用”误判成“增量订阅可用”。

### 4.2 Web/移动浏览器：WebSocket + JSON-RPC

Web/VS Code 侧边栏通过 `/chat` WebSocket 连接。消息格式是 JSON-RPC 2.0：

```json
{
  "jsonrpc": "2.0",
  "id": 42,
  "method": "conversation.runtimeSnapshot",
  "params": { "conversationId": "conversation-1" }
}
```

响应使用同一个 `id` 对应请求；服务器主动推送的事件没有 `id`，而是：

```json
{
  "jsonrpc": "2.0",
  "method": "chat.assistantDelta",
  "params": {
    "conversationId": "conversation-1",
    "event": { "delta": "..." }
  }
}
```

### 4.2.1 Web 客户端身份

每条 `/chat` WebSocket 连接生成一个随机 `client_id`。后端维护两张关键表：

```text
IDE_CONTEXT_CHAT_CLIENTS
  client_id → outbound WebSocket sender

IDE_CONTEXT_CHAT_CLIENT_CONVERSATIONS
  client_id → 已注册的 conversation_id
```

`conversation.open` 和 `conversation.create` 会注册当前 client 对应的会话。断开时，两张表和会话占用都会清理。

### 4.2.2 Web 通知路由

聊天广播不是“所有连接都收到所有增量”。后端通过 `client_id → conversation_id` 映射筛选目标连接，只向已经注册当前会话的 Web client 发送通知。

这带来一个非常重要的边界：

```text
WebSocket 连接仍然打开
    ≠
当前 conversation 订阅映射仍然存在
```

浏览器切后台、代理重连、服务端连接清理或异常恢复，都可能只丢映射而不立即表现为 WebSocket 关闭。

### 4.2.3 Web 探针

Web 不能调用 APP 的 `probe_active_chat_view_stream`，因为该 command 依赖 Tauri Window 和本机 Channel，所以它被显式列为 Web native-only。

Web 使用协议级探针替代：

```text
前端注册 probeId 等待器
  → conversation.streamProbe
  → Rust 校验 opened_conversation_id 与 client 映射
  → Rust 向当前会话的 Web client 发送 chat.streamProbeAck
  → 前端校验 conversationId + probeId
  → RPC delivered=true 且收到 ack 才算 healthy
```

只收到 `delivered=true` 还不够；`delivered` 说明服务端至少成功投递到某个 outbound sender，最终必须由当前连接收到自己的 ack。

## 5. 后端到前端的事件语义

| 事件 | 发送时机 | 前端主要动作 | 是否代表正式消息完成 |
| --- | --- | --- | --- |
| `chat.historyFlushed` | 队列消息写入正式历史后 | 合并已落盘消息，处理新一轮 generation | 否 |
| `chat.roundStarted` | assistant 真实消息已创建、流式即将开始 | 建立真实 assistant 目标和等待态 | 否 |
| `chat.assistantDelta` | 文本、思维链、工具状态或工具结果发生变化 | 更新目标消息投影和运行态 | 否 |
| `chat.roundFinished` | 最终 assistant 消息已生成 | 合并正式消息、结束流式、清理状态 | 是 |
| `conversation.runtimeStateUpdated` | 会话运行态改变 | 更新会话列表运行标记 | 否 |
| `conversation.messageAppended` | 正式消息追加 | 按 ID 合并消息 | 可能是，但不能代替 roundFinished 收口 |
| `chat.streamProbeAck` | Web 探针请求收到 | 只解决对应 probe 等待器 | 否 |

### 5.1 为什么 `chat.assistantDelta` 不是最终真相

增量事件可能丢失、乱序、晚到或被旧 generation 丢弃。它的职责是低延迟投影，不是保证历史完整性。

前端必须遵守：

- 事件属于当前会话才处理；
- 事件属于当前轮次才处理；
- 有 `streamCache` 时优先用缓存投影，避免重复拼接 delta；
- 完成态以正式 assistant 消息或按 ID 读取结果为准。

## 6. 流式缓存的职责和边界

当前 `streamCache` 包含或可能包含：

| 字段 | 用途 |
| --- | --- |
| `activationId` | 识别一次 assistant 激活轮次 |
| `requestId` | 识别对应请求 |
| `assistantText` | 当前已累计的正文投影 |
| `streamBlocks` | 文本、思维链、工具块结构 |
| `toolStatusText` / `toolStatusState` | 当前工具执行状态 |
| `startedAt` / `startedAtMs` | 流开始时间和计时恢复 |
| `updatedAt` | 后端缓存最近一次有意义更新的 revision |
| `hasVisibleProgress` | 是否有可见正文、工具状态或块变化 |
| `persistedAssistantMessageId` | 运行投影对应的真实 assistant 消息 ID |

### 6.1 缓存更新规则

后端只有在可见流式事件、工具状态或工具块发生变化时更新缓存。纯控制事件不应伪造正文进度。

前端收到带 `streamCache` 的事件时，应把它视为“当前投影快照”，而不是再把同一事件的 `delta` 拼接一次。当前 Web 逻辑使用 `hasStreamCache` 防止重复追加。

### 6.2 缓存不能单独证明运行中

完成收口的短窗口内，缓存可能短暂残留；反过来，流刚开始时缓存也可能还没有可见文本。因此：

- `runtimeState`、`isProcessing`、pending queue 决定后端是否仍在运行；
- `streamCache` 只用于恢复显示和身份对账；
- `hasVisibleProgress=false` 不能单独说明“流已经断了”；
- `streamCache` 有内容也不能单独说明“后端仍在流式”。

## 7. 前端状态机

### 7.1 APP 前端

APP chat flow 维护 round phase，主要可见阶段为：

- `idle`；
- `queued`；
- `waiting`；
- `streaming`。

APP 还保存：

- 当前 generation；
- 当前 activation/request；
- 当前 assistant message ID；
- 当前绑定 Channel；
- 每个会话的本地 stream cache。

### 7.2 Web/侧边栏前端

Web 侧边栏使用：

- `busy`；
- `streamingAssistantMessageId`；
- `streamActivationId`；
- `streamRequestId`；
- `streamRevision`；
- `messages` 中带 `_streaming` metadata 的目标 assistant 投影。

它不依赖 Tauri Channel，而依赖当前 WebSocket 的 notification handler。

### 7.3 共享前台恢复决策

APP 和 Web 共用 `decideForegroundRecovery`。输入包括：

- 后端是否 `assistant_streaming`；
- 前端是否仍处于流式或忙态；
- 前后端 assistant message ID；
- activation ID；
- request ID；
- 后端与前端 revision；
- 当前探针状态：`unknown`、`healthy`、`unhealthy`。

输出是平台无关的动作语义：

| 动作 | 语义 |
| --- | --- |
| `keep` | 当前投影和订阅可信，不能刷新页面或消息 |
| `probe_stream` | 两边都认为正在流式，需要验证增量链路 |
| `resume_stream` | 订阅、轮次身份或投影已不可信，优先恢复目标流 |
| `refresh_target_message` | 后端已完成或前端状态过期，只读取目标正式消息 |
| `reload_conversation` | 轻量恢复缺少必要身份或失败，最后才完整打开会话 |

决策顺序必须保持：

```text
运行态
  → assistant 目标 ID
  → activation/request 身份
  → revision
  → 订阅探针
  → 平台恢复动作
  → 目标消息读取
  → 完整会话兜底
```

“平台差异”只能出现在最后的适配动作：APP 绑定 Tauri Channel，Web 重建 WebSocket 会话映射。不能因为平台不同而复制两套互相漂移的判定逻辑。

## 8. 前台恢复完整流程

### 8.1 触发条件

Web/移动浏览器监听：

- `visibilitychange`；
- `focus` / `blur`；
- `pageshow`。

APP 使用窗口活动状态同步和对应的前台生命周期事件。

恢复必须具备幂等性：同一次切屏可能触发多个事件，但不能重复打开会话、重复插入消息或重复启动轮次。

### 8.2 第一步：只检查传输可用性

Web 先调用 `bridge.ping`。它只证明：

- WebSocket 仍能发 RPC；
- 服务端仍能回 RPC。

它不能证明当前 conversation 的通知映射存在，所以 ping 成功后仍要继续做 runtime snapshot 和 stream probe。

APP 的 Tauri invoke 可用也不能证明 active view binding 仍然有效，必须使用 Channel 探针。

### 8.3 第二步：读取 runtime snapshot

恢复读取 `conversation.runtimeSnapshot`，不直接整读 `Conversation.messages`。这是为了：

- 先判断后端是否还在跑；
- 取得目标 assistant ID；
- 取得 stream cache 用于恢复投影；
- 避免每次切前台都重载完整历史。

### 8.4 第三步：两边都流式

```text
后端 streaming + 前端 streaming
  ├─ 目标 assistant ID 缺失或不一致 → resume_stream
  ├─ activation/request/revision 明显落后 → resume_stream
  └─ 身份一致 → probe_stream
       ├─ 探针健康 → keep，不改 DOM
       └─ 探针失败 → resume_stream
```

Web 在 `resume_stream` 时：

1. 调用 `conversation.resumeSubscription` 重新注册当前 client 到 conversation；
2. 重新取得 runtime snapshot；
3. 再次执行 notification probe；
4. probe 成功后，把 runtime stream cache 应用到同一个目标 assistant；
5. 只有上述步骤无法确认目标或订阅时，才 `conversation.open`。

APP 对应动作是重新绑定活动 Channel；如果无法恢复，则走 APP 的会话切换恢复路径。

健康分支绝不能做以下事情：

- `conversation.open`；
- 刷新整个消息数组；
- 清掉当前 streaming metadata；
- 重新提交用户消息；
- 生成第二个 assistant 消息。

### 8.5 后端流式、前端不流式

这通常表示浏览器或窗口切后台时丢失了前端 round state，但后端仍在生成。

处理顺序：

1. 使用后端 `persistedAssistantMessageId`；
2. 恢复订阅或绑定；
3. 应用完整 `streamCache` 到这个 assistant；
4. 设置前端 busy/streaming；
5. 继续接收后续增量。

不能只读取正式消息正文，因为正在生成的正文可能仍只存在运行时缓存。

### 8.6 后端已完成、前端仍显示流式

这是“roundFinished 丢失或晚到”的典型场景。

处理顺序：

1. 优先使用前端已有 assistant message ID；
2. 调用 `get_unarchived_conversation_message_by_id`；
3. 只替换消息数组中的对应 ID；
4. 保留稳定渲染 ID；
5. 移除 `_streaming` 和工具状态 metadata；
6. 清理 busy/streaming 状态。

这条路径不应调用 `conversation.open`，因为后端已经给出了明确的完成目标。

### 8.7 两边都空闲

空闲不代表前端一定最新。恢复会再检查 `conversation.freshnessSnapshot` 的最后正式消息 ID：

- 与当前 formal tail 相同：只同步已读状态；
- 不同且能取得 `latestTailId`：按 ID 单条读取并合并；
- 单条读取失败、身份缺失或协议不一致：最后才完整打开会话。

## 9. 为什么不能只依赖 ping

### 问题

WebSocket 可以保持 OPEN，JSON-RPC 请求也能得到响应，但后端的 `client_id → conversation_id` 映射已经丢失。此时：

```text
bridge.ping       成功
conversation.list  成功
chat.assistantDelta 不再到达
```

如果把 ping 当成订阅健康，页面会永久卡在流式中，因为前端既没有收到完成事件，也不会触发恢复。

### 解决

订阅健康必须满足两个条件：

1. RPC probe 的 `delivered=true`；
2. 当前连接收到带相同 `conversationId` 和 `probeId` 的回执通知。

APP 同理：必须收到绑定 Channel 的回环事件，而不能只看 invoke 返回值。

## 10. 为什么不能每次切前台都刷新页面

完整会话刷新不是无害操作：

- 需要读取和规范化较大消息集合；
- 会覆盖当前前端投影；
- 可能丢失滚动位置、折叠状态、工具状态和本地 round metadata；
- 在流式中会把“运行时缓存正文”和“正式历史正文”混为一谈；
- 会放大 Web 移动端切屏频率带来的请求和渲染成本；
- 不能修复单独的订阅映射丢失。

因此完整 `conversation.open` 只能是最后兜底，而不是默认恢复手段。

## 11. 身份、revision 与 generation

### 11.1 assistant message ID

它是最重要的投影锚点。任何恢复动作都必须优先回答：

```text
当前正在恢复的正文，究竟属于哪一个 assistant 消息？
```

如果后端和前端 ID 不一致，不能继续把新 delta 写入旧消息。

### 11.2 activationId / requestId

它们用于区分相邻或重叠的 assistant 激活轮次。会话 ID 相同不等于轮次相同。

典型风险：

- 旧轮次的迟到事件覆盖新轮次；
- 同一会话快速重试后，前端把第二轮误认为第一轮；
- Web 恢复时使用了旧缓存，但后端已经开始新 activation。

因此身份一致性检查必须在探针之后仍然保留，不能因为“通道活着”就忽略轮次身份。

### 11.3 revision

`updatedAt` 是后端 stream cache 最近一次有意义更新的 revision。前端保存最近应用的 revision，用于判断自己的投影是否落后。

当前比较是保守的：

- 任一侧没有 revision 时，不直接判定不一致；
- 两侧都有 revision 且不相同，则不能保持旧投影；
- revision 不同但目标 ID 明确时，优先恢复目标流或目标消息，而不是立即刷新整段会话。

### 11.4 generation

generation 是前端本地顺序保护，不等于后端 requestId。它解决的是：

- 前端会话切换后旧事件晚到；
- 同一前端重复绑定后旧 Channel 回调仍触发；
- `historyFlushed` 使当前 round 进入下一代。

后端身份和前端 generation 是两种不同维度，不能互相替代。

## 12. 事件丢失、乱序和重复的处理原则

### 12.1 事件丢失

丢失 `chat.assistantDelta`：使用 runtime stream cache 恢复当前完整投影。

丢失 `chat.roundFinished`：runtime snapshot 变为 idle 后，按 assistant message ID 单条读取。

丢失 `chat.roundStarted`：如果后端仍 streaming 且有真实 assistant ID，恢复时创建对应前端投影；如果没有 ID，不能猜测，进入完整兜底。

### 12.2 事件乱序

前端必须用会话 ID、assistant message ID、activation/request 和 generation 过滤。不能只按到达时间追加。

### 12.3 事件重复

正式消息按消息 ID 去重；流式快照按目标 ID覆盖；工具块按工具调用 ID 合并。

重复的 `roundStarted` 不应生成第二个 assistant；重复的 `roundFinished` 不应再次插入同一正式消息。

### 12.4 旧 Channel 或旧 Web handler

APP 使用 channel sequence 和 generation；Web 使用当前 active conversation 和 probeId。任何与当前会话不符的事件都应丢弃，而不是尝试“猜测它可能属于当前消息”。

## 13. 远程访问和认证边界

`/chat` 的远程连接在认证前不会加入聊天客户端广播表，因此不会收到会话通知。

认证流程：

```text
WebSocket 建立
  → bridge.ready(authRequired=true)
  → auth.login(password)
  → 服务端签发 authToken
  → 当前 client 加入广播表
  → 才能调用聊天 JSON-RPC 和接收会话通知
```

前端会缓存 token，但 token 过期时 transport 会清理缓存并触发重新发现/登录。恢复逻辑不能把“未认证”误判成“订阅丢失”；应先完成认证，再做 runtime snapshot 和 probe。

连接断开时，服务端会清理：

- outbound sender；
- client → conversation 映射；
- detached conversation 占用。

重连后即使使用同一个浏览器页面，也必须重新注册当前 active conversation。

## 14. 典型边界场景

| 场景 | 真实状态 | 正确动作 |
| --- | --- | --- |
| 切前台后 WebSocket 已关闭 | transport 不可用 | 重连，重新认证，重新注册会话，再对账 |
| WebSocket OPEN，但订阅映射丢失 | ping 成功、probe 无 ack | `resumeSubscription`，再 probe |
| 后端仍流式，前端没有目标 assistant ID | 两边 busy，但投影锚点缺失 | 用后端真实 ID恢复目标投影 |
| 后端完成，前端仍 busy | `runtimeState=idle`、前端 streaming | 按 ID 读取并收口目标消息 |
| 后端流式，但 streamCache 没有 assistant ID | 运行态有，恢复锚点无 | 不能猜 ID，最后完整打开会话 |
| 两边都 idle，最后消息 ID一致 | 正式历史已同步 | 不刷新，只做已读同步 |
| 两边都 idle，最后消息 ID不同 | 前端落后 | 按最新尾消息 ID单条读取 |
| probe RPC 返回成功但无 ack | 只能证明服务端调用成功 | 判定不健康，不能 keep |
| probe ack 的 conversationId 不匹配 | 可能是其他会话广播 | 丢弃 ack，等待超时 |
| 旧 assistant 事件晚到 | message/activation/generation不匹配 | 丢弃，不覆盖当前投影 |
| 完成事件和最后一个 delta 乱序 | 正式消息已存在 | 正式消息按 ID覆盖流式投影 |
| 两个客户端打开同一会话 | 服务端按当前映射广播 | 前端仍必须按自身 active 会话过滤；占用规则由会话层约束 |
| 前端刚切换会话就收到旧会话事件 | 当前会话 ID不匹配 | 丢弃，不回写旧列表 |
| 浏览器 pageshow 与 visibilitychange 同时触发 | 同一恢复窗口多次触发 | `sidebarForegroundReconciling` 保证幂等 |
| 认证刚恢复但 activeConversationId 保留 | socket新、映射旧 | 先 `resumeSubscription`，再对账 |
| 后端进程重启 | discovery/连接失效 | 重新发现、重连、认证；必要时最后完整打开 |

## 15. 大量 QA

### Q1：assistant 消息是前端临时生成的吗？

不是。后端在 assistant 轮次启动前创建真实消息并固定 `assistantMessageId`。前端的流式 metadata 和显示块只是对这条真实消息的运行时投影。

### Q2：为什么要在第一个 delta 之前创建 assistant？

因为恢复和去重必须有稳定锚点。没有真实 ID，切屏后无法判断缓存正文属于哪条消息，也无法按 ID 单条收口。

### Q3：`historyFlushed` 是否表示 assistant 已完成？

不是。它主要表示队列中的输入消息已经写入正式历史。之后仍可能启动 assistant 轮次。

### Q4：`chat.roundStarted` 是否表示已经有正文？

不是。它表示真实 assistant 消息已经建立、运行态即将进入流式；正文可能尚未产生。

### Q5：`chat.assistantDelta` 是否可以作为唯一进度来源？

不能。它是低延迟事件，可能丢失或延迟。后端 `streamCache` 才是恢复时的完整运行投影。

### Q6：为什么不把每次 delta 都持久化成正式消息？

这样会把一次 assistant 轮次拆成大量不可管理的历史碎片，并破坏消息聚合、工具轮次和最终请求体重建。正式消息在轮次边界持久化，流式期间用 runtime cache 投影。

### Q7：Web ping 成功，为什么页面仍可能卡住？

因为 ping 只验证 WebSocket RPC，不能验证当前会话通知映射。映射丢失时，服务器仍然可以回复 ping，却不会把 delta 送给该连接。

### Q8：为什么 Web 不能直接复用 APP 的 probe command？

APP probe 依赖 Tauri Window、`active_chat_view_bindings` 和 Tauri Channel。浏览器没有这些对象，所以 Web 使用 JSON-RPC + 通知 ack 的等价语义。

### Q9：`delivered=true` 不是已经证明成功了吗？

不完全是。服务端可能向某个 outbound sender 投递成功，但当前连接仍可能没有收到。只有当前前端收到了相同 probeId 的 ack，才能判定当前链路健康。

### Q10：为什么恢复时优先 `resumeSubscription`？

因为很多故障只发生在“client 到 conversation 的通知映射”这一层。重新注册映射成本低，不会覆盖消息列表，也不会破坏滚动和本地投影。

### Q11：为什么不能直接 `conversation.open`？

它会读取并应用完整会话结果，成本和副作用都更大；更重要的是，健康连接下它没有必要，订阅丢失时它也不是唯一或最精确的修复方式。

### Q12：什么时候允许完整打开会话？

只有在轻量路径缺少必要身份、目标消息按 ID 读取失败、订阅恢复仍无法确认，或前后端会话身份不一致且无法安全推断时。

### Q13：后端仍流式但前端没有 assistant ID，能否用最后一条消息猜？

不能。最后一条可能是 user、旧 assistant 或工具相关消息。没有后端提供的真实目标 ID时，猜测会把新正文写入错误消息，因此只能走保守兜底。

### Q14：后端 idle、前端仍 streaming 时为什么按 ID读取？

这通常是 `roundFinished` 丢失，而不是历史真的缺失。后端已经完成，正式 assistant 消息可按 ID读取；无需重载整段会话。

### Q15：为什么不能只把 `busy=false`？

只改 busy 会留下 `_streaming` metadata、工具状态和未完成投影，下一次恢复仍会误判为流式。必须先替换正式消息，再完整清理目标状态。

### Q16：为什么 `hasVisibleProgress=false` 不能说明断流？

流刚开始时可能还没有可见文本，工具等待也可能没有正文；反过来完成收口时缓存可能短暂残留。它只描述投影内容，不描述运行生命周期。

### Q17：activationId 和 messageId 都有了，为什么还需要 requestId？

不同入口可能复用或派生 activation 标识。requestId 能帮助区分同一会话相邻请求，尤其适合排查重试和重连后的迟到事件。

### Q18：revision 不同就必须刷新页面吗？

不必须。revision 不同只说明当前投影落后或不一致；如果目标 ID明确，优先恢复目标流或目标消息。

### Q19：为什么前端缺少 revision 时不直接判定不一致？

兼容旧事件、早期 roundStarted 和没有 streamCache 的增量。缺失字段不能被当作“明确冲突”，但如果同时缺少目标 ID，就必须保守处理。

### Q20：为什么要用 generation，而不是只比较时间？

事件到达顺序可能与生成时间不同，且多个请求的时间可能相近。generation 能直接表达“这是不是当前前端轮次”。

### Q21：Web 重连后 activeConversationId 还在，为什么还要重新注册？

因为服务端连接断开时会清理 client 映射。浏览器本地 ref 仍保存会话 ID，不代表服务端知道这个连接订阅了它。

### Q22：如果重连后 `resumeSubscription` 失败怎么办？

恢复链路会记录失败并进入最后的完整会话兜底；如果连接本身还未恢复，则先由 transport 重连/认证流程解决。

### Q23：如果 probe 期间用户切换会话怎么办？

ack 会校验当前 active conversation。旧会话 ack 被丢弃，probe 超时后不会把结果应用到新会话。

### Q24：如果同一个 probe ack 重复到达怎么办？

probeId 第一次解决等待器后会被删除，后续重复 ack 没有 resolver，不会重复执行恢复动作。

### Q25：如果两个恢复事件并发触发怎么办？

Web 使用 `sidebarForegroundReconciling` 串行门闩；APP 的活动状态同步和 generation 也承担同样的防重职责。恢复逻辑必须保持幂等，不能并行 `conversation.open`。

### Q26：为什么两边都 idle 还要查 freshness？

运行态只说明“现在不再执行”，不说明前端已经收到最后一条正式消息。freshness 用最后正式消息 ID做最小对账。

### Q27：为什么 freshness 不直接返回完整消息？

它的职责只是判断是否落后。先拿 ID再按需读取，可以避免每次前台恢复都整读历史。

### Q28：按 ID读取后把消息追加到数组末尾会不会顺序不对？

正常目标是当前尾消息或已有 streaming assistant，通常已在列表中并执行替换；只有目标尚未出现在本地列表时才追加。若无法保证局部顺序，应使用完整会话兜底，不应盲目重排整个历史。

### Q29：为什么保留 stable render ID？

消息业务 ID相同但对象被替换时，Vue 可能重新创建气泡。保留 stable render ID 可以让“按 ID刷新目标消息”不产生不必要的视觉闪烁。

### Q30：Web 通知为什么不广播给所有连接？

避免一个会话的高频 delta 污染其他窗口，也避免不同会话的消息误写入当前投影。服务端先按 client→conversation 映射筛选。

### Q31：同一会话有多个 watcher 会怎样？

通知路由可以向映射到同一会话的多个 Web client 发送；每个客户端仍需校验自己的 active conversation 和本地目标 ID。会话占用规则另行防止不允许的并发编辑。

### Q32：如果后端进程在流式中崩溃，runtime snapshot 还能恢复吗？

不能假设能恢复。此时 transport、discovery 或 runtime snapshot 会失败，轻量恢复无法建立，最终只能显示真实错误或执行重新连接后的完整兜底。不能用默认演示数据假装恢复成功。

### Q33：如果模型没有任何正文但执行了工具，`streamCache` 有意义吗？

有。工具块、工具状态和 reasoning block 都属于可见流式进度，恢复时应通过 `streamBlocks` 和工具状态还原，而不是只看 `assistantText`。

### Q34：如果工具状态事件没有正文，为什么仍要广播？

工具状态是用户可见的运行投影，且可能是唯一进度。它不能被当作最终消息，但不能静默丢弃。

### Q35：如果 `roundFinished` 先到、最后 delta 后到怎么办？

正式消息和完成状态优先，旧 delta 应被 activation/generation/streaming 状态过滤，不能重新把已完成消息标记为流式。

### Q36：为什么不在 Web 恢复时重新提交原 user 消息？

那会重复触发模型、生成第二条 assistant，破坏真实消息 ID和调度幂等性。恢复只能修复订阅和投影，不能重放业务操作。

### Q37：为什么 APP 与 Web 不直接共用一个“恢复函数”？

判定语义必须共用，但动作适配不同：APP 可绑定 Tauri Channel，Web 必须重建 WebSocket 会话映射。当前设计将决策抽成纯函数，将平台动作注入适配器，避免状态机分叉。

### Q38：为什么不把所有状态都放在全局 store？

当前架构按 composable 管理状态；真正重要的是区分持久化真相、后端运行态、传输绑定和前端投影。集中存储不能自动解决层次混淆，反而可能让旧会话状态泄漏到新会话。

### Q39：移动浏览器切后台后，页面一定会收到 pageshow 吗？

不一定。还必须监听 visibilitychange 和 focus，并通过 transport 状态判断是否需要重连。pageshow 是补充触发器，不是唯一触发器。

### Q40：恢复成功时用户应该看到什么？

健康连接时保持当前页面和当前气泡，不出现整页闪烁；订阅恢复时目标 assistant 继续更新；后端已完成时目标气泡从流式状态自然收口。恢复实现不应添加解释性 UI 文案。

## 16. 设计不变量

以下不变量必须在代码审查和测试中持续检查：

1. 每个正在流式的 assistant 投影都能追溯到真实 `persistedAssistantMessageId`。
2. 后端 `assistant_streaming` 不会因为前端暂时没收到 delta 就被改成 `idle`。
3. Web ping 成功不能单独导致 `keep`。
4. `keep` 分支不调用 `conversation.open`，不刷新消息数组。
5. `refresh_target_message` 只替换目标 ID，不整读历史。
6. `resumeSubscription` 不制造新 assistant，不重放 user 输入。
7. 完成消息优先于残留 stream cache。
8. 旧会话、旧 activation、旧 generation 的事件不能覆盖当前投影。
9. 缺少必要身份时宁可完整兜底，也不猜消息归属。
10. 恢复失败不能用 mock、fallback、默认账户或演示数据掩盖。
11. 所有可见错误必须是真实运行错误或产品已有文案来源。
12. 轻量读取失败后才允许完整会话兜底。

## 17. 代码索引

### 后端调度与运行态

- `src-tauri/src/features/chat/scheduler.rs`
  - 队列事件、会话运行态、轮次启动和收口。
- `src-tauri/src/features/chat/scheduler/stream_runtime.rs`
  - stream cache、增量路由、活动窗口 Channel 和 Web 通知投递。
- `src-tauri/src/features/chat/scheduler/round_events.rs`
  - `chat.roundStarted`、`chat.roundFinished` 等事件。
- `src-tauri/src/features/core/domain/runtime_state.rs`
  - `AppState` 中的运行槽位、Channel 和绑定映射。

### APP 前端

- `src/features/chat/composables/use-chat-flow.ts`
  - chat flow 聚合入口。
- `src/features/chat/composables/use-chat-flow-channel-binding.ts`
  - Tauri Channel 绑定、解绑、probe、generation 过滤。
- `src/features/chat/composables/use-chat-flow-foreground-rounds.ts`
  - APP 前台 round 和 stream cache 投影恢复。
- `src/features/chat/composables/use-chat-window-recording-orchestrator.ts`
  - APP 窗口前台恢复触发和共享恢复决策接入。

### Web/侧边栏前端

- `src/features/sidebar/composables/use-ws-transport.ts`
  - WebSocket、JSON-RPC request/response、notification 和认证状态。
- `src/features/sidebar/App.vue`
  - active conversation、事件注册、前台恢复触发和目标消息替换。
- `src/features/sidebar/composables/use-sidebar-assistant-stream.ts`
  - Web assistant 投影、stream cache、目标 ID和 revision。
- `src/features/sidebar/composables/sidebar-foreground-recovery.ts`
  - Web 平台动作适配和共享决策执行。

### Web 桥接后端

- `src-tauri/src/features/system/commands/ide_context/bridge_server.rs`
  - `/chat` WebSocket 生命周期、认证和 client 清理。
- `src-tauri/src/features/system/commands/ide_context/jsonrpc_dispatch.rs`
  - Web JSON-RPC 方法分派和 Web native-only 能力边界。
- `src-tauri/src/features/system/commands/ide_context/chat_methods.rs`
  - `conversation.runtimeSnapshot`、`conversation.resumeSubscription`、`conversation.streamProbe` 等方法。
- `src-tauri/src/features/system/commands/ide_context/bridge_clients.rs`
  - Web client 到会话通知的定向投递。

### 共享恢复决策

- `src/features/chat/composables/foreground-recovery-decision.ts`
  - APP/Web 共用的纯决策函数。
- `src/features/chat/composables/foreground-recovery-decision.spec.ts`
  - 身份、探针和目标缺失的决策测试。
- `src/features/sidebar/composables/sidebar-foreground-recovery.spec.ts`
  - Web 恢复动作测试，验证健康时不刷新、订阅丢失时恢复目标投影、完成时按 ID收口。

## 18. 变更和排障建议

修改通信或状态机前，先回答：

1. 这是持久化真相、后端运行态、传输绑定还是前端投影的问题？
2. 是否可以只增加一个轻量 snapshot、probe 或 message-by-id 接口？
3. 是否会让 APP/Web 的共享判定语义分叉？
4. 是否会把“连接健康”误当作“会话订阅健康”？
5. 是否会在完成态继续保留 `_streaming`？
6. 是否会让旧 generation 或旧 activation 覆盖新消息？
7. 是否会把完整 `conversation.open` 变成默认路径？
8. 是否会引入没有产品来源的可见解释性文案？

推荐排障顺序：

```text
1. 看 transport 是否连接
2. 看认证是否有效
3. 看 runtime snapshot 的 conversationId/runtimeState
4. 看 streamCache 的 persistedAssistantMessageId/activationId/requestId/updatedAt
5. 看 probe RPC 是否 delivered
6. 看当前连接是否收到 probe ack
7. 看目标消息按 ID 是否存在
8. 最后才看完整会话打开和历史合并
```

不要从“页面看起来卡住”直接跳到“刷新整个页面”。先定位是哪一层事实落后，才能保持恢复动作最小、可验证且不会重复触发业务。
