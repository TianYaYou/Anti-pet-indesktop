#!/usr/bin/env bash
# Start Anti-Pet in Desktop (macOS / Linux)
cd "$(dirname "$0")"

if [ "$1" == "--web" ]; then
  echo "正在启动 Anti-Pet 本地 Web 服务..."
  node server/index.js &
  SERVER_PID=$!
  trap 'echo ""; echo "正在关闭 Anti-Pet 服务..."; kill $SERVER_PID 2>/dev/null; exit 0' INT TERM
  sleep 1
  echo "正在打开 Web 预览模式..."
  if command -v open >/dev/null 2>&1; then
    open "http://127.0.0.1:4199"
  elif command -v xdg-open >/dev/null 2>&1; then
    xdg-open "http://127.0.0.1:4199"
  fi
  wait $SERVER_PID
elif [ "$1" == "--appkit" ]; then
  echo "正在启动下一代 macOS AppKit 原生偏好设置 (0 KB 增量与原生毛玻璃)..."
  if [ -f "desktop/target/release/anti-pet-desktop" ]; then
    ./desktop/target/release/anti-pet-desktop --appkit
  else
    cargo run --bin anti-pet-desktop --manifest-path desktop/Cargo.toml -- --appkit
  fi
elif [ "$1" == "--slint" ]; then
  echo "正在启动下一代 Slint 现代偏好设置 (独立调试预览模式)..."
  cargo run --bin slint_settings --manifest-path desktop/Cargo.toml
elif [ "$1" == "--legacy" ]; then
  echo "正在启动 Anti-Pet 服务 (Legacy WebKit 模式)..."
  node server/index.js &
  SERVER_PID=$!
  trap 'echo ""; echo "正在关闭 Anti-Pet 服务..."; kill $SERVER_PID 2>/dev/null; exit 0' INT TERM
  sleep 1
  echo "正在启动 Legacy WebKit 桌面透明桌宠..."
  cargo run --release --manifest-path desktop/Cargo.toml --no-default-features --features legacy-webkit
  kill $SERVER_PID 2>/dev/null
else
  # macOS 原生优先：若已打包 Anti-Pet.app，优先通过 open 唤起，享受 0 终端无感启动
  if [ "$(uname)" = "Darwin" ] && [ -d "dist/Anti-Pet.app" ]; then
    echo "正在以 macOS 原生应用模式启动 Anti-Pet.app (无终端常驻)..."
    open dist/Anti-Pet.app
    exit 0
  fi

  if [ -f "desktop/target/release/anti-pet-desktop" ]; then
    echo "正在启动大肥猫桌宠 v0.5.1 (Pure CPU 2D + NextGen UI)..."
    exec ./desktop/target/release/anti-pet-desktop
  else
    echo "正在首次编译并启动大肥猫桌宠 v0.5.1..."
    cargo run --release --bin anti-pet-desktop --manifest-path desktop/Cargo.toml
  fi
fi
