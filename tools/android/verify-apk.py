#!/usr/bin/env python3
"""验证 APK：native libs + release cleartext=false（合并 Manifest 静态校验）。

用法: python tools/android/verify-apk.py <apk-path> [release|debug]
"""
import os
import re
import sys
import zipfile

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

REQUIRED = [
    "lib/arm64-v8a/libeasy_call_ai_lib.so",
    "lib/arm64-v8a/libproot_exec.so",
    "lib/arm64-v8a/libproot_loader.so",
    "lib/arm64-v8a/libtalloc.so",
    "lib/arm64-v8a/libandroid-shmem.so",
]


def check_native_libs(apk: str) -> bool:
    names = set(zipfile.ZipFile(apk).namelist())
    ok = True
    for lib in REQUIRED:
        if lib in names:
            print(f"OK: {lib}")
        else:
            print(f"ERROR: {lib} 缺失", file=sys.stderr)
            ok = False
    return ok


def check_release_cleartext() -> bool:
    # 优先合并 Manifest；缺失时回退检查 manifest 源文件中的 release 占位符配置
    candidates = [
        os.path.join(ROOT, "apps/android/app/build/intermediates/merged_manifests/release/processReleaseManifest/AndroidManifest.xml"),
        os.path.join(ROOT, "apps/android/app/build/intermediates/merged_manifest/release/processReleaseManifest/AndroidManifest.xml"),
    ]
    for path in candidates:
        if os.path.isfile(path):
            text = open(path, encoding="utf-8").read()
            if "usesCleartextTraffic=\"true\"" in text:
                print("ERROR: release Manifest usesCleartextTraffic=true（安全回归）", file=sys.stderr)
                return False
            print("OK: release usesCleartextTraffic=false")
            return True
    # build.gradle.kts release 块必须显式 false
    gradle = os.path.join(ROOT, "apps/android/app/build.gradle.kts")
    if os.path.isfile(gradle):
        text = open(gradle, encoding="utf-8").read()
        if re.search(r'getByName\("release"\)[\s\S]{0,400}usesCleartextTraffic"\]\s*=\s*"false"', text):
            print("OK: build.gradle.kts release 显式 usesCleartextTraffic=false")
            return True
    print("WARNING: 未找到 release cleartext 校验依据（合并 Manifest 或 gradle release 块）", file=sys.stderr)
    return False


def main() -> int:
    if len(sys.argv) < 2:
        print("usage: verify-apk.py <apk-path> [release|debug]", file=sys.stderr)
        return 1
    apk = sys.argv[1]
    mode = sys.argv[2] if len(sys.argv) > 2 else "release"

    ok = check_native_libs(apk)
    if mode == "release":
        ok = check_release_cleartext() and ok
    if not ok:
        print(f"APK 校验失败: {apk}", file=sys.stderr)
        return 1
    print(f"APK 校验通过: {apk}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
