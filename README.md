# 大肥猫桌宠挂件 (Anti-Pet In Desktop)

<p align="center">
  <img src="assets/icon.png" width="160" alt="大肥猫桌宠挂件应用图标" style="border-radius: 36px; box-shadow: 0 16px 36px rgba(0,0,0,0.2);">
</p>

<p align="center">
  <a href="https://github.com/TianYaYou/Anti-pet-indesktop/releases"><img alt="版本" src="https://img.shields.io/badge/%E7%89%88%E6%9C%AC-v0.5.1-blue?style=flat-square"></a>
  <a href="https://github.com/TianYaYou/Anti-pet-indesktop/blob/main/LICENSE"><img alt="开源许可证" src="https://img.shields.io/badge/License-MIT-orange?style=flat-square"></a>
  <img alt="平台支持" src="https://img.shields.io/badge/%E5%B9%B3%E5%8F%B0-macOS%20%7C%20Windows%20%7C%20Linux-8A2BE2?style=flat-square">
  <img alt="内存占用" src="https://img.shields.io/badge/%E5%86%85%E5%AD%98%E5%8D%A0%E7%94%A8-~12MB%20RSS-brightgreen?style=flat-square">
  <img alt="技术栈" src="https://img.shields.io/badge/%E6%9E%B6%E6%9E%84-Pure%20CPU%202D%20%2B%20AppKit%20%2F%20Slint-ff69b4?style=flat-square">
  <a href="https://github.com/TianYaYou/Anti-pet-indesktop/issues"><img alt="Issues" src="https://img.shields.io/github/issues/TianYaYou/Anti-pet-indesktop?style=flat-square"></a>
</p>

<p align="center">
  <b>面向 Google Antigravity IDE 与现代桌面的极轻量、跨平台半身大肥猫伴侣桌宠</b><br>
  纯 CPU 2D 软渲染引擎 · 双原生偏好设置 (macOS AppKit / Windows Fluent 2 Slint) · 亚微秒级 Alpha 穿透 · Agent 实时状态共鸣 · Gemini 双周期额度胶囊
</p>

---

> ### v0.5.1 (NextGen UI & Pure CPU 2D Core) 里程碑升级说明
>
> - **纯 CPU 2D 软光栅渲染引擎 (Zero WebKit / 0% GPU)**：彻底摆脱重型 WebView 运行时，基于预乘 RGBA8888 帧缓冲 (`Pixmap`)、Box-Filtered Mipmap 双线性插值与 SDF 抗锯齿矢量卡片绘制，运行态常驻内存仅 **~12 MB RSS**；
> - **下一代双原生偏好设置架构 (NextGen Dual-Native UI)**：
>   - **macOS 原生 AppKit 控制器**：100% 遵循 Apple Human Interface Guidelines (HIG)，采用 `NSVisualEffectView` 原生毛玻璃侧栏、Inset Grouped 分组卡片、SF Symbols 矢量图标、`NSPopUpButton` 与 `NSSwitch` 原生控件（0 KB 额外 UI 依赖）；
>   - **Windows / Linux Slint Fluent Design 2 界面**：采用现代 Fluent Design 2 视觉规范、SVG 导航图标、分组卡片、实时皮肤预览与变更即刻自动保存 (`Auto-Save`)；
>   - **无头状态机契约 (`SettingsStore`)**：抽离跨平台统一的 `SettingsStore` 领域状态机，并通过临时文件原子重命名 (`Write-to-Temp & Atomic Rename`) 保障断电或强退时配置零损坏；
> - **原生 Rust 额度探针与有界环形账本**：内置 `NativeQuotaProber` 异步后台线程，直接发现本机 Antigravity `language_server` 进程并通过 Connect-RPC 同步 5h / 周额度与重置倒计时；本地降级账本支持 500 条 / 7 天滚动环形截断；
> - **亚微秒级像素 Alpha 穿透 (`< 1μs` Hit-Testing)**：直接读取内存帧缓冲 Alpha 通道 (`alpha >= 20`) 动态切换鼠标穿透，支持免打扰模式下右键猫咪直接唤起原生 `muda` 上下文菜单；
> - **多架构发布矩阵与自动化内存泄漏审计**：覆盖 `macOS-arm64`、`windows-x64`、`windows-arm64` 与 `linux-x64`，内置 500 轮连续光栅化零泄漏 CI 审计与零 Emoji 代码规范检查。

