# P-ai Android 移植开发指南

> 分支：`android`（仓库 https://github.com/WhiteMoon319/P-ai ）
> 本文档面向后续继续开发的开发者 / AI Agent，记录移植现状、架构决策、已踩的坑与下一步工作。

## 1. 项目概述

P-ai 是基于 Tauri 2 的桌面 AI 助手应用（Rust 后端 + Vue 3 前端，多窗口架构）。本分支将其移植到 Android 平台：

- **UI 策略**：不移植桌面多窗口前端（chat.html 等），而是复用项目自带的**移动端访问页面 `sidebar.html`**（原本用于手机浏览器远程访问桌面端 / VSCode 侧边栏）。
- **通信方式**：sidebar 页面完全通过 **WebSocket**（`ws://127.0.0.1:8429/chat`）与 Rust 后端通信，不依赖 Tauri IPC 多窗口体系，天然适配移动端。
- **构建方式**：全自动 GitHub Actions 构建（本地不需要 Android 开发环境），产物为 debug 签名的 aarch64 APK。

## 2. 技术架构

### 2.1 技术栈

| 层 | 技术 |
|---|---|
| 前端 | Vue 3 + TypeScript + Vite（多 HTML 入口：index/chat/archives/quick-setup/file-reader/runtime-logs/sidebar/settings） |
| 后端 | Rust + Tauri 2（crate-type: staticlib/cdylib/rlib，`#[cfg_attr(mobile, tauri::mobile_entry_point)]`） |
| 存储 | rusqlite + tantivy + TOML 配置 |
| Android 通信链路 | WebView 加载 `sidebar.html` → WebSocket 连本机 web access 服务（默认端口 8429）→ 后端 |

### 2.2 Android 端运行时链路

```
Tauri Android WebView (origin: http://tauri.localhost)
  └─ 加载打包资产 sidebar.html?chatUrl=ws://127.0.0.1:8429/chat
       └─ WebSocket 连接 /chat 桥（bridge_server.rs）
            ├─ loopback 连接自动免密（authenticated = peer_is_local）
            └─ 后端 web access 服务由 run_deferred_setup 延迟启动（不依赖前端就绪）
```

关键事实（均已在代码中验证）：
- web access 服务默认开启（`default_web_access_enabled() = true`），默认端口 `8429`（`types_config.rs`）。
- WS 桥对回环地址连接**自动免密**（`bridge_server.rs`：`authenticated = ide_context_peer_is_local(&peer_addr)`）。
- 前端 `loadDiscovery()`（sidebar App.vue）支持从 URL query 读取 `chatUrl`，无需注入 `__PAI_SIDEBAR_BRIDGE__`。

### 2.3 CI/CD（.github/workflows/android-build.yml）

- 触发：push 到 `main`/`android` 分支或手动 workflow_dispatch。
- 环境：ubuntu-latest + pnpm + Node 24 + JDK 17(temurin) + rust stable（target `aarch64-linux-android`）+ runner 预装 NDK。
- 关键步骤：Export Android NDK path（把 `$ANDROID_NDK_LATEST_HOME` 写入 `$GITHUB_ENV` 的 `NDK_HOME`/`ANDROID_NDK_HOME`/`ANDROID_NDK_ROOT`，并把 llvm 工具链加入 PATH）→ `pnpm tauri android init --ci` → `pnpm tauri android build --apk --target aarch64` → keytool 生成 debug keystore + apksigner 签名 → 上传 artifact `p-ai-android-apk`。
- 注意：`${{ env.XXX }}` 只能引用 workflow env 块，**runner 的 OS 环境变量必须先用 shell 步骤导出到 `$GITHUB_ENV`**。

## 3. 关键配置文件与修改点

