---
name: device-control
description: 当需要冻结/解冻/卸载/安装应用、删除受限文件、触控屏幕、执行 Android 系统命令（pm/cmd/input/toybox 等）或检查提权状态时，必须立刻阅读我。
---

# Device Control

## 核心规则

- 设备控制优先走 `device_control.*` 工具；`shell_exec` 终端里输入 `pm`/`cmd`/`input`/`am`/`dumpsys`/`settings`/`service`/`getprop`/`screencap`/`toybox`/`wm`/`netd`/`appops`/`content` 等首词白名单命令时**自动路由到 Android 域**（Shizuku 首选 / root 兜底提权执行），无需手动拼接工具。
- 歧义命令（`ls`/`rm`/`cat`/`cp` 等 Linux 与 Android 域都有）默认落 Linux 域（proot），需要进 Android 域时必须加 `sys:` 前缀显式覆盖（如 `sys:rm -f /data/local/tmp/x`）；未命中白名单的命令一律落 Linux 域，不会静默提权。
- 终端路由到 Android 域的命令禁止含 shell 元字符（`;`/`|`/`&`/`$`/反引号/`()` 等），含元字符会被拒绝并提示拆分或改用工具，防注入绕过。
- 先查 `device_control_status` 确认提权状态，再执行任何操作。
- 冻结/卸载/安装/删除文件是危险操作，必须带 `confirm: true`（用户已二次确认）才执行。
- 只使用工具白名单，不拼接自由 shell；包名只允许 `[a-zA-Z0-9._]`。
- 删除文件路径必须落在 Android 工作区沙盒内，拒绝系统路径与 `..`。
- 权限不足时如实报错并引导用户激活 Shizuku（ADB 或 root），不假装成功。

## 提权状态

- `device_control_status` 返回 `privilegeState`：
  - `disabled`：无提权通道，引导安装/激活 Shizuku 或开启 root。
  - `shizuku_pending`：已装未授权，调用 `device_control_request_privilege` 触发授权弹窗。
  - `shizuku_ready` / `root_ready`：可直接操作。

## 常用操作

```text
device_control_status
device_control_list_packages(thirdPartyOnly: true)
device_control_freeze(package: "com.example", confirm: true)
device_control_unfreeze(package: "com.example")
device_control_uninstall(package: "com.example", confirm: true)
device_control_install(apkPath: "/工作区/x.apk", confirm: true)
device_control_delete_file(path: "tmp/old.bin", confirm: true)
device_control_tap(x: 540, y: 1200)
device_control_swipe(x1: 540, y1: 2000, x2: 540, y2: 400, durationMs: 300)
device_control_key_event(keycode: 4)
device_control_screenshot(fileName: "now.png")
```

## 触控与截屏

- 先 `device_control_screenshot` 截屏，看截图定坐标，再 `tap` / `swipe` / `key_event`。
- 坐标为屏幕物理像素；keycode 参考 Android KeyEvent（4=返回、3=主页、26=电源）。
- 触控为**注入式**（Shizuku UserService 进程内 `injectInputEvent`，MAA-Meow 语义），root 环境下降级 `input` 命令。
- 中文文本输入暂不支持直接注入（v1.1），需要输入中文时提示用户手动操作或走无障碍方案。

## 禁做清单

- 不执行任意自由 shell，只用 `device_control.*` 工具（或终端里白名单首词命令的自动路由）。
- 不删除工作区沙盒外的文件（系统文件、其他应用数据）。
- 不修改系统设置、不做跨用户操作、不查看其他应用私有数据。
- 不在 proot Linux 终端里执行 Android 系统命令（会被自动路由到 Android 域提权执行）。
