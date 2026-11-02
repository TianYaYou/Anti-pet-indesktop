# 大肥猫桌宠挂件 (Anti-Pet In Desktop)

<p align="center">
  <img src="assets/icon.png" width="160" alt="大肥猫桌宠挂件应用图标" style="border-radius: 36px; box-shadow: 0 16px 36px rgba(0,0,0,0.2);">
</p>

<p align="center">
  <a href="https://github.com/TianYaYou/Anti-pet-indesktop/releases"><img alt="版本" src="https://img.shields.io/badge/%E7%89%88%E6%9C%AC-v0.2.0-blue?style=flat-square"></a>
  <a href="https://github.com/TianYaYou/Anti-pet-indesktop/blob/main/LICENSE"><img alt="开源许可证" src="https://img.shields.io/badge/License-MIT-orange?style=flat-square"></a>
  <img alt="平台支持" src="https://img.shields.io/badge/%E5%B9%B3%E5%8F%B0-macOS%20%7C%20Windows-8A2BE2?style=flat-square">
  <img alt="核心体积" src="https://img.shields.io/badge/%E5%86%85%E6%A0%B8%E4%BD%93%E7%A7%AF-%3C8MB-brightgreen?style=flat-square">
  <img alt="技术栈" src="https://img.shields.io/badge/%E6%9E%B6%E6%9E%84-Rust%20Wry%20%2B%20Webview%20%2B%20Node-ff69b4?style=flat-square">
  <a href="https://github.com/TianYaYou/Anti-pet-indesktop/issues"><img alt="Issues" src="https://img.shields.io/github/issues/TianYaYou/Anti-pet-indesktop?style=flat-square"></a>
</p>

<p align="center">
  <b>面向 Google Antigravity IDE 与现代桌面的极轻量、跨平台半身大肥猫伴侣桌宠</b><br>
  全透明无感穿透 · 原生系统托盘常驻 · 滚轮丝滑缩放 · Agent 实时状态共鸣 · Gemini 双周期额度胶囊 · 物理重力吸附 · 声明式换肤体系
</p>

---

> ### v0.2.0 里程碑特性说明
>
> - **macOS 纯托盘 Accessory 模式**：原生适配系统托盘挂件标准，不在 Dock 栏驻留应用图标，启动即常驻托盘菜单，保持桌面环境干净整洁；
> - **独立原生偏好设置中心**：摒弃传统全屏/桌面毛玻璃模糊层，采用 macOS 与 WinUI 3 原生控件风格的专属设置窗口，分类管理常规显示、自启动与更新、皮肤管理、台词编辑以及右键菜单定制；
> - **兼容 dsh-pet 皮肤生态**：无缝支持 `dsh-pet-indesktop` 角色配置一键导入，智能映射帧宽高与动作帧序；
> - **动态右键菜单定制与 Antigravity 状态感应**：支持在偏好设置中自由启闭各项菜单功能，实时感知 Antigravity 进程运行状态并在已启动时自动隐藏重复唤起选项；
> - **脱离重型运行时**：告别动辄 150MB+ 的打包体积，基于 **Rust Wry/Tao** 微内核，单二进制文件仅 **约 6~8MB**，内存占用维持在 30MB 左右；
> - **动态像素穿透检测 (Hit-Testing)**：鼠标划入大肥猫身体或气泡时接管交互，划出透明区域时零阻挡，底层编辑器与浏览器点击畅通无阻。

---

## 📑 目录

