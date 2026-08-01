# 未发布

## 修复

- 修复新建“隔离工作树”会话时 Git 根目录二次校验会弹出控制台窗口的问题：Windows 下以 `CREATE_NO_WINDOW` 执行校验进程。
- 修复后台子进程弹出控制台窗口的遗漏点：Git 幽灵快照、VSCode 桥接网络探测、winget 安装、WSL/Shell 终端启动器、默认程序打开文件，均以 `CREATE_NO_WINDOW` 抑制多余控制台窗口。
- 聊天消息无头像时不再渲染头像占位（含首字母兜底），用户消息靠右、助理名称靠左布局不变。
- 修复 MCP 工具权限兼容名回归：规范化组成员工具命名后丢失了旧格式候选名（`server-id::search` 等 `server_id::provider_tool_name` 组合），已补回。

## 依赖

## 功能
