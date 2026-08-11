# 仓库结构

P-AI 是 **Android-only** 单仓库：Kotlin/Compose 客户端 + Rust 进程内后端（JNI 通信）。

## 顶层

```
P-ai/
├─ apps/android/          # Android 宿主与 UI（Gradle 工程）
│  ├─ app/src/main/java/com/whitemoon319/pai/
│  │  ├─ MainActivity.kt / PaiApplication.kt
│  │  ├─ bridge/          # NativeRpcClient / NativeEventPump / NativeError / PaiNative(JNI)
│  │  ├─ service/         # ChatService 等 RPC 方法门面
│  │  ├─ model/           # RpcModels / ChatModels / SettingsModels / WorkspaceModels
│  │  ├─ viewmodel/       # AppViewModel（Chat/Settings/Workspace/RemoteIm 拆分进行中）
│  │  ├─ ui/              # Compose 页面（app/chat/settings/workspace/remoteim/common）
│  │  └─ platform/        # PaiForegroundService / LiveUpdate / AudioRecorder / FileProvider
│  └─ build.gradle.kts / settings.gradle / gradle.properties
├─ crates/
│  ├─ pai-protocol/       # JSON-RPC 协议类型 + 契约（RPC 请求/响应/事件/方法名）
│  ├─ pai-backend/        # 平台无关业务（聊天/记忆/远程IM/任务/配置/迁移）【迁移中】
│  ├─ pai-android-bridge/ # JNI / runtime / dispatch / event queue / 任务句柄【迁移中】
│  └─ pai-android-platform/# workspace / rootfs / proot / TLS / 沙盒路径【迁移中】
├─ contracts/native-rpc/  # methods.json / events.json（唯一 RPC 契约来源）
├─ third_party/android/   # proot / rootfs manifest（版本与来源）
├─ tools/android/         # prepare-native-libs / verify-apk / verify-manifest / verify-native-libs
├─ docs/
│  ├─ architecture/       # repository-structure / runtime-layers / native-rpc
│  └─ android/            # build / debugging / workspace / release / troubleshooting
└─ .github/workflows/     # rust-check / android-check / android-debug / android-release
```

## 迁移状态

- ✅ `apps/android`：Gradle 工程（含 5 个 native libs），Kotlin model/bridge/service/platform 已按目标结构拆分
- ✅ `crates/pai-protocol`：RPC 类型 + 契约 JSON + 一致性测试
- ✅ `contracts/native-rpc`：methods.json（129 方法）+ events.json
- ✅ `tools/android`：验证脚本（native libs / cleartext / manifest）
- ✅ CI：rust-check / android-check / android-debug / android-release
- 🚧 `crates/pai-backend` / `pai-android-bridge` / `pai-android-platform`：待从 `src-tauri` include!() 单入口拆分（依赖 include→module 转换）
- 🚧 ViewModel/UI 深度分包：AppViewModel 单文件 2004 行，拆 5 个 ViewModel 待后续
- 🗑️ `src-tauri/` / Vue 前端 / Tauri：迁移验证完成后删除（阶段 12）

## 旧工程说明

`src-tauri/` 是迁移前的 Rust 单 crate（include!() 单入口，Android 优先），当前仍是 Android .so 的唯一构建来源。
`apps/android/app/src/main/jniLibs/` 中的 `libeasy_call_ai_lib.so` 由它交叉编译产出。
阶段 3-6（include→module→crates 拆分）完成后，`src-tauri/` 将被删除。
