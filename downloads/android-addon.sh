#!/bin/sh
# ============================================================================
#  York Android Addon  (Linux / macOS)
#  Downloads the Android build compiler tools into ~/.york/android so that
#  `york mobile app.yk --apk` can produce a real signed .apk.
#  This is an OPTIONAL addon — like installing Android Studio — it is NOT
#  bundled with the York language installer.
#
#  Requires: curl, unzip (macOS only), a JDK for signing.
#  Size: ~120 MB one-time download from Google's repository.
# ============================================================================
set -e

ROOT="${HOME}/.york/android"
mkdir -p "$ROOT"

case "$(uname -s | tr '[:upper:]' '[:lower:]')" in
  linux*)  OS="linux" ;;
  darwin*) OS="macosx" ;;
  *)       echo "unsupported OS — use android-addon.ps1 on Windows" >&2; exit 1 ;;
esac

BT_DIR="$ROOT/build-tools-36"
PLATFORM_DIR="$ROOT/platforms-34"
JAR="$PLATFORM_DIR/android.jar"
BT_ZIP="$ROOT/build-tools.zip"
PL_ZIP="$ROOT/platform.zip"

BT_URL="https://dl.google.com/android/repository/build-tools_r36.1-${OS}.zip"
PLATFORM_URL="https://dl.google.com/android/repository/platform-34-ext7_r03.zip"

echo "York Android Addon — installing aapt2 / d8 / zipalign / apksigner + android.jar (~120MB, one-time)."

if [ ! -f "$BT_DIR/aapt2" ] && [ ! -f "$BT_DIR/aapt2.exe" ]; then
  if [ ! -f "$BT_ZIP" ]; then
    echo "  downloading Android build tools ..."
    curl -fL -C - -o "$BT_ZIP" "$BT_URL"
  fi
  TMP="$ROOT/bt_extract"; rm -rf "$TMP"; mkdir -p "$TMP"
  if command -v unzip >/dev/null 2>&1; then unzip -q "$BT_ZIP" -d "$TMP"; else tar -xf "$BT_ZIP" -C "$TMP"; fi
  INNER=$(ls -d "$TMP"/*/ 2>/dev/null | head -n1)
  [ -n "$INNER" ] || { echo "build tools archive was empty" >&2; exit 1; }
  mkdir -p "$BT_DIR"
  cp -R "$INNER"/* "$BT_DIR/"
  rm -rf "$TMP"
fi

if [ ! -f "$JAR" ]; then
  if [ ! -f "$PL_ZIP" ]; then
    echo "  downloading Android platform android.jar ..."
    curl -fL -C - -o "$PL_ZIP" "$PLATFORM_URL"
  fi
  TMP="$ROOT/pl_extract"; rm -rf "$TMP"; mkdir -p "$TMP"
  if command -v unzip >/dev/null 2>&1; then unzip -q "$PL_ZIP" -d "$TMP"; else tar -xf "$PL_ZIP" -C "$TMP"; fi
  INNER=$(ls -d "$TMP"/*/ 2>/dev/null | head -n1)
  [ -n "$INNER" ] || { echo "platform archive was empty" >&2; exit 1; }
  rm -rf "$PLATFORM_DIR"
  cp -R "$INNER" "$PLATFORM_DIR"
  rm -rf "$TMP"
fi

if [ ! -f "$ROOT/debug.keystore" ]; then
  KEYTOOL=$(command -v keytool || true)
  if [ -n "$KEYTOOL" ]; then
    echo "  creating local debug signing key ..."
    "$KEYTOOL" -genkeypair -v -keystore "$ROOT/debug.keystore" -storepass android -alias androiddebugkey -keypass android -keyalg RSA -keysize 2048 -validity 10000 -dname "CN=Android Debug,O=York,C=US" 2>/dev/null || true
  else
    echo "  (no JDK on PATH — signing key will be created automatically by York when needed)"
  fi
fi

rm -f "$BT_ZIP" "$PL_ZIP"

echo ""
echo "Done: York Android Addon installed in $ROOT"
echo "  Usage:"
echo "    cd your-app"
echo "    york mobile src/app.yk --apk"
echo ""