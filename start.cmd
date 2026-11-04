@echo off
chcp 65001 >nul

if exist "C:\Program Files\Rust stable MSVC 1.91\bin" (
  set "PATH=C:\Program Files\Rust stable MSVC 1.91\bin;%PATH%"
)

echo 正在启动 Anti-Pet 伴侣桌宠 (Windows)...

:: 1. 尝试结束 4199 端口上的旧实例，确保加载最新实时额度探针代码
for /f "tokens=5" %%a in ('netstat -aon 2^>nul ^| findstr "127.0.0.1:4199" ^| findstr "LISTENING"') do (
  taskkill /f /pid %%a >nul 2>&1
)

:: 2. 后台启动 node 服务
start "Anti-Pet Server" /b node server/index.js
timeout /t 1 /nobreak >nul

if "%1"=="--web" (
  start http://127.0.0.1:4199
) else (
  cargo run --manifest-path desktop/Cargo.toml -- %*
)
