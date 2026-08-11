# AGENTS.md — P-AI Android-only 单仓库

This file provides guidance to AI coding when working with code in this repository.

## 项目概述

P-AI 是 **Android-only** 的 AI 工作系统：Kotlin/Compose 客户端 + Rust 进程内后端（JNI 通信）。
仓库已彻底移除 Tauri / WebView / Vue 桌面前端 / 桌面打包 / VS Code 扩展，**禁止恢复**。

- 包名：`com.whitemoon319.pai`
- Android 工程：`apps/android/`（Gradle）
- Rust 后端（迁移中）：旧 `src-tauri/`（include!() 单入口）→ 目标 `crates/pai-*`
- 唯一 RPC 契约：`contracts/native-rpc/methods.json` + `events.json`

## 构建与开发命令

```bash
# Rust .so（旧 src-tauri 单 crate，Android 目标；Windows 需 NDK env）
cd src-tauri
export NDK="$HOME/AppData/Local/Android/Sdk/ndk/<version>"
export CC_aarch64_linux_android="$NDK/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android21-clang.cmd"
export AR_aarch64_linux_android="$NDK/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
export RUSTC_WRAPPER="" && export CARGO_BUILD_RUSTC_WRAPPER=""
export CARGO_PROFILE_DEV_CODEGEN_UNITS=1 && export CARGO_PROFILE_DEV_INCREMENTAL=false
cargo check --target aarch64-linux-android      # 快速校验
cargo build --target aarch64-linux-android      # 产出 .so
cp target/aarch64-linux-android/debug/libeasy_call_ai_lib.so ../apps/android/app/src/main/jniLibs/arm64-v8a/

# 契约测试（workspace crates）
cd .. && cargo test -p pai-protocol

# APK
cd apps/android
bash gradlew :app:assembleDebug --no-daemon
bash gradlew :app:assembleRelease --no-daemon

# 校验
python ../tools/android/verify-apk.py app/build/outputs/apk/debug/app-debug.apk debug
python ../tools/android/verify-apk.py app/build/outputs/apk/release/app-release-unsigned.apk release
```

### Windows 交叉编译注意

- rustc 链接大 crate 可能 `0xc0000409`：必须 codegen-units=1 + 关增量。
- 默认 `target/` 被占用时用 `CARGO_TARGET_DIR=target-tests`（仓库内，勿新建其他 target-*）。
- 禁止 `pnpm tauri android init/build`；APK 走本地 cargo 交叉编译 + gradle。

## 架构

### 通信

```
Kotlin → NativeRpcClient.call(method, params) → PaiNative.call(JSON) → Rust native_bridge → 响应
Rust → push_native_delta_event → NATIVE_DELTA_QUEUE → Kotlin NativeEventPump.pollEvents → notifications
```

- 事件顺序：单一队列 + 顺序 tryEmit；**禁止**并发 launch emit / collectLatest（流式 delta 会乱序）。
- 请求超时：普通 15s；长任务（migration/workspace/rootfs）callLong 600s 或任务状态机。
- 方法名唯一来源 `contracts/native-rpc/methods.json`；Rust dispatch 与 Kotlin service 必须一致。

### Kotlin 分层

- `bridge/`：NativeRpcClient / NativeEventPump / NativeError / PaiNative(JNI)
- `service/`：ChatService 等 RPC 门面
- `model/`：RpcModels / ChatModels / SettingsModels / WorkspaceModels
- `viewmodel/`：AppViewModel（拆分 Chat/Settings/Workspace/RemoteIm 进行中）
- `ui/`：Compose（app/chat/settings/workspace/remoteim/common）
- `platform/`：PaiForegroundService / LiveUpdate / AudioRecorder / FileProvider

### Rust 分层（迁移目标）

- `pai-protocol`：RPC 类型 + 契约（零平台依赖）
- `pai-backend`：平台无关业务（不依赖 tauri/jni/Android/NativeAppHandle）
- `pai-android-bridge`：JNI / runtime / dispatch / event queue / 任务句柄
- `pai-android-platform`：workspace / rootfs / proot / TLS / 沙盒

## 数据持久化

- 数据目录：`<dataDir>`（= `/data/user/0/com.whitemoon319.pai`）
- `config/app_config.toml`、`config/agents.json`
- `memory/memory_store.db`（SQLite + FTS5）
- `state/runtime_state.json`（含 messageStoreMigrationVersion）
- `chat/`、`llm-workspace/`、`runtime/android-workspace/`

## 开发约定

- **禁止恢复 Tauri/WebView/Vue/桌面**：不引入 tauri crate、不重建 WebView、不写桌面打包。
- 禁止 `unwrap()`/`expect()`（测试除外），统一 `Result` 传递可读错误。
- 禁止返回假成功/假"暂不支持"；失败必须走 JSON-RPC error 结构。
- 配置保存用 `patch_config` 局部更新（全量 save_config 会覆盖丢配置）。
- 消息读取默认禁止整读 `Conversation.messages`；用轻量路径（metadata/recent/block）。
- 路径访问必须经 Android 沙盒与 symlink 防护；rootfs 解压拒绝绝对路径/`..`/非法 link。
- 新增 RPC 方法/事件：同步更新 `contracts/native-rpc/` + Rust dispatch + Kotlin service + 契约测试。
- 文件 < 1500 行，函数 < 100 行；超过需评审。
- 提交信息用 Conventional Commits（中文描述）；改动用户可见行为时补"未发布" changelog。

## 验证与测试

- 改动后跑最小必要检查：`cargo check --target aarch64-linux-android` + 相关单测。
- 提交前必须通过：`git diff --check`、受影响测试、APK 构建（如改动 Kotlin）。
- 禁止用 `|| true` 吞构建失败；CI workflow 失败必须失败。
- APK 必须含 5 个 native libs（libeasy_call_ai_lib / libproot_exec / libproot_loader /
  libtalloc / libandroid-shmem）；release 必须 usesCleartextTraffic=false。
- 不默认跑 `cargo fmt` 全仓格式化；不反复跑无关全量测试。

## 保活 / live update

- `app.keepAlive {active}` → `PaiForegroundService.start/stop`（API 34+ specialUse）。
- 通知不依赖 POST_NOTIFICATIONS（无权限时前台服务仍保活，通知不显示）。
- Android 13+ 运行时请求 POST_NOTIFICATIONS。

## 迁移状态

- ✅ apps/android、pai-protocol、contracts、tools、CI、docs
- 🚧 Rust include!() → module → crates 拆分（阶段 3-6）
- 🗑️ src-tauri/ 与桌面内容：验证完成后删除（阶段 12）
