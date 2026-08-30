#!/usr/bin/env bash
set -euo pipefail

VERSION="${1:-0.0.0}"
ARCH="${2:-$(uname -m)}"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
DIST="$ROOT/dist/macos"
STAGE="$DIST/stage"
BINARY_DIR="${BINARY_DIR:-$ROOT/target/release}"
MULTICA_RESOURCE_DIR="${MULTICA_RESOURCE_DIR:-}"
DMG="$DIST/claude-codex-pro-${VERSION}-macos-${ARCH}.dmg"
ICON_SOURCE="$ROOT/apps/claude-codex-pro-manager/src-tauri/icons/icon.png"
ICON_NAME="claude-codex-pro.icns"
ICON_ICNS="$DIST/$ICON_NAME"

rm -rf "$DIST"
mkdir -p "$STAGE"

prepare_icon() {
  local iconset="$DIST/claude-codex-pro.iconset"
  rm -rf "$iconset"
  mkdir -p "$iconset"

  sips -z 16 16 "$ICON_SOURCE" --out "$iconset/icon_16x16.png" >/dev/null
  sips -z 32 32 "$ICON_SOURCE" --out "$iconset/icon_16x16@2x.png" >/dev/null
  sips -z 32 32 "$ICON_SOURCE" --out "$iconset/icon_32x32.png" >/dev/null
  sips -z 64 64 "$ICON_SOURCE" --out "$iconset/icon_32x32@2x.png" >/dev/null
  sips -z 128 128 "$ICON_SOURCE" --out "$iconset/icon_128x128.png" >/dev/null
  sips -z 256 256 "$ICON_SOURCE" --out "$iconset/icon_128x128@2x.png" >/dev/null
  sips -z 256 256 "$ICON_SOURCE" --out "$iconset/icon_256x256.png" >/dev/null
  sips -z 512 512 "$ICON_SOURCE" --out "$iconset/icon_256x256@2x.png" >/dev/null
  sips -z 512 512 "$ICON_SOURCE" --out "$iconset/icon_512x512.png" >/dev/null
  sips -z 1024 1024 "$ICON_SOURCE" --out "$iconset/icon_512x512@2x.png" >/dev/null

  iconutil -c icns "$iconset" -o "$ICON_ICNS"
}

create_app() {
  local app_name="$1"
  local executable_name="$2"
  local binary_path="$3"
  local bundle_id="$4"
  local lsui_element="${5:-false}"
  local app_dir="$STAGE/$app_name.app"

  if [ ! -x "$binary_path" ]; then
    echo "error: binary not found or not executable: $binary_path" >&2
    return 1
  fi

  rm -rf "$app_dir"
  mkdir -p "$app_dir/Contents/MacOS" "$app_dir/Contents/Resources"
  if [ -n "$MULTICA_RESOURCE_DIR" ]; then
    if [ ! -d "$MULTICA_RESOURCE_DIR" ]; then
      echo "error: Multica resource directory not found: $MULTICA_RESOURCE_DIR" >&2
      return 1
    fi
    mkdir -p "$app_dir/Contents/Resources/multica"
    cp -R "$MULTICA_RESOURCE_DIR/." "$app_dir/Contents/Resources/multica/"
  fi
  cp "$binary_path" "$app_dir/Contents/MacOS/$executable_name"
  cp "$ICON_ICNS" "$app_dir/Contents/Resources/$ICON_NAME"
  chmod +x "$app_dir/Contents/MacOS/$executable_name"
  printf 'APPL????' > "$app_dir/Contents/PkgInfo"
  cat > "$app_dir/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>
  <string>$app_name</string>
  <key>CFBundleDisplayName</key>
  <string>$app_name</string>
  <key>CFBundleIdentifier</key>
  <string>$bundle_id</string>
  <key>CFBundleVersion</key>
  <string>$VERSION</string>
  <key>CFBundleShortVersionString</key>
  <string>$VERSION</string>
  <key>CFBundleInfoDictionaryVersion</key>
  <string>6.0</string>
  <key>CFBundlePackageType</key>
  <string>APPL</string>
  <key>CFBundleSignature</key>
  <string>????</string>
  <key>CFBundleExecutable</key>
  <string>$executable_name</string>
  <key>CFBundleIconFile</key>
  <string>$ICON_NAME</string>
  <key>LSMinimumSystemVersion</key>
  <string>12.0</string>
  <key>NSHighResolutionCapable</key>
  <true/>
  <key>LSUIElement</key>
  <$lsui_element/>
</dict>
</plist>
PLIST
}

