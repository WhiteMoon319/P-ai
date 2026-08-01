#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="${1:-.}"
ANDROID_APP_DIR="$ROOT_DIR/src-tauri/gen/android/app"
ACTIVITY="$ANDROID_APP_DIR/src/main/java/ai/easycall/app/MainActivity.kt"
MANIFEST="$ANDROID_APP_DIR/src/main/AndroidManifest.xml"

if [[ ! -f "$MANIFEST" ]]; then
  echo "Android manifest not found: $MANIFEST" >&2
  exit 1
fi

mkdir -p "$(dirname "$ACTIVITY")"
cat > "$ACTIVITY" <<'KOTLIN'
package ai.easycall.app

import android.os.Bundle
import androidx.core.view.WindowCompat

class MainActivity : TauriActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        WindowCompat.setDecorFitsSystemWindows(window, false)
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

echo "=== MainActivity.kt ==="
cat "$ACTIVITY"
echo "=== AndroidManifest.xml permission entries ==="
grep -nE 'INTERNET|ACCESS_NETWORK_STATE|RECORD_AUDIO|MODIFY_AUDIO_SETTINGS|android.hardware.microphone' "$MANIFEST" || true
