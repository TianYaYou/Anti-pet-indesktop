# Anti-Pet In Desktop · 大肥猫桌宠伴侣

> 轻量级、高颜值的跨平台 Antigravity 智能伴侣桌宠，支持 macOS 与 Windows。实时联动 Agent 思考与工具执行状态，开箱自带傲娇台词、额度监控、按压回弹与声明式换肤系统。

<p align="center">
  <img src="skins/fat-cat/frames/idle.png" width="180" alt="大肥猫桌宠">
</p>

---

## 🌟 核心特性

- 🐾 **大肥猫专属形态**：支持待机呼吸、按压形变、弹性回弹、鼠标抓取悬空与左右转身。
- ⚡ **毫秒级 Agent 状态联动**：
  - 🧠 **思考中** (PreInvocation)：猫猫探头思考，展示思考粒子与脑回路台词；
  - 🛠️ **工具执行中** (PreToolUse)：挥舞小扳手，展示工具名称；
  - ✨ **任务完成** (PostToolUse/Invocation)：duang 弹跳庆祝，飘落星星；
  - 😿 **报错遇阻** (Error)：委屈掉眼泪，及时给予反馈。
- 📊 **Gemini 双周期额度胶囊**：悬浮气泡实时显示 5 小时与每周用量进度，低于 30% 琥珀色预警，低于 10% 玫红告警。
- 🔊 **物理音效与傲娇台词**：按压气泡音、清脆点击音、摸头呼噜呼噜反馈，内置数十句中文互动台词。
- 🪶 **超小体积与双模态架构**：
  - **模态 A (In-IDE & Web 纯享)**：基于纯 Webview，**0MB 额外二进制**，随时随地在侧栏或浏览器查看；
  - **模态 B (Desktop 桌面穿透悬浮)**：基于 Rust Wry 微内核，**独立打包体积仅 ~6MB**（告别 150MB+ 的重型 Electron 与 Python/Qt），支持跨平台透明穿透与置顶。
- 👗 **极简可扩展换肤**：只需在 `skins/<name>/` 下放置 `skin.json` 与切片图片，即可即时热插拔。

---

## 🚀 快速上手

### 1. 启动轻量服务与 Web 伴侣 (全平台推荐)

```bash
# 启动本地服务与实时事件总线
npm start
```
打开浏览器访问：`http://127.0.0.1:4199` 即可立即体验大肥猫伴侣交互。

### 2. 启动桌面透明穿透挂件 (macOS & Windows)

```bash
# 运行极小微内核桌面端
npm run desktop

# 生产级构建 (<8MB 独立可执行文件)
npm run desktop:build
```

---

## 🔌 Antigravity 插件配置

本项目完全兼容 Antigravity 官方插件规范：

1. **软链接或复制到插件目录**：
   ```bash
   ln -s "$(pwd)" ~/.gemini/config/plugins/anti-pet-indesktop
   ```
2. **生命周期钩子** (`hooks.json`) 会在会话启动、Agent 思考、执行命令时自动同步状态到大肥猫。
3. **在对话中互动**：可以在 Antigravity 中对 Agent 说：
   - *“让大肥猫说一句傲娇台词”*
   - *“查看当前大肥猫伴侣状态与额度”*

---

## 🎨 制作与替换新皮肤

在 `skins/` 目录下创建新文件夹，例如 `skins/my-pet/`：

```text
skins/my-pet/
├── skin.json          # 动作机映射、渲染尺寸、音效路径
├── frames/            # 各状态精灵图 (idle.png, proud.png, sad.png...)
├── audio/             # 音效 (可选)
└── dialogue.json      # 专属台词库
```

`skin.json` 示例：
```json
{
  "id": "my-pet",
  "name": "自定义小萌宠",
  "version": "1.0.0",
  "render": { "defaultWidth": 200 },
  "frames": {
    "idle": "frames/idle.png",
    "proud": "frames/proud.png",
    "sad": "frames/sad.png"
  },
  "states": {
    "idle": { "frame": "idle", "motion": "breath", "loop": true },
    "thinking": { "frame": "idle", "motion": "ponder", "loop": true },
    "success": { "frame": "proud", "motion": "duang", "durationMs": 3000 }
  }
}
```

---

## 📁 目录结构

```text
Anti-pet-indesktop/
├── plugin.json               # Antigravity 插件清单
├── hooks.json                # IDE 生命周期事件监听
├── rules/AGENTS.md           # 智能体交互规范
├── skills/desktop-pet/       # 伴侣交互专属技能
├── core/                     # 纯 Web 表现层核心 (Canvas / 动作机 / 物理拖拽 / 额度条)
├── skins/                    # 皮肤库 (预置: fat-cat 大肥猫星语伴侣)
├── server/                   # 本地轻量事件中枢 (Zero-dep Node.js HTTP/SSE)
├── desktop/                  # 跨平台极小桌面壳 (Rust Wry/Tao)
└── package.json
```

## 📄 许可声明

代码采用 MIT 许可证。部分美术素材、音频与灵感致敬源自开源社区项目 `dsh-pet-indesktop` 与 `Antigravity-Star-Suite`。
