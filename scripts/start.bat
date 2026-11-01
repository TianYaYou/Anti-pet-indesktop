@echo off
chcp 65001 >nul
title 大肥猫桌宠挂件 (Anti-Pet In Desktop)

cd /d "%~dp0.."

echo ==========================================
echo   🐾 启动 大肥猫桌宠挂件 (Anti-Pet) v0.1.0
echo ==========================================

:: 1. 尝试静默启动后台 node 服务
start "" /b node server\index.js >nul 2>&1
timeout /t 1 /nobreak >nul

:: 2. 启动桌面挂件
if exist anti-pet-desktop.exe (
    start "" anti-pet-desktop.exe
) else if exist desktop\target\release\anti-pet-desktop.exe (
    start "" desktop\target\release\anti-pet-desktop.exe
) else (
    echo [错误] 未找到 anti-pet-desktop.exe 可执行程序！
    pause
)
