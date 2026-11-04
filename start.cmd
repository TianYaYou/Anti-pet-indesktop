@echo off
chcp 65001 >nul

cd /d "%~dp0"

if "%1"=="--web" (
  for /f "tokens=5" %%a in ('netstat -aon 2^>nul ^| findstr "127.0.0.1:4199" ^| findstr "LISTENING"') do (
    taskkill /f /pid %%a >nul 2>&1
  )
  start "Anti-Pet Server" /b node server/index.js
  timeout /t 1 /nobreak >nul
  start http://127.0.0.1:4199
  exit /b 0
)

if "%1"=="--slint" (
  if exist "desktop\target\release\slint_settings.exe" (
    start "" "desktop\target\release\slint_settings.exe"
    exit /b 0
  )
  cargo run --bin slint_settings --manifest-path desktop\Cargo.toml
  exit /b 0
)

:: 优先以无黑框 GUI 模式直启已编译的 Release 二进制
if exist "desktop\target\release\anti-pet-desktop.exe" (
  start "" "desktop\target\release\anti-pet-desktop.exe" %*
  exit /b 0
)

if exist "desktop\target\debug\anti-pet-desktop.exe" (
  start "" "desktop\target\debug\anti-pet-desktop.exe" %*
  exit /b 0
)

:: 首次运行调用 cargo 编译并启动
echo 正在首次编译并启动 Anti-Pet 纯 CPU 2D 伴侣桌宠...
cargo run --release --bin anti-pet-desktop --manifest-path desktop\Cargo.toml -- %*
