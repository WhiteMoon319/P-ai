# 未发布

## 修复：摘要标题原子账本一致性

- 修复（summary-title-shard-consistency）：自动生成摘要标题后，会话概览与消息正文保持一致。统一 replace / batch replace 的派生标题规则：替换消息若改变摘要标题状态，按替换后消息集合重算 `latest_summary_title`；`update_unarchived_conversation_by_id` 提交 v3 替换时使用统一派生元数据并同步内存缓存，不再用旧派生字段覆盖新标题。
- 修复（summary-title-batch-consistency）：批量替换 `provider_meta` 与单条替换路径共享同一派生规则，最终元数据与最终消息集合一致；替换非最新摘要、删除标题、多消息替换均按最终摘要范围取值，普通消息替换不改变既有正文长度、预览等派生字段行为。

## 功能：MCP 一卡一组，组内多服务器与工具名前缀

- 功能（mcp-group-card）：一张 MCP 卡片即一组服务器，definitionJson 可整体保存多个服务器并整组启停；部署时组内每个服务器独立连接、工具合并，已有单卡单服务器数据自然兼容。
- 功能（mcp-multi-format）：兼容 mcpServers 对象/数组、根级平铺对象、根级数组与单服务器直接字段五种嵌套格式；`headers` 作为 `httpHeaders` 别名，env 支持 `{value, secret}` 对象形态，`transport: "sse"` 与 `type` 别名识别。
- 功能（mcp-tool-prefix）：MCP 工具名统一带 `{成员名}_{工具名}` 前缀暴露给 LLM，按最后一个下划线还原路由到对应成员；组内歧义前缀与跨卡片成员重名在部署/校验阶段报可读错误。
- 功能（mcp-structured-errors）：MCP 校验错误改为结构化错误码 + 参数，前端按 i18n 渲染为可读文案（中/英/繁）。
- 功能（mcp-ai-fix）：校验失败时可通过专家模型一键修复 MCP 配置格式，敏感字段值脱敏占位、修复后还原，结果回填编辑框由用户确认保存。

## 功能：MCP 支持 SSE transport

- 功能（mcp-sse-transport）：MCP 客户端新增 legacy SSE（HTTP+SSE）传输支持，连接 SSE 端点、经 endpoint 事件获取 message 地址后 POST JSON-RPC，响应经 SSE 通道异步返回；鉴权头在连接与 message 请求中均携带，不再将 `transport: "sse"` 静默降级为 streamable HTTP。
- 依赖（rmcp-3）：rmcp 升级 2.1.0 → 3.0.1，适配 get_stream 签名变更与 sse-stream 0.2.5 的 API 更名。

## 功能：更新下载代理游标切换

- 功能（updater-download-proxy-cursor）：自动与中转更新下载每次仅使用当前代理游标；请求失败、HTTP 非成功、响应流中断、下载总时限 10 分钟或内容长度不完整时，持久化推进至下一个 HTTPS 下载代理并结束本次更新；完整下载后保持当前游标，直连模式不受影响。

## 修复：更新页面链接使用直连地址

- 修复（updater-release-page-direct-url）：更新窗口的“打开 Releases”始终使用 GitHub 原始发布页地址，不再把网页请求发送到仅支持资源下载的代理；更新检查、清单与安装包下载仍按所选更新方式走原有代理链路。

## 修复：前台会话流式状态恢复

- 重构（chat-runtime-single-path）：主对话的前台恢复、水位与终态收口从录音模块收敛到唯一聊天运行时；APP 与 Web/VS Code 继续复用同一 `main-chat` 入口和传输门面。
- 修复（chat-foreground-tail-watermark）：前台恢复以会话变更水位决定是否读取正式尾消息；水位推进时强制以单条正式消息收口同 ID 的流式半成品，并在读取失败时回退轻量快照。
- 修复（chat-stop-guided-history）：停止回复后，引导消息出队产生的正式历史事件仍会立即合并到当前会话；停止保护仅抑制已停止轮次的等待/流式投影，不再吞掉正式消息。
- 修复（chat-foreground-stream-recovery）：压缩中尚未产生正式 assistant 消息时仅显示会话忙态；收到 `roundStarted` 的正式消息 ID 后才显示对应气泡，避免暴露内部占位身份。
- 修复（chat-stop-preserves-message-activity）：中止调度时直接以正式消息的内容块保存思考与工具活动，不再依赖废弃的前端流式镜像。
- 修复（chat-virtual-measurement）：聊天时间线改为真实行高驱动的虚拟布局，不再以预估高度定位富文本消息，避免首次加载或切换会话时消息短暂重叠。

## 调整：统一设置页章节模板

- 调整（settings-template-sections）：以图像生成配置页为标准，将 API 供应商、Codex、外观、通知、关于、网络访问、聊天设置、快捷键、人格、使用统计和存储迁移页的一级章节标题与说明移到内容卡片外；普通 Codex 认证字段接入 `ConfigTemplate`，保留动态列表、统计表格和专用交互逻辑；开发用 Demo 页保持不变。
- 调整（settings-template-rows）：按“组—行—项”重新整理快捷键、聊天设置、外观、通知、人格和日志页；拆分默认模型、STT、录音参数、记忆设置、日志容量等独立设置项，取消 API/Codex 动态模型编辑器中的多字段并排布局，保留原有事件、保存和动态管理逻辑。

## 修复：计划确认提示词仅发送一次

- 修复（plan-confirm-one-shot-prompt）：用户确认执行计划后，计划路径提示词仅随该次继续执行请求发送；后续普通消息与上下文压缩不再重复回灌活动计划。

## 优化：聊天模型名称紧凑显示

- 优化（chat-model-label-separator）：聊天输入栏的模型名称以中点分隔供应商、模型和思维强度；供应商在紧凑栏内最多显示两个字符，完整名称仍可通过悬浮提示和下拉菜单查看。
- 修复（chat-status-banner-info-tone）：补齐聊天状态横幅的 `info` 类型，并让上下文压缩状态显式使用该样式。

## 调整：协调与远程客服预设权限白名单

- 调整（leader-and-remote-customer-service-whitelists）：leader 与远程客服部门的默认权限改为白名单；leader 保留协调、核验和联网检索所需的最小工具，远程客服保留新闻检索、媒体阅读、表情包和图片创作能力。所有内置预设部门的默认 Skill 白名单均包含 `memory-generation`，远程客服额外启用 `news-analyst`。

## 修复：删除部门时同步断开树关系

- 修复（department-delete-prunes-tree-edges）：部门设置页删除自定义部门时，会同步移除所有指向该部门的直属关系；不再因保存时还原旧树边而触发无效子部门校验、导致删除无法保存。

## 调整：预设部门可配置且可还原

- 调整（preset-department-defaults-not-locks）：explorer、reviewer 与 saddler 的名称、说明、模型、模型兜底和权限从强制策略改为可编辑的默认值；加载和保存不再覆盖用户配置。点击“还原”时，前端只传部门 ID，并经统一传输接口从后端获取唯一的完整默认草稿（桌面与 Web 共用），恢复对应的模型与权限白名单。

## 更新：固定预设部门与能力资产约束

- 功能（fixed-preset-departments）：新增固定的 explorer、reviewer 与 saddler 部门，将其固定为主助理的直属下级，并预置各自独立的模型、工具和 Skill 白名单；配置加载会自动补齐或修正这些内置部门与关联。
- 修复（saddler-pai-scope）：为 saddler 部门的文件与终端工具增加运行时范围校验，仅允许在当前项目 `.pai/` 目录内写入或更新能力资产。
