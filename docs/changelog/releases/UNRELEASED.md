# 未发布

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
