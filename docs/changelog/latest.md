# 变更日志

## 发布：v0.74.2

## 新增

- **设备控制（Shizuku/root 提权）**：Android 端新增 `device_control` 工具集，支持通过 Shizuku（首选）或 root（兜底）提权执行有限设备控制操作。
  - 提权状态查询与 Shizuku 授权引导；授权结果经插件事件实时回传前端
  - 应用管理：冻结/解冻/卸载/安装应用（危险操作需二次确认）
  - 文件操作：删除/安装走 `/sdcard/Android/data/<pkg>/files/device_control` 中转区；截屏由 shell 身份写 `/data/local/tmp` + base64 回传应用私有目录（规避 sdcard 两侧权限冲突）
  - 触控：点击/滑动/按键/截屏（注入式 `injectInputEvent`，Shizuku UserService shell 身份进程内反射注入，MAA-Meow 语义；root 兜底降级 `input` 命令）
  - 能力开关（总开关 + 分项）默认全部关闭，未开启拒绝执行；agent 侧注册 `device_control` 工具（action 分发）
  - 终端执行域改为**显式环境切换**：设置页/`config "terminal set android|linux"` 切换，exec 命令走对应通道（linux=proot 沙盒，android=Android 域提权 shell，允许正常 shell 语法）；`sys:` 前缀可单命令强制 Android 域；不再做命令首词自动路由；Android 域危险关键字（uninstall/rm -rf 等）在命令任意位置出现都会触发二次确认
  - 命令白名单枚举，禁止自由 shell 拼接；路径与包名校验
  - 预设 skill `device-control` 指引 agent 正确使用 Android 域工具；设置页工具页「设备控制」卡片（含能力开关组）

## 修复

- 恢复上游 cherry-pick 时丢失的 96 个 Android 特有 locale 词条（androidWorkspace/remoteFrontend 等）
- 修复设置窗口左侧导航栏无法滚动的问题：导航菜单项较多时现在可以正常滚动到末尾