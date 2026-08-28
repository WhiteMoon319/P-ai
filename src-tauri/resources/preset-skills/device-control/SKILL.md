---
name: device-control
description: 当需要冻结/解冻/卸载/安装应用、删除受限文件、触控屏幕、执行 Android 系统命令（pm/cmd/input/toybox 等）或检查提权状态时，必须立刻阅读我。
---

# Device Control

## 核心规则

- 终端执行域由**显式环境切换**决定，非自动路由：`config "terminal get"` 查当前环境，`config "terminal set android|linux"` 切换。
  - `linux`（默认）：exec 走 proot 沙盒（文件/代码/构建命令）。
  - `android`：exec 走 device_control 提权 shell（pm/cmd/input/toybox/dumpsys 等系统命令）。
  - `sys:` 前缀可单命令强制 Android 域（如 `sys:pm list packages`），不依赖环境切换。
- **能力默认全部关闭**：先查 `device_control {action: "status"}` 只反映提权状态；总开关与分项开关在「设置 → 工具 → 设备控制」开启后，对应操作才可执行。未开启时返回结构化错误，不假装成功。
- Android 域（含 sys: 覆盖）命令禁止含 shell 元字符（`;`/`|`/`&`/`$`/反引号/`()` 等），含元字符会被拒绝并提示拆分或改用工具，防注入绕过。
- 冻结/卸载/安装/删除文件是危险操作，必须带 `confirm: true`（用户已二次确认）才执行；Android 域的卸载/冻结/安装/删除类命令也会触发用户确认。
- 只使用工具白名单，不拼接自由 shell；包名只允许 `[a-zA-Z0-9._]`。
- 删除/安装/截屏路径必须落在设备控制中转区（`/sdcard/Android/data/<pkg>/files/device_control`），拒绝系统路径与 `..`。
- 权限不足时如实报错并引导用户激活 Shizuku（ADB 或 root），不假装成功。

## 提权状态

- `device_control_status` 返回 `privilegeState`：
  - `disabled`：无提权通道，引导安装/激活 Shizuku 或开启 root。
  - `shizuku_pending`：已装未授权，调用 `device_control_request_privilege` 触发授权弹窗。
  - `shizuku_ready` / `root_ready`：可直接操作。

## 常用操作

```text
device_control {action: "status"}
device_control {action: "list_packages", thirdPartyOnly: true}
device_control {action: "freeze", package: "com.example", confirm: true}
device_control {action: "unfreeze", package: "com.example"}
device_control {action: "uninstall", package: "com.example", confirm: true}
device_control {action: "install", path: "/sdcard/Android/data/ai.easycall.app/files/device_control/x.apk", confirm: true}
device_control {action: "delete_file", path: "/sdcard/Android/data/ai.easycall.app/files/device_control/tmp/old.bin", confirm: true}
device_control {action: "tap", x: 540, y: 1200}
device_control {action: "swipe", x1: 540, y1: 2000, x2: 540, y2: 400, durationMs: 300}
device_control {action: "key", keycode: 4}
device_control {action: "screenshot", fileName: "now.png"}
```

## 触控与截屏

- 先 `device_control_screenshot` 截屏，看截图定坐标，再 `tap` / `swipe` / `key_event`。
- 坐标为屏幕物理像素；keycode 参考 Android KeyEvent（4=返回、3=主页、26=电源）。
- 触控为**注入式**（Shizuku UserService 进程内 `injectInputEvent`，MAA-Meow 语义），root 环境下降级 `input` 命令。
- 中文文本输入暂不支持直接注入（v1.1），需要输入中文时提示用户手动操作或走无障碍方案。

## 禁做清单

- 不执行任意自由 shell，只用 `device_control.*` 工具（或在 Android 域/`sys:` 覆盖下执行系统命令）。
- 不删除工作区沙盒外的文件（系统文件、其他应用数据）。
- 不修改系统设置、不做跨用户操作、不查看其他应用私有数据。
- 不在 proot Linux 终端里执行 Android 系统命令（会被自动路由到 Android 域提权执行）。