install_app_runtime() {
  local runtime_name="$1"
  local binary_path="$BINARY_DIR/$runtime_name"
  local destination="$STAGE/Claude Codex Pro.app/Contents/MacOS/$runtime_name"

  if [ ! -x "$binary_path" ]; then
    echo "error: runtime binary not found or not executable: $binary_path" >&2
    return 1
  fi

  cp "$binary_path" "$destination"
  chmod +x "$destination"
}

describe_macos_binary() {
  local label="$1"
  local binary_path="$2"

  echo "codesign target: $label ($binary_path)"
  file "$binary_path"
  lipo -info "$binary_path"
}

sign_and_verify_binary() {
  local label="$1"
  local binary_path="$2"

  describe_macos_binary "$label" "$binary_path"
  codesign --force --sign - "$binary_path"
  codesign --verify --strict --verbose=4 "$binary_path"
  codesign -d --verbose=4 "$binary_path" 2>&1
}

sign_app() {
  local app_dir="$1"
  local executable
  local main_executable
  local mcp_runtime
  executable="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleExecutable' "$app_dir/Contents/Info.plist")"
  main_executable="$app_dir/Contents/MacOS/$executable"
  mcp_runtime="$app_dir/Contents/MacOS/claude-codex-pro-mcp"

  echo "codesign host architecture: $(uname -m)"
  sign_and_verify_binary "MCP runtime" "$mcp_runtime"
  sign_and_verify_binary "main executable" "$main_executable"
  codesign --force --sign - "$app_dir"
  codesign --verify --deep --strict --verbose=4 "$app_dir"
  codesign -d --verbose=4 "$app_dir" 2>&1
}

verify_app() {
  local app_dir="$1"
  local plist="$app_dir/Contents/Info.plist"
  local plutil_bin
  plutil_bin="$(command -v plutil || true)"
  if [ -n "$plutil_bin" ]; then
    "$plutil_bin" -lint "$plist" >/dev/null
  else
    /usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$plist" >/dev/null
  fi
  if [ ! -f "$app_dir/Contents/PkgInfo" ]; then
    echo "error: missing PkgInfo in $app_dir" >&2
    return 1
  fi
  codesign --verify --deep --strict --verbose=4 "$app_dir" || {
    echo "error: codesign verification failed for $app_dir" >&2
    return 1
  }
  for runtime in claude-codex-pro claude-codex-pro-mcp; do
    if [ -f "$app_dir/Contents/MacOS/$runtime" ]; then
      codesign --verify --strict --verbose=4 "$app_dir/Contents/MacOS/$runtime" || {
        echo "error: runtime codesign verification failed for $runtime in $app_dir" >&2
        return 1
      }
    fi
  done
}

verify_app_runtime_before_signing() {
  local macos_dir="$STAGE/Claude Codex Pro.app/Contents/MacOS"
  for runtime in claude-codex-pro claude-codex-pro-mcp; do
    if [ ! -x "$macos_dir/$runtime" ]; then
      echo "error: app bundle runtime missing or not executable: $runtime" >&2
      return 1
    fi
  done
}

prepare_icon
create_app "Claude Codex Pro" "claude-codex-pro" "$BINARY_DIR/claude-codex-pro" "com.damonzs.claudecodexpro" "false"
install_app_runtime "claude-codex-pro-mcp"
ln -s /Applications "$STAGE/Applications"

verify_app_runtime_before_signing
sign_app "$STAGE/Claude Codex Pro.app"

verify_app "$STAGE/Claude Codex Pro.app"

hdiutil create -volname "Claude Codex Pro" -srcfolder "$STAGE" -ov -format UDZO "$DMG"
echo "$DMG"