---

## 目录

- [一、功能特性概览](#一功能特性概览)
- [二、快速开始](#二快速开始)
  - [1. 桌面端启动 (macOS / Windows / Linux)](#1-桌面端启动-macos--windows--linux)
  - [2. 偏好设置与调试启动模式](#2-偏好设置与调试启动模式)
  - [3. 浏览器 / Web 伴侣模式](#3-浏览器--web-伴侣模式)
  - [4. 挂载为 Antigravity 官方插件](#4-挂载为-antigravity-官方插件)
- [三、交互与操作指南](#三交互与操作指南)
- [四、系统托盘与原生右键菜单](#四系统托盘与原生右键菜单)
- [五、自定义皮肤与台词热重载](#五自定义皮肤与台词热重载)
- [六、架构设计与开发文档](#六架构设计与开发文档)
- [七、配置目录与隐私声明](#七配置目录与隐私声明)
- [八、常见问题 (FAQ)](#八常见问题-faq)
- [九、项目致谢与开源许可](#九项目致谢与开源许可)

---

## 一、功能特性概览

| 模块 | 核心能力 | 体验细节 |
| :--- | :--- | :--- |
| **纯 CPU 2D 渲染层** | 全透明无边框、跨工作区置顶 (`Topmost`) | macOS `CALayer` / Windows `UpdateLayeredWindow` 原生透明合成，无黑边无白框 |
| **双原生偏好设置** | macOS AppKit HIG + Windows Slint Fluent 2 | 5 大配置分区、实时皮肤预览、台词分类增删改、控件交互即刻自动持久化 |
| **弹性物理引擎** | 60 FPS 弹簧阻尼、抛投重力下落、边缘探头 | 拖拽随速度方向自动转身倾斜；支持贴合 Dock/任务栏上沿、0px 极底或自由悬浮 |
| **情感手势识别** | 头部往返抚摸 (`Petting`)、点按 `Duang` 回弹 | 光标在猫头区域快速往返滑动触发呼噜享受动画与专属傲娇台词 |
| **IDE 状态共鸣** | 实时感知 Antigravity Agent 执行生命周期 | 思考中 (`Thinking`) / 调用工具 (`Working`) / 完成 (`Success`) / 异常 (`Error`) |
| **双周期用量监控** | Gemini 5小时与每周双额度进度条 + 倒计时 | 异步探测本机 `language_server` 实时额度，离线时自动回退至 500 条有界本地账本 |
| **打字防遮挡感知** | 全局键盘输入检测与自动收起 | 检测到用户高频敲代码或窗口失焦时，思考云朵气泡自动隐藏，绝不遮挡视线 |
| **原生菜单与音效** | `muda` 原生右键/托盘菜单 + 内嵌 WAV 音效 | 支持按需勾选/隐藏右键菜单项，内置清脆按压与回弹音频反馈 |

---

## 二、快速开始

### 1. 桌面端快速安装与启动

#### 方式 A：macOS Homebrew Cask 一键安装 (推荐)

```bash
# 添加官方 Tap 并一键安装
brew tap TianYaYou/tap
brew install --cask anti-pet

# 或者直接通过仓库 Cask 文件安装
brew install --cask Casks/anti-pet.rb
```

#### 方式 B：macOS DMG 拖拽安装

前往 [Releases 页面](https://github.com/TianYaYou/Anti-pet-indesktop/releases) 下载最新 `Anti-Pet-vX.Y.Z-macOS-arm64.dmg`，双击打开后将 `Anti-Pet.app` 拖入 `Applications` 文件夹即可完成安装。

#### 方式 C：源码编译与一键脚本 (macOS / Windows / Linux)

确保本机已安装 [Rust / Cargo](https://rustup.rs/)（如需 Web 预览或 Hook 辅助服务可同时安装 [Node.js](https://nodejs.org/) v18+）。

```bash
# 1. 克隆代码仓库
git clone https://github.com/TianYaYou/Anti-pet-indesktop.git
cd Anti-pet-indesktop

# 2. 一键启动默认桌面端 (Pure CPU 2D + NextGen UI)
./start.sh

# Windows 用户双击运行 start.cmd 或在终端中执行：
start.cmd
```

直接通过 Cargo 编译并运行独立原生二进制（无需启动 Node.js 服务）：
```bash
cargo run --release --bin anti-pet-desktop --manifest-path desktop/Cargo.toml
```

本地打包 macOS 原生应用包 (`.app`) 与安装镜像 (`.dmg`)：
```bash
# 构建 dist/Anti-Pet.app
./scripts/build_macos_app.sh

# 构建标准拖拽安装镜像 dist/Anti-Pet-macOS.dmg
./scripts/build_macos_dmg.sh
```

### 2. 偏好设置与调试启动模式

`start.sh` 与 `anti-pet-desktop` 支持多种启动参数，便于快速预览不同前端：

```bash
# 直接打开 macOS 原生 AppKit 偏好设置窗口 (Apple HIG 毛玻璃风格)
./start.sh --appkit

# 直接打开 Slint Fluent Design 2 现代偏好设置窗口 (跨平台预览模式)
./start.sh --slint

# 启动旧版 WebKit 引擎进行回退对比测试
./start.sh --legacy
```

### 3. 浏览器 / Web 伴侣模式

如果你希望在浏览器或 IDE 内置 Webview 中作为伴侣运行：

```bash
./start.sh --web
# 或直接运行：
npm start
```
打开浏览器访问 [http://127.0.0.1:4199](http://127.0.0.1:4199)（设置页位于 [http://127.0.0.1:4199/settings.html](http://127.0.0.1:4199/settings.html)）。

### 4. 挂载为 Antigravity 官方插件

将当前工程软链接到 Antigravity 插件目录，让 Agent 在对话与执行工具时通过 [`hooks.json`](hooks.json) 自动驱动大肥猫状态：

- **macOS / Linux**：
  ```bash
  ln -s "$(pwd)" ~/.gemini/config/plugins/anti-pet-indesktop
  ```
- **Windows (PowerShell 管理员)**：
  ```powershell
  New-Item -ItemType SymbolicLink -Path "$HOME\.gemini\config\plugins\anti-pet-indesktop" -Target (Get-Location)
  ```

---

## 三、交互与操作指南

- **滚轮无级缩放 (`50% ~ 200%`)**：将鼠标悬停在大肥猫身体上滚动滚轮，即可平滑放大或缩小体型，并在气泡中实时显示当前缩放百分比，400ms 防抖后自动持久化。
- **手势摸头 (`Petting`)**：在猫咪额头与耳尖区域左右往返滑动鼠标，触发眯眼享受动画与 `petting` 分类台词。
- **拖拽抛投、落点基准与边缘探头**：
  - 按住左键拖拽猫咪时，身体随水平速度方向自动转身并产生动态倾斜；
  - 松手后支持三种落点模式（可在偏好设置或右键菜单随时切换）：**贴合任务栏与 Dock 上沿 (`taskbar`)**、**屏幕绝对落底 (`bottom`)**、**自由悬浮 (`free`)**；
  - 将猫咪拖至屏幕左右边缘松手，可触发半身探头磁吸效果。
- **完全穿透免打扰模式 (`DND Passthrough`)**：开启后猫咪呈现柔和半透明状态，左键点击完全穿透至底层代码编辑器；**只需在猫咪身上点击鼠标右键**，即可随时唤出原生菜单关闭免打扰。

---

## 四、系统托盘与原生右键菜单

在猫咪身上点击右键或点击系统托盘图标，即可唤出由 `muda` 驱动的原生系统菜单（所有菜单项均可在「偏好设置 → 右键菜单配置」中自由开启或隐藏）：

- `偏好设置...`（唤起 macOS AppKit 或 Windows Slint 原生设置中心）
- `复位居中`（无条件常驻，一键将迷航的桌宠拉回屏幕正中央）
- `换一句傲娇台词` / `摸摸猫猫头`
- `立即刷新 Antigravity 额度` / `气泡内显示额度条`
- `免打扰模式 (右键猫咪可随时退出)`
- `启用物理重力掉落` 与 `掉落落点基准` 子菜单
- `猫咪体型缩放` (`80%` / `100%` / `125%` / `150%`)
- `启用 Q 弹互动音效`
- `显示 / 隐藏 大肥猫` / `唤起 Antigravity IDE`（当检测到 Antigravity 已在运行时自动隐藏重复唤起项）
- `大肥猫桌宠 v0.5.1 (Pure CPU 2D)` / `退出`

---

## 五、自定义皮肤与台词热重载

所有皮肤与台词资源统一收敛在根目录 [`skins/`](skins)（单一事实来源，如 [`skins/fat-cat/`](skins/fat-cat)）：

```text
skins/fat-cat/
├── skin.json             # 皮肤渲染尺寸与动作状态映射
├── dialogue.json         # 五大状态分类台词库 (idle / petting / thinking / success / error)
├── frames/
│   └── idle.png          # 高清透明猫咪立绘贴图
├── audio/                # 互动 WAV 音效 (pop-press / pop-release / click-press / click-release)
└── ui/                   # 气泡装饰 SVG 资源
```

你可以在「偏好设置 → 台词与问候」中直接按分类（待机日常、互动抚摸、深度思考、任务成功、任务遇阻）新增或删除台词，修改结果会实时自动保存至 `skins/fat-cat/dialogue.json`。

---

## 六、架构设计与开发文档

关于 `v0.5.1` 的模块分层、无头 `SettingsStore` 状态机契约、双原生 UI 桥接机制、跨平台编译构建与自动化测试规范，请查阅项目内置的开发者文档：

- **[DEVELOPMENT.md — 架构设计与开发指南](DEVELOPMENT.md)**

常用研发与质量验证指令：
```bash
# 运行全部 48 项 Rust 单元测试与双前端状态契约测试
cargo test --manifest-path desktop/Cargo.toml

# 运行跨平台 500 轮连续渲染内存占用与零泄漏审计
node scripts/audit_memory_leak.js

# 运行零 Emoji 代码规范审计
node scripts/audit_zero_emojis.js
```

---

## 七、配置目录与隐私声明

| 数据文件 | 默认存储策略 | 持久化与容量安全机制 |
| :--- | :--- | :--- |
| `.antigravity-pet-settings.json` | 项目根目录及用户主目录 (`~/.antigravity-pet-settings.json`) | 采用临时文件写入 + 原子重命名 (`Atomic Rename` + NTFS Copy Fallback)，杜绝断电损坏 |
| `.antigravity-pet-state.json` | 项目根目录或用户主目录 | 记录最新一次 Agent 生命周期事件、动作状态与实时额度快照 |
| `.antigravity-pet-ledger.json` | 项目根目录或用户主目录 | 严格限制最多保留 **最近 500 条且在 7 天窗口内** 的交互权重记录，防止文件膨胀 |

> **隐私承诺**：本程序 100% 在本机回环地址 (`127.0.0.1`) 内运行，仅读取本机 Antigravity 语言服务额度摘要与状态事件，绝不向任何外部服务器上传代码、对话内容或密钥。

---

## 八、常见问题 (FAQ)

### Q: macOS 打开 `Anti-Pet.app` 提示无法验证或已损坏？
A: 从源码编译或下载未公证构建包时，可在终端执行以下命令移除隔离属性：
```bash
xattr -dr com.apple.quarantine dist/Anti-Pet.app
```

### Q: 开启「免打扰模式」后左键点不动猫咪怎么办？
A: 免打扰模式下左键点击会 100% 穿透给下层窗口。只需**在猫咪图案上点击鼠标右键**，在弹出的原生菜单中取消勾选「免打扰模式」，或在设置窗口中关闭该开关即可恢复。

---

## 九、项目致谢与开源许可

本项目采用 [MIT License](LICENSE) 开源协议。

- [dsh-pet-indesktop](https://github.com/MerZlin/dsh-pet-indesktop)：启发了跨平台桌宠物理落点与皮肤兼容导入设计；
- [Antigravity-Star-Suite](https://github.com/Monokumakuki/Antigravity-Star-Suite)：提供了 Antigravity 本地额度探测思路与角色灵感；
- [Tao](https://github.com/tauri-apps/tao)、[Muda](https://github.com/tauri-apps/muda) 与 [Slint](https://slint.dev/)：提供了跨平台窗口事件循环、原生菜单与声明式 GUI 支持。
