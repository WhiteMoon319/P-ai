# Android 调试

## 设备连接

```bash
adb devices                      # 确认设备（如 9930ae4d）
adb -s <serial> install -r apps/android/app/build/outputs/apk/debug/app-debug.apk
adb -s <serial> shell am start -n com.whitemoon319.pai/.MainActivity
```

## 日志

- Kotlin 诊断：`adb logcat -d | grep -E "PaiNotify|PaiNative"`
- Rust 日志：`runtime_log_*` 写入数据目录 runtime 日志；Android 上部分走 logcat
  `RustStdoutStderr`（`adb logcat -d | grep -i rust`）。
- 崩溃：`adb logcat -d | grep -E "AndroidRuntime|FATAL"`

## 数据目录

`adb shell run-as com.whitemoon319.pai ls /data/user/0/com.whitemoon319.pai/`

- `config/`：app_config.toml / agents.json
- `memory/`：memory_store.db（含 FTS）
- `state/`：runtime_state.json（含 messageStoreMigrationVersion）
- `llm-workspace/`、`runtime/`：工作区与 rootfs

## 常见问题

- 流式乱码/词序颠倒：确认 `NativeEventPump.dispatchPolledEvents` 顺序 tryEmit、
  `AppViewModel` notificationJob 用 collect 而非 collectLatest；禁止并发 launch emit。
- 发送状态不复位：确认 `chat.roundFinished` 到达（emit_round_completed_event Android 分支
  push native 队列）。
- 保活不生效：确认 `app.keepAlive` 事件 + `PaiForegroundService` Manifest 声明与权限。
- 后台被杀：检查厂商后台限制；前台服务为 ongoing notification。
