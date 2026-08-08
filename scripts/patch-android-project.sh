#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-.}"
ANDROID_APP_DIR="$ROOT_DIR/src-tauri/gen/android/app"
ACTIVITY="$ANDROID_APP_DIR/src/main/java/ai/easycall/app/MainActivity.kt"
MANIFEST="$ANDROID_APP_DIR/src/main/AndroidManifest.xml"
NOTIFICATION_ICON_SRC="$ROOT_DIR/src-tauri/icons/android/drawable/ic_stat_pai.png"
NOTIFICATION_ICON_DST="$ANDROID_APP_DIR/src/main/res/drawable/ic_stat_pai.png"

if [[ ! -f "$MANIFEST" ]]; then
  echo "Android manifest not found: $MANIFEST" >&2
  exit 1
fi

# 通知小图标：PAI 原图标（通知插件全局配置 icon=ic_stat_pai 引用）
if [[ -f "$NOTIFICATION_ICON_SRC" ]]; then
  mkdir -p "$(dirname "$NOTIFICATION_ICON_DST")"
  cp "$NOTIFICATION_ICON_SRC" "$NOTIFICATION_ICON_DST"
  echo "Copied notification icon to $NOTIFICATION_ICON_DST"
else
  echo "Notification icon source not found: $NOTIFICATION_ICON_SRC" >&2
fi

mkdir -p "$(dirname "$ACTIVITY")"
cat > "$ACTIVITY" <<'KOTLIN'
package ai.easycall.app

import android.os.Bundle
import android.webkit.WebSettings
import android.webkit.WebView
import androidx.core.view.WindowCompat

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WindowCompat.setDecorFitsSystemWindows(window, false)
    }

    // 远程前端模式：壳层 origin 为 https://tauri.localhost（满足电脑端桥接的 https 校验），
    // 但嵌入的电脑 PAI iframe 是 http://<电脑IP>:8429/sidebar。https 页面加载 http 子资源
    // 会触发 Android WebView 的 Mixed Content 默认拦截（MIXED_CONTENT_NEVER_ALLOW），导致
    // 电脑页面整体加载失败（PC 端显示无连接、设置页打不开）。
    // 这里放开混合内容，让 http 电脑 iframe 能在 https 壳层内正常加载。
    override fun onWebViewCreate(webView: WebView) {
        super.onWebViewCreate(webView)
        webView.settings.mixedContentMode = WebSettings.MIXED_CONTENT_ALWAYS_ALLOW
    }
}
KOTLIN

if ! grep -q 'android.permission.INTERNET' "$MANIFEST"; then
  sed -i '/<application/i\    <uses-permission android:name="android.permission.INTERNET" />' "$MANIFEST"
  echo "Added INTERNET permission"
else
  echo "INTERNET permission already exists"
fi

if ! grep -q 'android.permission.ACCESS_NETWORK_STATE' "$MANIFEST"; then
  sed -i '/<application/i\    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />' "$MANIFEST"
  echo "Added ACCESS_NETWORK_STATE permission"
else
  echo "ACCESS_NETWORK_STATE permission already exists"
fi

if ! grep -q 'android.permission.RECORD_AUDIO' "$MANIFEST"; then
  sed -i '/<application/i\    <uses-permission android:name="android.permission.RECORD_AUDIO" />' "$MANIFEST"
  echo "Added RECORD_AUDIO permission"
else
  echo "RECORD_AUDIO permission already exists"
fi

if ! grep -q 'android.permission.MODIFY_AUDIO_SETTINGS' "$MANIFEST"; then
  sed -i '/<application/i\    <uses-permission android:name="android.permission.MODIFY_AUDIO_SETTINGS" />' "$MANIFEST"
  echo "Added MODIFY_AUDIO_SETTINGS permission"
else
  echo "MODIFY_AUDIO_SETTINGS permission already exists"
fi

if ! grep -q 'android.hardware.microphone' "$MANIFEST"; then
  sed -i '/<application/i\    <uses-feature android:name="android.hardware.microphone" android:required="false" />' "$MANIFEST"
  echo "Added microphone feature declaration"
else
  echo "Microphone feature declaration already exists"
fi

if ! grep -q 'extractNativeLibs' "$MANIFEST"; then
  sed -i 's/<application/<application android:extractNativeLibs="true"/' "$MANIFEST"
  echo "Added extractNativeLibs=true"
else
  echo "extractNativeLibs already present"
fi

# 导出分享：让模板自带的 FileProvider（authority ${applicationId}.fileprovider）
# 能解析沙盒根 app_data_dir()（Android 上为 /data/data/<pkg>/，即 filesDir 的父级）。
# 模板默认 file_paths.xml 只有 external-path + cache-path，覆盖不到 dataDir 下的
# llm-workspace；追加 root-path 兜底（仅对显式 getUriForFile 的文件生成 URI，
# 配合 grantUriPermissions 临时授权，不扩大默认暴露面）。
FILE_PATHS="$ANDROID_APP_DIR/src/main/res/xml/file_paths.xml"
if [[ -f "$FILE_PATHS" ]]; then
  if ! grep -q 'workspace-io-root' "$FILE_PATHS"; then
    sed -i 's#</paths>#  <root-path name="workspace-io-root" path="." />\n</paths>#' "$FILE_PATHS"
    echo "Appended root-path to $FILE_PATHS"
  else
    echo "root-path already present in $FILE_PATHS"
  fi
else
  echo "WARNING: file_paths.xml not found at $FILE_PATHS; export share will fail to resolve sandbox paths" >&2
fi

echo "=== MainActivity.kt ==="
cat "$ACTIVITY"
echo "=== AndroidManifest.xml permission entries ==="
grep -nE 'INTERNET|ACCESS_NETWORK_STATE|RECORD_AUDIO|MODIFY_AUDIO_SETTINGS|android.hardware.microphone' "$MANIFEST" || true
