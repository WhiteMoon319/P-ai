# P-ai (PAI) — Android 移植版

[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE)
[![Tauri 2](https://img.shields.io/badge/Tauri-2-24C8D8?logo=tauri)](https://tauri.app)
[![Vue 3](https://img.shields.io/badge/Vue-3-4FC08D?logo=vue.js)](https://vuejs.org)
[![Rust](https://img.shields.io/badge/Rust-000000?logo=rust)](https://www.rust-lang.org)
[![Android](https://img.shields.io/badge/Android-3DDC84?logo=android&logoColor=white)](https://developer.android.com)

> 本仓库是 **PAI 的 Android 移植版**，基于桌面版 P-AI 二次开发，面向移动端重构。
> 桌面版原仓库（持续更新、功能最全）：**[kawayiYokami/P-ai](https://github.com/kawayiYokami/P-ai)**

**Languages / 语言**
[简体中文](README.md) | [English](README.en.md)

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
- **移动端细节**：内嵌 WebView 单窗口架构、设置页可滚动、录音权限适配、安全区适配

### 构建与开发

```bash
# 前端依赖
pnpm install

# 类型检查
pnpm typecheck
cd src-tauri && cargo check --target aarch64-linux-android

# 构建 Android APK（aarch64）
pnpm tauri android build --apk --target aarch64 --debug   # debug 包
pnpm tauri android build --apk --target aarch64           # release 包
```

CI 构建（GitHub Actions）：

- **Android Build (Debug)**：`main`/`dev` 分支推送或手动触发，构建 debug APK，
  版本号由 git 派生（`versionCode` = 提交总数，`versionName` = `git describe` 派生），
  上传签名后的 `P-ai-<分支名>-aarch64.apk` 到 artifact
- **Android Release**：推送 `v*` tag 或手动触发，release 构建 + secrets 签名
  （`KEYSTORE_BASE64` / `KEYSTORE_PASSWORD` / `KEY_ALIAS` / `KEY_PASSWORD`），
  发布签名 APK 到 GitHub Release

### 致谢

本项目是 **[kawayiYokami/P-ai](https://github.com/kawayiYokami/P-ai)**（桌面版 P-AI）的 Android 移植，
感谢原作者与上游社区。技术栈依赖：Tauri 2 · Vue 3 · DaisyUI · Tailwind CSS · tokio · reqwest ·
rusqlite · tantivy · proot（Termux）等优秀开源项目。

## License

This project is licensed under [GNU General Public License v3.0](LICENSE).
