#!/usr/bin/env bash
# Postwoman을 macOS .app 번들로 패키징한다.
# 사용법: ./scripts/bundle-macos.sh [바이너리경로]
#   - 인자 없으면 target/release/postwoman 사용 (먼저 `cargo build --release` 필요)
#   - 유니버설 빌드 등 다른 바이너리를 쓰려면 경로를 인자로 전달
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/dist/Postwoman.app"
BIN="${1:-$ROOT/target/release/postwoman}"

if [[ ! -x "$BIN" ]]; then
  echo "릴리스 바이너리가 없습니다. 먼저 'cargo build --release'를 실행하세요." >&2
  exit 1
fi

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"

cp "$BIN" "$APP/Contents/MacOS/Postwoman"
chmod +x "$APP/Contents/MacOS/Postwoman"

cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key>            <string>Postwoman</string>
  <key>CFBundleDisplayName</key>     <string>Postwoman</string>
  <key>CFBundleExecutable</key>      <string>Postwoman</string>
  <key>CFBundleIdentifier</key>      <string>com.kyle.postwoman</string>
  <key>CFBundleVersion</key>         <string>0.1.0</string>
  <key>CFBundleShortVersionString</key> <string>0.1.0</string>
  <key>CFBundlePackageType</key>     <string>APPL</string>
  <key>LSMinimumSystemVersion</key>  <string>11.0</string>
  <key>NSHighResolutionCapable</key> <true/>
</dict>
</plist>
PLIST

# 코드서명이 없으면 Gatekeeper가 격리할 수 있으므로 ad-hoc 서명.
codesign --force --deep --sign - "$APP" 2>/dev/null || true

echo "생성 완료: $APP"