- [一、功能特性概览](#一功能特性概览)
- [二、快速开始](#二快速开始)
  - [1. 桌面端启动 (macOS / Windows)](#1-桌面端启动-macos--windows)
  - [2. 浏览器 / 内置伴侣模式](#2-浏览器--内置伴侣模式)
  - [3. 挂载为 Antigravity 官方插件](#3-挂载为-antigravity-官方插件)
- [三、交互与操作指南](#三交互与操作指南)
  - [滚轮缩放](#滚轮缩放)
  - [手势揉头与摸摸](#手势揉头与摸摸)
  - [物理抓取与重力吸附](#物理抓取与重力吸附)
  - [完全穿透免打扰模式](#完全穿透免打扰模式)
  - [光标注视与输入法感知](#光标注视与输入法感知)
- [四、系统托盘管理](#四系统托盘管理)
- [五、自定义皮肤与角色包扩展](#五自定义皮肤与角色包扩展)
- [六、配置目录与隐私声明](#六配置目录与隐私声明)
- [七、常见问题 (FAQ)](#七常见问题-faq)
- [八、项目致谢与开源许可](#八项目致谢与开源许可)

---

## 一、功能特性概览

| 模块 | 核心能力 | 体验细节 |
| :--- | :--- | :--- |
| **桌面表现层** | 全透明无边框、跨工作区置顶 (Topmost) | macOS 多虚拟桌面跟随，Windows DWM 硬件级透明混合，无黑边无白框 |
| **交互引擎** | 鼠标滚轮丝滑缩放、按压形变、弹性回弹 | 滚轮步进阻尼插值（55%~240%），自动记住缩放偏好，带弹性气泡反馈 |
| **物理系统** | 自由拖拽悬空、抛投下落、Dock 边栏吸附 | 拖拽时根据位移速度实时左右转身；松手后受重力下落吸附底栏与侧边 |
| **情感手势** | 揉头撫摸（Petting）、连续点击互动 | 鼠标在猫猫头部快速来回划动触发享受呼噜表情、冒出爱心粒子与定制台词 |
| **IDE 状态共鸣** | 实时感知 Antigravity Agent 执行生命周期 | 思考中 (Thinking) / 执行工具中 (Working) / 成功 (Success) / 报错 (Error) |
| **用量监控** | Gemini 5小时与每周双额度指示条 | 悬浮云朵气泡直接读取 Antigravity 账本，低于 30% 琥珀色预警，低于 10% 玫红告警 |
| **系统托管** | 跨平台菜单栏 / 任务栏托盘托管 | 点击托盘图标显隐桌宠，右键快捷菜单提供免打扰、缩放重置、切换皮肤与安全退出 |
| **皮肤生态** | 声明式 `skin.json` 零编译热插拔 | 纯文件目录扩展，支持切片图集、台词库、配套音效与对齐锚点自由定义 |

---

## 二、快速开始

### 1. 桌面端启动 (macOS / Windows)

确保本机已安装 [Node.js](https://nodejs.org/) (v18+) 与 [Rust / Cargo](https://rustup.rs/)。

```bash
# 1. 克隆代码仓库
git clone https://github.com/TianYaYou/Anti-pet-indesktop.git
cd Anti-pet-indesktop

# 2. 一键启动 (macOS / Linux)
./start.sh

# Windows 用户双击运行 start.cmd 或在终端中执行：
start.cmd
```

> 首次运行 Cargo 会自动拉取 Wry/Tao/Tray-Icon 依赖并编译桌面二进制，后续二次启动秒开。

编译独立发布版（体积小于 8MB）：
```bash
npm run desktop:build
# 产物位于 desktop/target/release/anti-pet-desktop
```

### 2. 浏览器 / 内置伴侣模式

如果你只希望在浏览器或 IDE 侧边栏内作为一个轻量 Web 伴侣运行：

```bash
npm start
```
打开浏览器访问 [http://127.0.0.1:4199](http://127.0.0.1:4199) 即可直接进行交互与调试。

### 3. 挂载为 Antigravity 官方插件

将当前工程软链接到 Antigravity 插件目录，让 Agent 在对话与执行工具时自动驱动大肥猫状态：

- **macOS / Linux**：
  ```bash
  ln -s "$(pwd)" ~/.gemini/config/plugins/anti-pet-indesktop
  ```
- **Windows (PowerShell 管理员)**：
  ```powershell
  New-Item -ItemType SymbolicLink -Path "$HOME\.gemini\config\plugins\anti-pet-indesktop" -Target (Get-Location)
  ```

重启 Antigravity IDE 后，插件将自动加载。

---

## 三、交互与操作指南

### 滚轮缩放
- **操作方式**：将鼠标移至大肥猫身体上，**滑动滚轮向上放大、向下滑动缩小**。
- **视觉反馈**：猫咪头顶弹出 `🐾 120%` 动态刻度胶囊，缩放完成后自动将偏好尺寸持久化到本机存储。
- **一键重置**：随时在右键控制面板或托盘菜单中点击「重置缩放 (100%)」。

### 手势揉头与摸摸
- **操作方式**：按住鼠标或用光标在大肥猫额头/耳尖区域**来回快速滑动 3 次以上**。
- **反馈效果**：大肥猫进入舒服微闭双眼的享受状态，周围爆发爱心粒子，伴随呼噜呼噜专属声效与亲昵台词。

### 物理抓取与重力吸附
- **抓取悬空**：鼠标左键按住猫猫身体即可抓起，身体呈现悬空晃动姿态，移动方向偏左自动面向左侧、偏右自动面向右侧；
- **重力与落点**：松手后猫猫受虚拟重力下落：
  - **自动模式**：在 macOS 上自动停靠在 Dock 栏顶部平齐位置；全屏无 Dock 时对齐屏幕最底边；
  - **边缘吸附**：拖拽至左右屏幕边缘时自动磁吸附。

### 完全穿透免打扰模式
- **快捷键**：按下 `Cmd + Shift + P`（Windows 为 `Ctrl + Shift + P`），或在右键菜单中开启「完全穿透」。
- **表现**：大肥猫变为半透明幽灵状态，所有鼠标点击直接穿透到背后窗口；右下角常驻微小的唤醒胶囊，点击即可一键恢复。

### 光标注视与输入法感知
- **光标注视**：猫猫头部与眼神会随着全屏幕光标移动产生细微的注视偏转；
- **敲代码避让**：当检测到你在键盘上高频打字输入时，气泡会自动淡出收起，绝不遮挡代码输入视线。

---

## 四、系统托盘管理

大肥猫启动后会自动常驻在系统托盘（macOS 右上角菜单栏 / Windows 右下角任务栏）：

- **左键单击托盘图标**：一键切换桌宠挂件的显示与隐藏；
- **右键托盘图标**：唤出系统托盘上下文菜单：
  - `显示 / 隐藏 大肥猫`
  - `完全穿透 (免打扰模式)`
  - `重置缩放 (100%)`
  - `切换皮肤`
  - `关于 大肥猫桌宠挂件 v0.2.0`
  - `退出`
- **防误触设计**：点击窗口关闭按钮时默认**收拢至托盘后台常驻**，绝不意外丢失挂件状态，只有在托盘菜单点击「退出」才会完全关闭。

---

## 五、自定义皮肤与角色包扩展

换肤系统采用纯文件声明式设计，**添加新皮肤无需重新编译任何 Rust 或 C++ 代码**。

在 `skins/` 目录下新建角色文件夹（例如 `skins/my-hero/`）：

```text
skins/my-hero/
├── skin.json             # 皮肤动作映射、渲染尺寸、对齐锚点
├── frames/               # 高清透明切片序列图
│   ├── idle.png          # 待机发呆
│   ├── blink.png         # 眨眼/按压
│   ├── proud.png         # 得意/任务完成
│   └── sad.png           # 报错委屈
├── audio/                # 按压与交互音效 (可选)
└── dialogue.json         # 专属台词库 (可选)
```

`skin.json` 模板规范：
```json
{
  "schemaVersion": 1,
  "id": "my-hero",
  "name": "自定义小可爱",
  "version": "1.0.0",
  "author": "YourName",
  "render": { "defaultWidth": 160 },
  "frames": {
    "idle": "frames/idle.png",
    "blink": "frames/blink.png",
    "proud": "frames/proud.png",
    "sad": "frames/sad.png"
  },
  "states": {
    "idle": { "frame": "idle", "motion": "breath", "loop": true },
    "thinking": { "frame": "idle", "motion": "ponder", "loop": true },
    "success": { "frame": "proud", "motion": "duang", "durationMs": 3000 },
    "error": { "frame": "sad", "motion": "worry", "durationMs": 4000 }
  }
}
```

放好后在大肥猫右键控制台或托盘菜单点击「切换皮肤」，即可即时生效。

---

## 六、配置目录与隐私声明

| 数据项 | 存储位置 | 说明 |
| :--- | :--- | :--- |
| **本地状态总线** | `.antigravity-pet-state.json` | 仅记录当前事件名称、状态机枚举与气泡台词 |
| **交互用量账本** | `.antigravity-pet-ledger.json` | 仅记录本地 7 天内的交互时间戳与用量统计权重，用于推算 5h/周额度条 |
| **用户偏好设置** | 浏览器 / Webview 本地存储 (`localStorage`) | 记录缩放大小（`anti-pet-scale`）、落点偏好与音效开关 |

> **🔒 隐私原则**：本项目 100% 运行在本机回环网络 (`127.0.0.1`)，**不向任何外部公网服务器上传你的代码、聊天内容、提示词或 API 密钥**。

---

## 七、常见问题 (FAQ)

### Q: macOS 打开提示“应用已损坏”或无法打开？
A: 首次从源码或未签名构建产物启动时，macOS Gatekeeper 安全机制可能会拦截。请在终端执行解除隔离指令：
```bash
xattr -dr com.apple.quarantine desktop/target/release/anti-pet-desktop
```
并在「系统设置 → 隐私与安全性」中选择允许。

### Q: 为什么有时候按着大肥猫拖不动？
A: 请确保未开启「完全穿透（免打扰模式）」。若开启了免打扰，大肥猫会呈现半透明幽灵状态，点击屏幕右下角的唤醒药丸按钮即可恢复正常交互。

### Q: 额度显示百分比是实时服务器数据吗？
A: 本项目通过与 Antigravity 本地语言服务通信及本机 7 天滚动账本协同推算，反映高频会话周期内的相对消耗，无需配置私有 API Key 或抓包凭证。

---

## 八、项目致谢与开源许可

本项目采用 [MIT License](LICENSE) 开源协议。

### 致敬与灵感来源：
- [dsh-pet-indesktop](https://github.com/MerZlin/dsh-pet-indesktop)：优秀的跨平台桌宠生态与工程实践，启发了本项目在开源文档规范、桌面物理落点与多形态交互上的设计；
- [Antigravity-Star-Suite](https://github.com/Monokumakuki/Antigravity-Star-Suite)：感谢 Monokumakuki 在 Antigravity 宿主额度感知与 Gemini 猫娘原设素材上的灵感启迪；
- [Tauri & Wry Team](https://github.com/tauri-apps/wry)：赋予极简微内核原生透明穿透窗口的能力。
