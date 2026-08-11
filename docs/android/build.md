# Android 构建

## 前置

- Android SDK + NDK（含 aarch64-linux-android 工具链）
- Rust stable + `aarch64-linux-android` target
- JDK 17
- proot 依赖（libproot_exec/libproot_loader/libtalloc/libandroid-shmem）与 Rust .so 位于
  `apps/android/app/src/main/jniLibs/arm64-v8a/`

## 步骤

```bash
# 1) 交叉编译 Rust .so（旧 src-tauri 单 crate，Android 目标）
cd src-tauri
export NDK="$HOME/AppData/Local/Android/Sdk/ndk/<version>"
export CC_aarch64_linux_android="$NDK/toolchains/llvm/prebuilt/windows-x86_64/bin/aarch64-linux-android21-clang.cmd"
export AR_aarch64_linux_android="$NDK/toolchains/llvm/prebuilt/windows-x86_64/bin/llvm-ar"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_aarch64_linux_android"
export RUSTC_WRAPPER=""
export CARGO_PROFILE_DEV_CODEGEN_UNITS=1
export CARGO_PROFILE_DEV_INCREMENTAL=false
cargo build --target aarch64-linux-android
cp target/aarch64-linux-android/debug/libeasy_call_ai_lib.so \
  ../apps/android/app/src/main/jniLibs/arm64-v8a/

# 2) 准备 native libs（proot 依赖缺失时提示；CI 负责下载）
bash tools/android/prepare-native-libs.sh src-tauri/target debug

# 3) Gradle 构建
cd apps/android
bash gradlew :app:assembleDebug --no-daemon

# 4) 校验 APK
python ../tools/android/verify-apk.py app/build/outputs/apk/debug/app-debug.apk debug
```

Release：`bash gradlew :app:assembleRelease --no-daemon` + `verify-apk.py ... release`。

## 本地交叉编译注意

- Windows 上 rustc 链接大 crate 可能 `0xc0000409`，用 codegen-units=1 + 关增量绕过。
- `src-tauri` 是 include!() 单入口旧工程；`crates/*` 是迁移目标（阶段 3-6 进行中）。
- 完整 proot 依赖下载步骤见 `.github/workflows/android-debug.yml` 的
  "Download proot native libs"（Termux 官方包 + patchelf 修正 soname/rpath）。

## CI

- `rust-check.yml`：workspace check/test + legacy Android check
- `android-check.yml`：.so 编译 + Kotlin 编译 + RPC 契约测试 + manifest 校验
- `android-debug.yml`：完整 debug APK + native libs 校验
- `android-release.yml`：release APK + 签名 + native libs + cleartext 校验 + GitHub Release
