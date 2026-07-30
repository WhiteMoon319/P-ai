# 未发布

## 修复：前台会话流式状态恢复

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
