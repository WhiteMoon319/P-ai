# P-ai (PAI) — Android 移植版

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri)](https://tauri.app)
[![Vue 3](https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust)](https://www.rust-lang.org)
[![Android](https://img.shields.io/badge/Android-3DDC84?logo=android&logoColor=white)](https://developer.android.com)

> 本仓库是 **PAI 的 Android 移植版**，基于桌面版 P-AI 二次开发，面向移动端重构。
> 桌面版原仓库（持续更新、功能最全）：**[kawayiYokami/P-ai](https://github.com/kawayiYokami/P-ai)**

---

> **A self-growing AI work system on Android — agent delegation, long-term memory, tool review, MCP, and sandboxed Linux workspace, all on your phone.**

---

PAI 是一个持续进化的 AI 工作系统，不只是聊天客户端。它围绕对话、任务、记忆、部门、工具、审查与远程消息组织成一套完整系统。Android 版将桌面端核心能力搬进移动端：

- **Rust 异步后端**（tokio + Tauri 2 WebView 桥接），响应快、本地运行
- **Vue 3 + DaisyUI** 移动端 UI，适配安全区、返回键、录音等交互细节
- **全部数据本地存储**，无中间服务器，API Key 不出设备

### 核心能力

- **对话与任务**：本地会话、远程会话（微信/飞书/钉钉/OneBot）、多会话并行、会话自动压缩归档
- **部门与人格**：多部门、多人格独立配置，各带独立头像与私有记忆；本地会话支持多智能体群聊
- **工具与审查**：内置 Skill 体系、MCP 支持、工具执行审查链，AI 可自主管理 MCP/Skill/人格/部门
- **记忆与上下文**：长对话动态压缩归档，低成本全面记忆系统，越用越懂你
- **Android 沙盒工作区**：内置 proot Linux 运行环境（arm64），支持 Ubuntu Base rootfs 下载/导入，
  在手机上跑 Linux 命令与脚本；`llm-workspace` 直接映射为 Linux `/workspace` 与 `/root/.pai`
- **远程前端模式**：输入电脑 PAI 的地址与端口，手机即成为电脑 PAI 的远程前端，
  实时同步聊天与设置界面；电脑 PAI 回复时手机仍收到通知
- **移动端细节**：内嵌 WebView 单窗口架构、设置页可滚动、录音权限适配、安全区适配、
  系统分享导出沙盒文件、content URI 文件导入

### 下载与安装

APK 发布在 [GitHub Releases](https://github.com/WhiteMoon319/P-ai/releases)：

- 推送 `v*` tag 触发 **Android Release** 构建：release 包 + secrets 签名后发布
- 推送 `main`/`dev` 分支触发 **Android Build (Debug)** 构建：debug APK 上传到 Actions artifact

安装后在应用内配置 LLM API Key 即可开始使用；如需语音输入、记忆检索等能力，
可在「设置 → LLM」补充 STT、Embedding、Rerank 等模型。

### 构建与开发

环境要求：Rust 工具链、Android SDK + NDK（建议 r27+）、Java 17+。

```bash
# Rust 后端交叉编译（aarch64）
cd src-tauri
# 设置 NDK 环境（路径按本机 NDK 调整）：
#   export CC_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/<host>/bin/aarch64-linux-android21-clang"
#   export AR_aarch64_linux_android="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/<host>/bin/llvm-ar"
#   export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
cargo build --target aarch64-linux-android --release
cp target/aarch64-linux-android/release/libeasy_call_ai_lib.so \
  gen/android/app/src/main/jniLibs/arm64-v8a/libeasy_call_ai_lib.so

# 构建 Android APK（aarch64）
cd gen/android
./gradlew :app:assembleDebug    # debug 包
./gradlew :app:assembleRelease  # release 包（R8 压缩）
```

> 说明：本项目 Android 端已彻底剥离 Tauri 运行时（原生 Kotlin/Compose 前端 + Rust JNI 后端），
> 不再使用 `pnpm tauri android` 系列命令。

CI 构建（GitHub Actions）：

- **Android Build (Debug)**：`main`/`dev` 分支推送或手动触发，构建 debug APK，
  版本号由 git 派生（`versionCode` = 提交总数，`versionName` = `git describe` 派生），
  上传签名后的 `P-ai-<分支名>-aarch64.apk` 到 artifact
- **Android Release**：推送 `v*` tag 或手动触发，release 构建 + secrets 签名
  （`KEYSTORE_BASE64` / `KEYSTORE_PASSWORD` / `KEY_ALIAS` / `KEY_PASSWORD`），
  发布签名 APK 到 GitHub Release

### 文档

- [Android 开发指南](docs/android-development-guide.md)：Android 移植分支的构建、调试与发布说明
- [更新日志](docs/changelog/)：按版本维护的 changelog（`CHANGELOG.md` 由脚本生成）
- 桌面版完整文档见上游仓库 [kawayiYokami/P-ai](https://github.com/kawayiYokami/P-ai)

### 致谢

本项目是 **[kawayiYokami/P-ai](https://github.com/kawayiYokami/P-ai)**（桌面版 P-AI）的 Android 移植，
感谢原作者与上游社区。技术栈依赖：Tauri 2 · Vue 3 · DaisyUI · Tailwind CSS · tokio · reqwest ·
rusqlite · tantivy · proot（Termux）等优秀开源项目。

## License

This project is licensed under [GNU General Public License v3.0](LICENSE).
