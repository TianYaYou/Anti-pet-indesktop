#!/usr/bin/env bash
# ==============================================================================
# 大肥猫桌宠挂件 (Anti-Pet In Desktop) 一键启动脚本
# ==============================================================================

set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )/.." && pwd )"
cd "$DIR"

echo "=========================================="
echo "  🐾 启动 大肥猫桌宠挂件 (Anti-Pet) v0.1.0"
echo "=========================================="

# 1. 检查 node 环境并启动本地轻量服务
if command -v node >/dev/null 2>&1; then
  echo "正在启动本地状态与额度同步服务 (端口 4199)..."
  node server/index.js &
  SERVER_PID=$!
  sleep 0.4
else
  echo "提示: 未检测到 node 命令，若已在外部启动服务可忽略。"
fi

# 2. 启动桌面透明穿透挂件核心
if [ -f "./anti-pet-desktop" ]; then
  echo "运行桌面挂件: ./anti-pet-desktop"
  ./anti-pet-desktop
elif [ -f "./desktop/target/release/anti-pet-desktop" ]; then
  echo "运行开发版挂件: ./desktop/target/release/anti-pet-desktop"
  ./desktop/target/release/anti-pet-desktop
else
  echo "错误: 未找到 anti-pet-desktop 可执行程序！"
  exit 1
fi
