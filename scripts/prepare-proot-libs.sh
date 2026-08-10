#!/usr/bin/env bash
# 本地（Windows Git Bash / Linux / macOS）构建前准备 proot native 库：
# 从 Termux 官方仓库下载 proot/libtalloc/libandroid-shmem 的 aarch64 deb，
# 解析 ar 归档提取 data.tar.xz，解压后按 Android jniLibs 约定改名放入
# src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a/，并用 patchelf
# 修补 SONAME / NEEDED / rpath（$ORIGIN）使 bionic linker 能解析。
# 与 .github/workflows/android-build.yml 的 "Download proot native libs" 步骤等效。
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
JNI_DIR="$ROOT_DIR/src-tauri/gen/android/app/src/main/jniLibs/arm64-v8a"
TMP_DIR="$ROOT_DIR/.pai-tmp/proot-prep"
mkdir -p "$JNI_DIR" "$TMP_DIR"
cd "$TMP_DIR"

PROOT_URL="https://packages.termux.dev/apt/termux-main/pool/main/p/proot/proot_5.1.107.89_aarch64.deb"
TALLOC_URL="https://packages.termux.dev/apt/termux-main/pool/main/libt/libtalloc/libtalloc_2.4.3_aarch64.deb"
SHMEM_URL="https://packages.termux.dev/apt/termux-main/pool/main/liba/libandroid-shmem/libandroid-shmem_0.7_aarch64.deb"

# ---- 从 ar 归档提取指定成员到 stdout ----
# ar 成员头固定 60 字节：name(16) mtime(12) uid(6) gid(6) mode(8) size(10) magic(2)
extract_ar_member() {
  local archive="$1" member="$2"
  local offset=8  # 跳过全局头 !<arch>\n
  while true; do
    local header
    header=$(dd if="$archive" bs=1 skip="$offset" count=60 2>/dev/null)
    [[ -n "$header" ]] || { echo "ar member not found: $member" >&2; return 1; }
    local name="${header:0:16}"
    name="${name%% *}"
    local size_str="${header:48:10}"
    size_str="${size_str// /}"
    local size=$((10#$size_str))
    # 成员名以 / 结尾（GNU ar 风格）
    local clean_name="${name%/}"
    local data_offset=$((offset + 60))
    if [[ "$clean_name" == "$member" ]]; then
      dd if="$archive" bs=1 skip="$data_offset" count="$size" 2>/dev/null
      return 0
    fi
    offset=$((data_offset + size))
    offset=$(( (offset + 1) / 2 * 2 ))  # 偶数对齐
  done
}

echo "==> 下载 Termux deb 包..."
for url in "$PROOT_URL" "$TALLOC_URL" "$SHMEM_URL"; do
  name="$(basename "$url")"
  curl -fsSL --connect-timeout 30 --retry 3 "$url" -o "$name"
done

echo "==> 解析 ar 归档提取 data.tar.xz..."
for deb in proot_*.deb libtalloc_*.deb libandroid-shmem_*.deb; do
  stem="${deb%.deb}"
  mkdir -p "$stem"
  extract_ar_member "$deb" "data.tar.xz" > "$stem/data.tar.xz" 2>/dev/null || true
  # symlink 顺序问题可能导致部分文件失败，但关键二进制已提取；忽略次要错误
  tar -xf "$stem/data.tar.xz" -C "$stem" 2>/dev/null || true
done

PREFIX="data/data/com.termux/files"

echo "==> 复制 proot 二进制..."
cp proot_*/$PREFIX/usr/bin/proot "$JNI_DIR/libproot_exec.so"
chmod +x "$JNI_DIR/libproot_exec.so"

LOADER_SRC="proot_*/$PREFIX/usr/libexec/proot/loader"
if ! ls $LOADER_SRC >/dev/null 2>&1; then
  LOADER_SRC="proot_*/$PREFIX/usr/lib/libproot_loader.so"
fi
if ls $LOADER_SRC >/dev/null 2>&1; then
  cp $LOADER_SRC "$JNI_DIR/libproot_loader.so"
  chmod 0644 "$JNI_DIR/libproot_loader.so"
else
  echo "ERROR: libproot_loader.so not found in proot deb" >&2
  exit 1
fi

echo "==> 复制依赖库并改名（libtalloc.so.2.4.3 -> libtalloc.so）..."
TALLOC_SRC=$(ls libtalloc_*/data/data/com.termux/files/usr/lib/libtalloc.so.2* 2>/dev/null | head -1)
if [ -z "$TALLOC_SRC" ]; then
  echo "ERROR: libtalloc.so.2* not found in libtalloc deb" >&2
  exit 1
fi
cp "$TALLOC_SRC" "$JNI_DIR/libtalloc.so"
chmod 0644 "$JNI_DIR/libtalloc.so"
cp libandroid-shmem_*/data/data/com.termux/files/usr/lib/libandroid-shmem.so "$JNI_DIR/libandroid-shmem.so"
chmod 0644 "$JNI_DIR/libandroid-shmem.so"

# ---- ELF 修补（patchelf 或等效 python 实现） ----
PATCH_ELF="$ROOT_DIR/scripts/patch-elf.py"
if command -v patchelf >/dev/null 2>&1; then
  echo "==> patchelf 修补 SONAME / NEEDED / rpath..."
  patchelf --set-soname libtalloc.so "$JNI_DIR/libtalloc.so" || true
  if patchelf --print-needed "$JNI_DIR/libproot_exec.so" | grep -qx 'libtalloc.so.2'; then
    patchelf --replace-needed libtalloc.so.2 libtalloc.so "$JNI_DIR/libproot_exec.so"
  fi
  patchelf --set-rpath '$ORIGIN' "$JNI_DIR/libproot_exec.so"
  echo "proot rpath: $(patchelf --print-rpath "$JNI_DIR/libproot_exec.so")"
  echo "proot deps: $(patchelf --print-needed "$JNI_DIR/libproot_exec.so")"
else
  echo "==> 使用 python 等效修补（patch-elf.py）..."
  python "$PATCH_ELF" "$JNI_DIR/libtalloc.so" soname "libtalloc.so"
  python "$PATCH_ELF" "$JNI_DIR/libproot_exec.so" needed "libtalloc.so.2" "libtalloc.so"
  python "$PATCH_ELF" "$JNI_DIR/libproot_exec.so" rpath '$ORIGIN'
fi

echo "==> 完成。jniLibs:"
ls -lh "$JNI_DIR"
rm -rf "$TMP_DIR"
