# 排障

## 构建

- **cargo check 失败（tauri 相关）**：`src-tauri` 是旧工程，`tauri-build` 仅非 android 生效；
  Android 构建不应触发。若报 tauri 缺失，确认在 `src-tauri` 目录内且未误用根 workspace。
- **assembleDebug 缺 .so**：先跑 `tools/android/prepare-native-libs.sh` 或 CI 的
  proot 下载 + cargo build 步骤。
- **gradle 无 exec 权限**：用 `bash gradlew`（CI 一致）。
- **Windows 链接崩溃 0xc0000409**：codegen-units=1 + 关增量。

## 运行时

- **冷启动黑屏**：看 logcat `AndroidRuntime/FATAL`；检查 PaiNative.init 返回错误
  （MainActivity Toast 展示）。
- **migration 一直转圈**：`runMigrationGate` 依赖连接建立事件；确认
  `messageStore.migration.progress` 事件被消费，失败时 `migrationState=Failed` 显示错误。
- **远程 IM 未启动**：native 初始化后自动拉起；`start_remote_im_services_inner` 日志
  `[远程IM] 启动完成 started=.. failed=..`；单渠道失败不影响整体。
- **保活服务未拉起**：`app.keepAlive active=true` 事件 → `PaiForegroundService.start`；
  检查 Manifest service 声明与 FOREGROUND_SERVICE 权限；Android 14+ specialUse 类型。

## 流式

- **词序颠倒/双份**：确认 `NativeEventPump` 单 poll 循环、顺序 tryEmit；
  `AppViewModel` notificationJob 用 `collect`（非 collectLatest）；`start()` 幂等。
- **卡中间不滚底**：ChatScreen 流式滚动用 scrollToItem + scrollBy（见 PaiApp.kt）。

## 数据

- 会话/记忆存储：`<dataDir>/memory/memory_store.db`（SQLite + FTS5）、`<dataDir>/chat/`。
- 迁移版本：`<dataDir>/state/runtime_state.json` 的 messageStoreMigrationVersion。
