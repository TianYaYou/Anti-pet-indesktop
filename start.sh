#!/usr/bin/env bash
# Start Anti-Pet in Desktop (macOS / Linux)
cd "$(dirname "$0")"

echo "🐾 正在启动 Anti-Pet 服务..."
node server/index.js &
SERVER_PID=$!

cleanup() {
  echo ""
  echo "正在关闭 Anti-Pet 服务..."
  kill $SERVER_PID 2>/dev/null
  exit 0
}

trap cleanup INT TERM

sleep 1

# Check if desktop binary should run or open web
if [ "$1" == "--web" ]; then
  echo "🌐 正在打开 Web 预览模式..."
  if command -v open >/dev/null 2>&1; then
    open "http://127.0.0.1:4199"
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "http://127.0.0.1:4199"
  fi
  wait $SERVER_PID
else
  echo "🖥️  正在启动桌面透明桌宠..."
  cargo run --manifest-path desktop/Cargo.toml
  cleanup
fi
