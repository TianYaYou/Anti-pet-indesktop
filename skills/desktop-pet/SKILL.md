---
name: desktop-pet
description: 控制与联动 Antigravity 大肥猫伴侣桌宠，包括查看桌宠状态、触发台词/动作彩蛋、切换皮肤、启动桌面透明悬浮窗或内置预览。
---

# 大肥猫 · 桌面伴侣交互指南 (desktop-pet)

本技能用于管理、控制和联动当前运行的“大肥猫”桌面伴侣。

## 常用操作指令

### 1. 启动伴侣服务与预览
- **内置 Web 预览模式**（极速零依赖）：
  ```bash
  npm run dev
  ```
  在浏览器中打开 `http://127.0.0.1:4199` 即可实时查看伴侣渲染、调试动作和额度状态。

- **桌面透明悬浮穿透模式**（Rust/Wry 微内核桌面端）：
  ```bash
  npm run desktop
  ```
  启动全屏穿透、边缘吸附、物理拖动的桌面透明桌宠。

### 2. 状态与台词交互
通过轻量命令行更新伴侣状态与台词：
```bash
# 触发思考中状态
node server/hook-handler.js PreInvocation

# 触发工具执行状态
node server/hook-handler.js PreToolUse run_command

# 触发任务成功得意状态
node server/hook-handler.js PostToolUse run_command

# 触发自定义台词与情绪
node server/hook-handler.js custom --text "主人写代码辛苦啦，喝口水吧！" --state petting
```

### 3. 皮肤管理
- 皮肤根目录位于 `skins/`。
- 查看已安装皮肤：直接列出 `skins/` 目录下的文件夹。
- 动态切换皮肤：在桌宠菜单或通过更新配置切换 `activeSkin`。
