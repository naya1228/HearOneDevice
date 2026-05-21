#!/usr/bin/env bash
set -e

# 현재 Rust 타겟 트리플 감지
TARGET=$(rustc -vV 2>/dev/null | grep host | awk '{print $2}')
if [ -z "$TARGET" ]; then
  echo "rustc를 찾을 수 없습니다. Rust가 설치되어 있는지 확인하세요."
  exit 1
fi

mkdir -p src-tauri/binaries
DEST="src-tauri/binaries/cloudflared-${TARGET}"

case "$TARGET" in
  x86_64-unknown-linux-gnu)
    URL="https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64"
    ;;
  aarch64-unknown-linux-gnu)
    URL="https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-arm64"
    ;;
  *)
    echo "지원하지 않는 타겟: $TARGET"
    exit 1
    ;;
esac

echo "Downloading cloudflared for $TARGET ..."
curl -fsSL "$URL" -o "$DEST"
chmod +x "$DEST"
echo "✅  $DEST"
