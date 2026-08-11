# Android 发布

## 版本来源

- 构建期注入 `PAI_ANDROID_APP_VERSION`（`scripts/patch-android-version.sh` 从 git describe 派生），
  与 APK versionName / Rust `android_current_app_version()` / 关于页 / 更新检查同源。
- `crates/pai-protocol` workspace 版本与 Cargo.toml 同步。
- 版本比较（v 前缀/预发布/大小）见 `crates/pai-protocol` 的 version_compare 逻辑。

## 发布流程

```text
tag vX.Y.Z
  ↓ Rust version（构建期注入）
  ↓ Android versionCode/versionName
  ↓ arm64 Rust .so（cargo build --release --target aarch64-linux-android）
  ↓ proot native libs（Termux 包 + patchelf）
  ↓ signed APK（assembleRelease + apksigner）
  ↓ GitHub Release
```

`android-release.yml` 自动执行（tag 触发）：构建 .so → 下载 proot libs → assembleRelease
→ 签名 → 校验（native libs + release cleartext=false）→ GitHub Release。

## 安全约束

- release APK 必须 `android:usesCleartextTraffic="false"`（build.gradle.kts release 块显式声明，
  `tools/android/verify-apk.py` 校验）。
- 5 个必需 native libs：libeasy_call_ai_lib / libproot_exec / libproot_loader / libtalloc /
  libandroid-shmem（`tools/android/verify-native-libs.py` 校验）。
- workflow 失败必须失败；禁止 `|| true` 吞构建错误。

## 更新检查

`check_github_update` 查询 `WhiteMoon319/P-ai` 的 latest Release，比较版本（正确处理 v 前缀/
预发布/大小），has_update 为 true 时 UI 引导打开对应 APK Release。
