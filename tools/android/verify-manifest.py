#!/usr/bin/env python3
"""验证 AndroidManifest.xml 静态安全约束（cleartext、前台服务声明、权限）。

用法: python tools/android/verify-manifest.py <manifest-path>
"""
import sys


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: verify-manifest.py <manifest-path>", file=sys.stderr)
        return 1
    path = sys.argv[1]
    try:
        text = open(path, encoding="utf-8").read()
    except OSError as exc:
        print(f"ERROR: 无法读取 manifest {path}: {exc}", file=sys.stderr)
        return 1

    fail = False
    if 'usesCleartextTraffic="true"' in text:
        print("ERROR: usesCleartextTraffic=true", file=sys.stderr)
        fail = True
    else:
        print("OK: usesCleartextTraffic 未强制开启")

    if "PaiForegroundService" in text:
        print("OK: PaiForegroundService 已声明")
    else:
        print("ERROR: PaiForegroundService 未声明", file=sys.stderr)
        fail = True

    for perm in [
        "android.permission.FOREGROUND_SERVICE",
        "android.permission.POST_NOTIFICATIONS",
        "android.permission.INTERNET",
    ]:
        if perm in text:
            print(f"OK: {perm}")
        else:
            print(f"WARNING: {perm} 缺失", file=sys.stderr)

    return 1 if fail else 0


if __name__ == "__main__":
    sys.exit(main())
