# 变更日志

## 发布：v0.74.2

## 新增

- **设备控制（Shizuku/root 提权）**：Android 端新增 `device_control.*` 工具集，支持通过 Shizuku（首选）或 root（兜底）提权执行有限设备控制操作。
  - 提权状态查询与 Shizuku 授权引导
  - 应用管理：冻结/解冻/卸载/安装应用（危险操作需二次确认）
  - 文件操作：删除工作区沙盒内受限文件（危险操作需二次确认）
  - 触控：点击/滑动/按键/截屏（v1 经 `input` 命令，shell 权限）
  - 命令白名单枚举，禁止自由 shell 拼接；路径与包名校验
  - 新增预设 skill `device-control` 指引 agent 正确使用 Android 域工具
  - 设置页工具页新增「设备控制」状态卡片

## 修复

- 恢复上游 cherry-pick 时丢失的 96 个 Android 特有 locale 词条（androidWorkspace/remoteFrontend 等）
- 修复设置窗口左侧导航栏无法滚动的问题：导航菜单项较多时现在可以正常滚动到末尾