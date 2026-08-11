#!/usr/bin/env python3
"""验证 APK 内含 5 个必需 native libs（arm64-v8a）。

用法: python tools/android/verify-native-libs.py <apk-path>
"""
import sys
import zipfile

REQUIRED = [
    "lib/arm64-v8a/libeasy_call_ai_lib.so",
    "lib/arm64-v8a/libproot_exec.so",
    "lib/arm64-v8a/libproot_loader.so",
    "lib/arm64-v8a/libtalloc.so",
    "lib/arm64-v8a/libandroid-shmem.so",
]


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: verify-native-libs.py <apk-path>", file=sys.stderr)
        return 1
    apk = sys.argv[1]
    try:
        names = set(zipfile.ZipFile(apk).namelist())
    except Exception as exc:  # noqa: BLE001
        print(f"ERROR: 无法读取 APK {apk}: {exc}", file=sys.stderr)
        return 1

    fail = False
    for lib in REQUIRED:
        if lib in names:
            print(f"OK: {lib}")
        else:
            print(f"ERROR: {lib} 缺失", file=sys.stderr)
            fail = True
    if fail:
        print(f"APK native libs 校验失败: {apk}", file=sys.stderr)
        return 1
    print(f"APK native libs 校验通过: {apk}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
