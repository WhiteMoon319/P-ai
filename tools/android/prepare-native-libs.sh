#!/usr/bin/env bash
# 准备 Android native libs：把 Rust .so 与 proot 依赖复制到 apps/android jniLibs。
# 用法: tools/android/prepare-native-libs.sh <cargo-target-dir> [release|debug]
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
JNI_DIR="$ROOT/apps/android/app/src/main/jniLibs/arm64-v8a"
CARGO_DIR="${1:?usage: prepare-native-libs.sh <cargo-target-dir> [release|debug]}"
PROFILE="${2:-debug}"

mkdir -p "$JNI_DIR"

case "$PROFILE" in
  release) SOURCE="$CARGO_DIR/aarch64-linux-android/release/libeasy_call_ai_lib.so" ;;
  debug)   SOURCE="$CARGO_DIR/aarch64-linux-android/debug/libeasy_call_ai_lib.so" ;;
  *) echo "unknown profile: $PROFILE" >&2; exit 1 ;;
esac

if [[ ! -f "$SOURCE" ]]; then
  echo "ERROR: Rust .so 不存在: $SOURCE（请先交叉编译 aarch64-linux-android）" >&2
  exit 1
fi

cp "$SOURCE" "$JNI_DIR/libeasy_call_ai_lib.so"

# proot 依赖若缺失给出明确错误（CI 的 android-release/android-debug workflow 负责下载，
# 本地构建可参考 .github/workflows/android-release.yml 的 Download proot native libs 步骤）。
for lib in libproot_exec.so libproot_loader.so libtalloc.so libandroid-shmem.so; do
  if [[ ! -f "$JNI_DIR/$lib" ]]; then
    echo "WARNING: $JNI_DIR/$lib 缺失（本地可跳过，CI 会下载 proot 依赖）" >&2
  fi
done

echo "prepared native libs in $JNI_DIR:"
ls -lh "$JNI_DIR"
