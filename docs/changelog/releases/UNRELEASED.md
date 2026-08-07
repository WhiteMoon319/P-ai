# 未发布

## 功能

- Android 日志文件输出到外部存储 `/sdcard/Android/data/<包名>/log/backend.log`，可通过 adb 直接查看，不再写入需 root 权限的内部 data 目录。

## 重构

## 修复

- 修复 Android 启动 ClassNotFoundException：workspace-io 插件的 Android 插件标识符从连字符 `app.tauri.workspace-io` 改为下划线 `app.tauri.workspace_io`（与 Kotlin 源码包名一致），避免启动时反射找不到插件类而闪退。
- 修复 Android 端 STT 语音转写请求失败：`stt_transcribe.rs` 两处裸 `reqwest::Client::builder().build()` 现在注入静态根证书（webpki），避免 HTTPS 证书校验失败导致无法请求语音模型。
- 全量排查并修复 Android 端其余裸 HTTP 客户端：更新检查/更新日志（`updater_android_stub`）、MCP streamable-http 与 SSE 传输、Codex OAuth 与用量接口、远程 IM 的飞书/钉钉/OneBot/个人微信请求、联网工具与模型列表补充路径，统一注入静态根证书。

## 依赖