| 文件 | 说明 |
|---|---|
| `src-tauri/tauri.android.conf.json` | Android 平台覆盖配置：单窗口 `chat`，url 为 `sidebar.html?chatUrl=ws://127.0.0.1:8429/chat`，`visible: true` |
| `src-tauri/Cargo.toml` | Android target 下 `openssl-sys = { version = "0.9", features = ["vendored"] }`（dingtalk-stream → tokio-tungstenite(native-tls) 依赖链无法用 pkg-config 交叉编译） |
| `src-tauri/capabilities/default.json` | 跨平台权限（core/dialog/notification） |
| `src-tauri/capabilities/desktop-only.json` | 桌面专属权限（global-shortcut/updater），带 `"platforms": ["windows","macOS","linux"]`；**Android 构建时 tauri-build 会校验 capabilities，桌面插件权限必须平台隔离** |
| `src-tauri/src/features/system/commands/ide_context/bridge_access.rs` | `ide_context_ws_origin_allowed` 放行 `tauri://` 前缀与 `tauri.localhost` host（Android WebView 的 Origin，否则 WS 握手 403） |
| `src-tauri/src/lib.rs` | ① Android 下补齐被 stub 文件丢失的 crate 根 use 块；② setup 中 Android 跳过桌面 `show_window` 启动逻辑（静态配置已声明可见窗口） |
| `src-tauri/src/features/system/windowing.rs` | mobile 缺失 API（unminimize/maximize/unmaximize/start_dragging/cursor_position/decorations/shadow）已逐点 cfg 门控 / builder 链拆分 / 整函数 stub |
| `src/features/sidebar/App.vue` | `bootstrap()` 对回环 chatUrl 连接失败自动重试（每 2s，最多 30 次），覆盖后端服务延迟启动窗口期 |

## 4. 已完成的工作

- [x] GitHub Actions 全自动 APK 构建流水线（7 轮 CI 迭代打通，含 NDK 导出、vendored OpenSSL、capabilities 平台拆分、41 处 Android 编译错误修复）
- [x] Android 单窗口方案切换为内置 sidebar.html 移动页面
- [x] WS 桥 Origin 放行 tauri.localhost + loopback 免密链路确认
- [x] sidebar 前端本机场景自动重连
- [x] APK 产出（debug 签名，aarch64，约 68MB）

## 5. 当前最重要的问题：启动黑屏根因分析（P0）

**现象**：APK 安装后打开黑屏。

**已排除**：并非 quick-setup 配置指引缺失——`read_config()`（storage_and_stt.rs）在配置文件不存在时直接返回 `AppConfig::default()`，不会报错；且 Android 方案已不依赖 quick-setup 窗口。

**真正根因（已定位，高置信）**：`AppState::new()`（`src-tauri/src/features/core/domain/runtime_state.rs`）通过 `directories::ProjectDirs::from("ai", "easycall", "p-ai")` 解析配置/数据目录：

```rust
fn resolve_standard_config_dirs() -> Result<(PathBuf, PathBuf), String> {
    let legacy_project_dirs = ProjectDirs::from("ai", "easycall", "easy-call-ai")...;
    let next_project_dirs = ProjectDirs::from("ai", "easycall", "p-ai")...;
    ...
}
```

`directories` crate 在 Android 上依赖 `$HOME`/XDG 环境变量，而 Android 应用进程中 `$HOME` 通常未设置或指向 `/`（只读）。结果两种可能：
1. `ProjectDirs::from` 返回 `None` → `AppState::new()` 返回 `Err`；
2. 解析出 `/.config/...` 之类不可写路径 → `create_dir_all` 权限失败 → `Err`。

而 `lib.rs` 的 `run()` 中（约 869 行）：

```rust
let state = match AppState::new() {
    Ok(state) => state,
    Err(err) => {
        runtime_log_error(...);
        return;   // ← Tauri Builder 根本不启动，WebView 永不加载 → 黑屏
    }
};
```

**修复方案（给下一位开发者/Agent 的具体步骤）**：

1. `runtime_state.rs`：新增 `AppState::new_with_root(app_root: PathBuf) -> Result<Self, String>`，跳过 portable 检测与 ProjectDirs 解析，直接以传入目录为 `app_root`（`config_dir = app_root.join("config")`），其余初始化逻辑与 `new()` 共用（建议把 `new()` 尾部的公共初始化提取成私有函数）。
2. `lib.rs run()`：
   - 桌面：保持现状（顶部 `AppState::new()` + `.manage(state)`）。
   - Android（`#[cfg(target_os = "android")]`）：**不在顶部构造 state**，改在 `.setup(|app| { ... })` 最开头：
     ```rust
     let app_root = app.path().app_data_dir().map_err(|e| ...)?;  // /data/data/ai.easycall.app/files 系
     let state = AppState::new_with_root(app_root)?;
     app.manage(state);
     ```
   - 注意：`app.manage(state)` 必须在 setup 中第一次 `app_handle.state::<AppState>()` 之前执行；`init_last_panic_snapshot_slot` 等依赖 state 的 run() 顶部调用也需要一并 cfg 处理（Android 挪进 setup）。
