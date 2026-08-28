# 变更日志

## 发布：v0.74.2

## 新增

- **设备控制（Shizuku/root 提权）**：Android 端新增 `device_control` 工具集，支持通过 Shizuku（首选）或 root（兜底）提权执行有限设备控制操作。
  - 提权状态查询与 Shizuku 授权引导；授权结果经插件事件实时回传前端
  - 应用管理：冻结/解冻/卸载/安装应用（危险操作需二次确认）
  - 文件操作：删除/安装/截屏走 `/sdcard/Android/data/<pkg>/files/device_control` 中转区（shell 与 app 双向可访问）
  - 触控：点击/滑动/按键/截屏（注入式 `injectInputEvent`，Shizuku UserService shell 身份进程内反射注入，MAA-Meow 语义；root 兜底降级 `input` 命令）
  - 能力开关（总开关 + 分项）默认全部关闭，未开启拒绝执行；agent 侧注册 `device_control` 工具（action 分发）
  - 执行域路由：`shell_exec` 终端命令首词命中 Android 系统白名单（pm/cmd/input/am/dumpsys/toybox 等）自动路由到 Android 域提权执行；歧义命令用 `sys:` 前缀显式覆盖；未命中一律落 Linux 域，不静默提权；路由命令含 shell 元字符（`;`/`|`/`&&`/`$()` 等）拒绝执行防注入；危险路由命令触发用户二次确认
  - 命令白名单枚举，禁止自由 shell 拼接；路径与包名校验
  - 预设 skill `device-control` 指引 agent 正确使用 Android 域工具；设置页工具页「设备控制」卡片（含能力开关组）

## 修复

- 恢复上游 cherry-pick 时丢失的 96 个 Android 特有 locale 词条（androidWorkspace/remoteFrontend 等）
- 修复设置窗口左侧导航栏无法滚动的问题：导航菜单项较多时现在可以正常滚动到末尾