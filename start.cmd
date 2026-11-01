@echo off
chcp 65001 >nul
echo 🐾 正在启动 Anti-Pet 伴侣桌宠 (Windows)...
start "Anti-Pet Server" /b node server/index.js
timeout /t 1 /nobreak >nul

if "%1"=="--web" (
  start http://127.0.0.1:4199
) else (
  cargo run --manifest-path desktop/Cargo.toml
)