3. 顺带检查 run() 顶部其他在 Android 上无意义/可能失败的调用（如 `cleanup_portable_update_temp_artifacts_for_current_runtime`），失败仅记日志的可保留。
4. 验证方式见 §7.2。

## 6. 开发指导原则

1. **所有 Android 特定修改用 `#[cfg(target_os = "android")]` / `#[cfg(not(target_os = "android"))]` 条件编译**，绝不破坏桌面路径；桌面版行为必须保持逐字节等价。
2. **移动端没有的窗口 API**（unminimize/maximize/unmaximize/start_dragging/cursor_position、builder 的 decorations/shadow）：
   - 单条语句 → 语句级 cfg；
   - builder 链 → 拆成 `let builder = ...;` + `#[cfg(not(target_os = "android"))] let builder = builder.decorations(false)...;`；
   - 整个函数只对桌面有意义 → 门控原函数 + Android stub 返回中性值。
3. **被 cfg stub 替换的源文件**，其头部 `use` 是 crate 根级的，Android 下需在 `lib.rs` 补齐（已有 `#[cfg(target_os = "android")]` use 块，往里加）。
4. **capabilities**：桌面插件权限只能放 `desktop-only.json`（带 platforms 字段），否则 Android 构建期报 `Permission ... not found`。
5. **文件修改流程**（AI Agent 在本机工作区受限时）：先复制到工作区 staging 目录编辑，再 Copy-Item 回 `D:\program\p-ai_for_android`，git 提交推送 `origin android`，由 CI 验证编译。
6. 不在本地搭建 Android 构建环境；**一切编译验证以 CI 为准**。

## 7. 给 AI Agent 的续作指引

### 7.1 优先级排序

| 优先级 | 任务 | 说明 |
|---|---|---|
| **P0** | 修复 AppState 数据目录初始化（§5） | 黑屏根因，改完必须真机复测 |
| **P1** | 真机验证 sidebar 全链路 | WS 连接、免密、会话收发；预期首屏 1~2 秒"连接中" |
| **P2** | 移动端体验优化 | sidebar 本为侧边栏设计，需检查小屏布局、软键盘遮挡、安全区 |
| **P3** | 正式签名 | CI 中用 secrets 注入 release keystore 替换 debug 签名 |
| **P4** | 内置 ARM Linux 工作区 | proot + Alpine rootfs 方案（参考 rikkahub），配合 extractNativeLibs、前台服务保活 |

### 7.2 调试方法

- **看 Rust 侧启动日志**：`adb logcat | grep -iE "RustStdoutStderr|pai|tauri"`。注意 `runtime_log_*` 可能写文件（数据目录不可用时同样失败），排查启动问题时建议临时在 `run()` 入口加 `eprintln!`（会进 logcat）。
- **看前端**：debug 构建的 WebView 可用 chrome://inspect 远程调试（需 `WebView.setWebContentsDebuggingEnabled`，Tauri debug 构建默认开启）。
- **验证 web access 服务是否起来**：`adb shell "curl -s http://127.0.0.1:8429/ | head -c 200"`（需设备有 curl，或 `adb forward tcp:8429 tcp:8429` 后本机访问）。
- **CI 日志拉取**：GitHub API `/actions/runs/{id}/jobs` + `/actions/jobs/{id}/logs`（需 Bearer token）；日志含 ANSI 色码，正则匹配前先 `replace(/\x1b\[[0-9;]*m/g,'')`。

### 7.3 改动检查清单（每次提交前）

- [ ] 桌面路径无行为变化（所有新逻辑都在 cfg 门控内）
- [ ] 新增/修改的 Rust 代码不引用 mobile 上不存在的窗口 API
- [ ] capabilities 未把桌面插件权限泄漏进跨平台文件
- [ ] 推送后确认 CI（android-build.yml）全绿，Artifacts 产出 APK
