#!/usr/bin/env bash
# 将 git 驱动版本号注入 Tauri Android 工程（CI 中 init 后执行，幂等）。
# versionCode = git rev-list --count HEAD
# versionName = git describe --tags --always 派生：
#   精确 tag vX.Y.Z            -> X.Y.Z
#   带预发布标签 vX.Y.Z-pre    -> X.Y.Z-pre
#   带提交距离 vX.Y.Z-N-g...   -> X.Y.(Z+1)-alpha.N
#   无 tag                     -> 去 v 前缀或 0.0.0-dev
set -euo pipefail

ROOT_DIR="${1:-.}"
GRADLE="$ROOT_DIR/src-tauri/gen/android/app/build.gradle.kts"

if [[ ! -f "$GRADLE" ]]; then
  echo "build.gradle.kts not found: $GRADLE" >&2
  exit 1
fi

if grep -q 'gitVersionCode' "$GRADLE"; then
  echo "Version logic already injected, skip"
  exit 0
fi

python3 - "$GRADLE" <<'PY'
import sys
path = sys.argv[1]
src = open(path, encoding='utf-8').read()
block = r'''val gitVersionCode: Int by lazy {
    providers.exec {
        commandLine("git", "rev-list", "--count", "HEAD")
    }.standardOutput.asText.get().trim().toInt()
}

val gitVersionName: String by lazy {
    val desc = providers.exec {
        commandLine("git", "describe", "--tags", "--always")
        isIgnoreExitValue = true
    }.standardOutput.asText.get().trim()
    val match =
        Regex("""^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.]+))?(?:-(\d+)-g[0-9a-f]+)?$""").matchEntire(
            desc
        )
    if (match != null) {
        val (major, minor, patch, pre, distance) = match.destructured
        when {
            distance.isEmpty() && pre.isEmpty() -> "$major.$minor.$patch"
            distance.isEmpty() -> "$major.$minor.$patch-$pre"
            else -> "$major.$minor.${patch.toInt() + 1}-alpha.$distance"
        }
    } else {
        desc.removePrefix("v").ifEmpty { "0.0.0-dev" }
    }
}

android {'''
if 'android {' not in src:
    print('ERROR: android { block not found in %s' % path, file=sys.stderr)
    sys.exit(1)
src = src.replace('android {', block, 1)
open(path, 'w', encoding='utf-8').write(src)
print("Inserted git version logic into %s" % path)
PY

sed -i \
  -e 's|versionCode = 1|versionCode = gitVersionCode|' \
  -e 's|versionName = "0.57.0"|versionName = gitVersionName|' \
  "$GRADLE"

# 派生当前版本（与 gradle gitVersionName 同一逻辑），供 cargo 构建期
# 以 PAI_ANDROID_APP_VERSION 注入 Rust（get_app_version / updater 同源）。
GIT_DESC="$(git describe --tags --always 2>/dev/null || echo '')"
VERSION_NAME="$(python3 - "$GIT_DESC" <<'PY'
import re, sys
desc = sys.argv[1]
m = re.match(r"^v?(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.]+))?(?:-(\d+)-g[0-9a-f]+)?$", desc)
if m:
    major, minor, patch, pre, distance = m.groups()
    if not distance and not pre:
        print(f"{major}.{minor}.{patch}")
    elif not distance:
        print(f"{major}.{minor}.{patch}-{pre}")
    else:
        print(f"{major}.{minor}.{int(patch)+1}-alpha.{distance}")
else:
    print(desc.removeprefix("v") or "0.0.0-dev")
PY
)"
if [[ -n "${GITHUB_ENV:-}" ]]; then
  echo "PAI_ANDROID_APP_VERSION=$VERSION_NAME" >> "$GITHUB_ENV"
  echo "导出 PAI_ANDROID_APP_VERSION=$VERSION_NAME"
fi

echo "=== version lines after patch ==="
grep -nE 'versionCode =|versionName =|gitVersion' "$GRADLE" | head -10
echo "=== derived app version ==="
echo "$VERSION_NAME"
