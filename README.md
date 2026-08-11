# P-AI (Android)

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust)](https://www.rust-lang.org)
[![Kotlin](https://img.shields.io/badge/Kotlin-7F52FF?logo=kotlin&logoColor=white)](https://kotlinlang.org)
[![Android](https://img.shields.io/badge/Android-3DDC84?logo=android&logoColor=white)](https://developer.android.com)

> **A self-growing AI work system on Android — agent delegation, long-term memory, tool review, MCP, and sandboxed Linux workspace, all on your phone.**

本仓库是 **Android-only 单仓库**：Kotlin/Compose 客户端 + Rust 进程内后端（JNI 通信）。
不再包含 Tauri / WebView / Vue 桌面前端 / 桌面打包 / VS Code 扩展。

## 架构

```
Kotlin (Compose UI)
   │ JSON-RPC over JNI（PaiNative.call / pollEvents）
   ▼
Rust (.so: libeasy_call_ai_lib.so)
   ├─ Tokio runtime（自建 8MB 栈）
   ├─ AppState（配置/会话/记忆/运行时状态）
   └─ native_bridge（请求分发 + 事件队列）
```

- `apps/android/`：Android 宿主与 UI（Gradle 工程）
- `crates/pai-protocol/`：JSON-RPC 协议类型 + 契约（唯一协议来源）
- `crates/pai-backend|pai-android-bridge|pai-android-platform/`：Rust 拆分目标（迁移中）
- `contracts/native-rpc/`：methods.json / events.json
- `third_party/android/`：proot / rootfs manifest
- `tools/android/`：构建与校验脚本

## 核心能力

- 对话与任务：本地会话、远程会话（微信/飞书/钉钉/OneBot）、多会话并行、自动压缩归档
- 部门与人格：多部门、多人格独立配置，各带独立头像与私有记忆
- 工具与审查：Skill 体系、MCP、工具执行审查链
- 记忆：长对话动态压缩、低成本记忆系统
- Android 沙盒工作区：内置 proot Linux 运行环境（arm64）、Ubuntu Base rootfs 下载/导入、
  终端、文件管理
- live update 保活通知、GitHub Release 更新检查

## 构建

见 [docs/android/build.md](docs/android/build.md)。简要：

```bash
# Rust .so（旧 src-tauri 单 crate，Android 目标）
cd src-tauri
cargo build --target aarch64-linux-android
cp target/aarch64-linux-android/debug/libeasy_call_ai_lib.so \
  ../apps/android/app/src/main/jniLibs/arm64-v8a/

# APK
cd ../apps/android
bash gradlew :app:assembleDebug --no-daemon
```

## 文档

- 架构：[repository-structure](docs/architecture/repository-structure.md) ·
  [runtime-layers](docs/architecture/runtime-layers.md) · [native-rpc](docs/architecture/native-rpc.md)
- Android：[build](docs/android/build.md) · [debugging](docs/android/debugging.md) ·
  [workspace](docs/android/workspace.md) · [release](docs/android/release.md) ·
  [troubleshooting](docs/android/troubleshooting.md)

## 迁移状态

- ✅ apps/android（含 5 个 native libs）、pai-protocol、RPC 契约、tools、CI
- 🚧 crates 拆分（include!() → module → pai-backend/bridge/platform，阶段 3-6）
- 🗑️ src-tauri/ 与桌面内容：迁移验证完成后删除（阶段 12）

## 致谢

本项目源自 **[kawayiYokami/P-ai](https://github.com/kawayiYokami/P-ai)**（桌面版 P-AI），
感谢原作者与上游社区。技术栈：Kotlin · Compose · Rust · tokio · reqwest · rusqlite · tantivy。
