# Anti-Pet In Desktop (v0.5.1) 架构设计与开发者指南

本文档面向参与 `Anti-pet-indesktop` 研发与维护的工程师，详细阐述 `v0.5.1 (Pure CPU 2D + NextGen Dual-Native UI)` 的系统架构、核心模块职责、跨平台一致性契约、数据持久化规范以及 CI/CD 质量门禁。

---

## 一、系统总体架构

自 `v0.3.0` ~ `v0.5.1` 重构以来，项目从早期的 `WebKit (wry) + Node.js` 双进程重型架构全面演进为 **纯 CPU 2D 软光栅主引擎 + 无头领域状态机 (`SettingsStore`) + 平台原生双前端设置中心** 的极简架构：

```text
Anti-pet-indesktop/
├── desktop/                          # Rust 原生桌面客户端主工程 (Crate: anti-pet-desktop v0.5.1)
│   ├── Cargo.toml                    # 编译特性 (default = ["cpu-2d"], 可选 "legacy-webkit")
│   ├── build.rs                      # Slint 编译脚本与 Windows 资源图标嵌入
│   ├── src/
│   │   ├── main.rs                   # 60 FPS 主事件循环、跨平台透明窗口管理、托盘与命令行路由
│   │   ├── lib.rs                    # 核心库导出入口 (供集成测试、契约测试与内存审计调用)
│   │   ├── core/
│   │   │   └── store.rs              # 无头设置状态机 (SettingsStore)、领域动作 (SettingsAction) 与原子持久化
│   │   ├── ui/
│   │   │   ├── macos/mod.rs          # macOS 原生 Cocoa/AppKit 设置控制器 (Apple HIG 规范，0 额外依赖)
│   │   │   └── windows/              # Windows/Linux Slint Fluent Design 2 设置前端与 .slint 声明文件
│   │   ├── renderer/
│   │   │   ├── mod.rs                # RGBA8888 预乘像素帧缓冲 (Pixmap)、SDF 图元光栅化与窗口提交 (SoftwareSurface)
│   │   │   ├── sprite.rs             # Box-Filtered Mipmap 贴图采样与 2D 仿射变换矩阵 (挤压、回弹、视线偏移)
│   │   │   ├── bubble.rs             # 思考云朵气泡 (o -> O -> 云朵卡片)、屏幕边缘防裁剪与双额度进度条
│   │   │   ├── text.rs               # 跨平台原生文字光栅化 (CoreText / GDI / Pango) 与光学垂直居中引擎
│   │   │   └── settings_ui.rs        # 内置轻量纯 2D 软光栅设置回退渲染器
│   │   ├── physics.rs                # 60 FPS 弹簧阻尼 (Spring-Damper)、重力抛投、Dock/任务栏贴合与摸头手势识别
│   │   ├── hit_test.rs               # < 1μs 内存级像素 Alpha 通道碰撞检测与免打扰右键穿透控制
│   │   ├── quota.rs                  # 原生 Antigravity language_server 进程发现、Connect-RPC 探测与有界账本解析
│   │   ├── state_watcher.rs          # 配置文件 (.antigravity-pet-settings.json) 与状态总线 (.antigravity-pet-state.json) 监听
│   │   ├── menu_audio.rs             # Muda 原生右键/托盘菜单管理、台词库 (DialogueBank) 与内嵌 WAV 音效播放
│   │   ├── bin/slint_settings.rs     # 独立 Slint 设置界面调试入口二进制
│   │   └── legacy_webkit.rs          # 旧版 WebKit 兼容层 (需通过 --features legacy-webkit 显式启用)
│   └── tests/
│       └── test_settings_store_contract.rs # 双前端状态机一致性契约测试套件
├── skins/                            # 皮肤与台词资产唯一事实来源 (Single Source of Truth)
│   └── fat-cat/                      # 默认大肥猫皮肤 (skin.json, dialogue.json, frames/, audio/, ui/)
├── server/                           # 辅助 Node.js 本地 Hub 与 Antigravity Hook 处理器
│   ├── index.js                      # 本地 4199 端口 HTTP/SSE 服务 (供 --web 模式与外部扩展使用)
│   ├── hook-handler.js               # IDE 生命周期 Hook 处理器与 500 条有界环形账本写入器
│   ├── quota-probe.js                # Node 端 Antigravity 额度探测模块
│   └── transcript-watcher.js         # 会话转录状态监听模块
├── core/                             # Web 伴侣模式前端资源 (index.html, pet.js, settings.html, settings.js)
└── scripts/                          # 自动化质量门禁与打包脚本
    ├── audit_memory_leak.js          # 500 轮连续软光栅内存占用与零泄漏 CI 审计脚本
    ├── audit_zero_emojis.js          # 源码零 Emoji 规范静态扫描脚本
    ├── build_macos_app.sh            # macOS 原生 Anti-Pet.app (v0.5.1) 独立应用包构建脚本
    ├── start.sh                      # 转发至根目录 ./start.sh
    └── start.bat                     # 转发至根目录 .\start.cmd
```

---

## 二、核心子系统设计详解

### 1. 纯 CPU 2D 软光栅渲染管线 (`desktop/src/renderer/`)

为彻底消除 WebView 进程动辄 150MB+ 的内存占用与 GPU 驱动兼容性问题，主窗口采用纯 CPU 软光栅设计：

1. **预乘 RGBA8888 帧缓冲 (`Pixmap`)**：
   - 窗口逻辑尺寸固定为 `340 × 400` (`COMPACT_WIN_W × COMPACT_WIN_H`)，物理像素尺寸按显示器 DPI 缩放因子 (`scale_factor`) 动态分配。
   - 提供基于有向距离场 (SDF) 的圆角矩形 (`fill_rounded_rect_sdf`)、抗锯齿圆 (`fill_circle_sdf`)、线段 (`draw_line_sdf`) 以及硬件级视口裁剪 (`set_clip_rect`)。
2. **多级 Mipmap 精灵贴图变换 (`sprite.rs`)**：
   - 加载 `skins/fat-cat/frames/idle.png` 时预计算 Box-Filtered Mipmap 金字塔；在 `50% ~ 200%` 无级缩放与弹簧形变过程中，自动选取匹配的 Mip 层级并执行双线性插值与逆向仿射矩阵映射，消除缩小时的摩尔纹与锯齿。
3. **跨平台零拷贝透明窗口提交 (`SoftwareSurface`)**：
   - **macOS**：通过 `CGDataProvider` 与 `CGImage` 将 `Pixmap` 缓冲区直接挂载至 `NSWindow` 根视图的 `CALayer.contents`，设置窗口层级为 `100`（高于 Dock 的 `20` 与状态栏的 `25`，低于原生右键菜单 `NSPopUpMenuWindowLevel = 101`）。
   - **Windows**：通过 Win32 `CreateDIBSection` 与 `UpdateLayeredWindow` 将预乘 Alpha 帧直接提交给 DWM 合成器，并调用 `DwmSetWindowAttribute` 移除窗口边框与圆角阴影残留。
   - **Linux**：基于 GTK/GDK/Cairo 透明 Visual 绘制提交。

### 2. 无头状态机与双原生设置前端 (`core/store.rs` & `ui/`)

在 `v0.5.1` 中，设置窗口被解耦为**一套核心领域状态机 + 两套平台原生视图层**：

- **领域状态机 (`SettingsStore`)**：
  - 统一管理 `temp_settings`、`original_settings`、`dialogue_bank` 与选中标签页 `active_tab`。
  - 所有前端交互均转化为强类型的 `SettingsAction`（如 `SetScale(f32)`、`CycleTheme`、`SetFloorMode(String)`、`ToggleMenuOption(usize)`、`AddDialogue(String)`、`DeleteDialogue(usize)`），由 `SettingsStore::dispatch` 执行统一的边界钳制（如缩放比例严格限制在 `0.50 ..= 2.00` 并按 `0.05` 步进对齐）。
- **macOS 原生前端 (`desktop/src/ui/macos/mod.rs`)**：
  - 当在 macOS 上点击右键菜单「偏好设置...」或运行 `./start.sh --appkit` 时启动。
  - 使用 `objc2` 运行时构建原生 `NSWindow`、`NSVisualEffectView` (Material = Sidebar)、`NSScrollView`、`NSPopUpButton`、`NSSlider`、`NSSwitch` 及 SF Symbols 图标，通过动态注册的 `AntiPetMacosActionTarget` Objective-C 类集中分发 Target-Action 事件并实时调用 `commit_to_disk()`。
- **Windows / Linux Slint 前端 (`desktop/src/ui/windows/`)**：
  - 在 Windows/Linux 上点击「偏好设置...」或运行 `./start.sh --slint` 时启动。
  - 基于 `desktop/src/ui/windows/ui/settings.slint` 实现 Fluent Design 2 风格界面，所有回调在 `SlintSettingsApp::bind_callbacks` 中映射到 `SettingsStore::dispatch` 并自动保存。

### 3. 原子文件持久化与有界账本机制

为解决多进程（主程序、独立设置子进程、Node Hook 进程）并发读写配置文件与账本膨胀问题，`v0.5.1` 实施了三项存储加固：

1. **设置文件原子写入 (`Write-to-Temp & Atomic Rename`)**：
   - `SettingsStore::commit_to_disk`、`NativeStateWatcher::save_settings` 与 `server/index.js` 的 `saveSettings` 均先将完整 JSON 写入临时文件 (`.antigravity-pet-settings.json.tmp`)，随后调用 `fs::rename` 原子替换目标文件。
   - 针对 Windows NTFS 文件系统在目标文件被读取句柄短暂锁定时 `rename` 可能返回 `ERROR_ACCESS_DENIED` 的特性，内置了 `std::fs::copy` + `std::fs::remove_file` 降级保护。
2. **双路径同步**：
   - 当检测到程序运行在工程目录（存在 `skins/` 子目录）时，设置变更会同时原子同步至工程根目录与用户主目录 `~/.antigravity-pet-settings.json`，确保源码调试与打包运行配置一致。
3. **500 条 / 7 天有界环形账本 (`Bounded Ledger`)**：
   - `server/hook-handler.js` 与 `server/index.js` 在记录每次工具调用权重时，自动过滤超过 7 天 (`ONE_WEEK_MS`) 的过期条目，并通过 `.slice(-500)` 将 `.antigravity-pet-ledger.json` 严格限制在 500 条以内（文件体积稳定在 `< 45 KB`）。
   - `desktop/src/quota.rs` 中的 `parse_ledger_quota_json` 对负数时间戳、`NaN` 或非有限浮点数权重进行了防御性过滤，确保额度百分比始终收敛于 `[5.0%, 100.0%]`。

---

## 三、本地开发、调试与构建流程

### 1. 环境依赖
- **Rust 工具链**：建议使用 `rustup` 安装最新 `stable` 工具链（2021 Edition）。
- **Linux 系统依赖**（仅 Ubuntu/Debian 编译时需要）：
  ```bash
  sudo apt-get install -y libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
  ```
- **Node.js**：v18 或 v20+（用于运行 CI 审计脚本、Hook 处理器或 `--web` 预览服务）。

### 2. 编译与调试命令速查

| 场景 | 执行命令 | 说明 |
| :--- | :--- | :--- |
| **启动主程序 (Release)** | `cargo run --release --bin anti-pet-desktop --manifest-path desktop/Cargo.toml` | 默认启动纯 CPU 2D 桌宠挂件 |
| **调试 macOS AppKit 设置** | `cargo run --bin anti-pet-desktop --manifest-path desktop/Cargo.toml -- --appkit` | 独立唤起 macOS 原生 Cocoa 设置窗口 |
| **调试 Slint Fluent 2 设置** | `cargo run --bin slint_settings --manifest-path desktop/Cargo.toml` | 独立唤起跨平台 Slint 设置窗口 |
| **打包 macOS `.app` 应用** | `./scripts/build_macos_app.sh` | 在 `dist/Anti-Pet.app` 生成独立应用包 |
| **运行全量 Rust 测试** | `cargo test --manifest-path desktop/Cargo.toml` | 运行 48 项单元、集成与双前端契约测试 |
| **运行内存与泄漏审计** | `node scripts/audit_memory_leak.js` | 验证 500 轮连续渲染 RSS `< 300MB`、Heap `< 100MB` |
| **运行零 Emoji 规范检查** | `node scripts/audit_zero_emojis.js` | 扫描 `core/` 与 `desktop/src/` 确保无 Emoji 字符 |

---

## 四、测试体系与 CI/CD 流水线规范

### 1. 自动化测试分层 (`48/48 Tests`)
- **核心库单元测试 (`desktop/src/lib.rs` -> `22 passed`)**：覆盖 `SettingsStore` 缩放步进钳制、主题与落点循环、脏状态回滚、临时目录沙箱原子写入（不污染工作区）、ISO-8601 时间戳解析、Antigravity 实时额度 JSON 解析及账本异常边界容错。
- **主程序单元测试 (`desktop/src/main.rs` -> `22 passed`)**：覆盖弹簧阻尼物理引擎、轻放单次 `Duang` 回弹、重抛二次弹跳衰减、Dock 高度贴合、像素 Alpha 碰撞检测与 UI 交互命中测试。
- **双前端契约测试 (`desktop/tests/test_settings_store_contract.rs` -> `4 passed`)**：验证 macOS AppKit 与 Windows Slint 共享的所有 `SettingsAction` 状态转移、9 项右键菜单开关、台词 CRUD 与取消回滚语义严格一致。

### 2. GitHub Actions 工作流
- **持续集成 (`.github/workflows/ci.yml`)**：
  1. `Code Quality & Zero-Emoji Audit`：校验 JS 语法、`package.json` / `plugin.json` 格式并执行 `scripts/audit_zero_emojis.js`。
  2. `Rust Compilation Check`：在 `macos-latest`、`ubuntu-latest`、`windows-latest` 三端执行 `cargo check`。
  3. `Multi-Platform Memory Footprint & Leak Audit`：在三端执行 `node scripts/audit_memory_leak.js`（仅针对 `--lib` 核心渲染循环执行 500 轮压测，避免在无头 CI 中编译多二进制超时）。
- **多平台发布 (`.github/workflows/release.yml`)**：
  - 推送 `v*` 标签或手动触发时，并行构建 4 个目标：`aarch64-apple-darwin` (`macOS-arm64`)、`x86_64-pc-windows-msvc` (`windows-x64`)、`aarch64-pc-windows-msvc` (`windows-arm64`) 与 `x86_64-unknown-linux-gnu` (`linux-x64`)，策略配置为 `fail-fast: false`。

---

## 五、代码贡献与发版检查清单 (Release Checklist)

在向 `dev` 或 `main` 分支提交 PR 或打标签发版前，请确保满足以下全部条件：

1. **零 Emoji 铁律**：`core/` 与 `desktop/src/` 下的所有 `.rs`、`.js`、`.html`、`.css` 文件严禁包含任何 Unicode Emoji 字符（所有图标必须使用 SF Symbols、SVG 矢量路径或纯文本符号），提交前必须通过 `node scripts/audit_zero_emojis.js`。
2. **单一皮肤事实来源**：所有内置皮肤、音效与台词修改必须在根目录 `skins/fat-cat/` 下进行，严禁在 `desktop/` 下重建冗余的 `desktop/skins/` 目录。
3. **版本号全链路同步**：升级版本号时，必须同步更新以下位置：
   - [`package.json`](package.json) 与 [`plugin.json`](plugin.json)
   - [`desktop/Cargo.toml`](desktop/Cargo.toml)
   - [`scripts/build_macos_app.sh`](scripts/build_macos_app.sh) (`CFBundleVersion` / `CFBundleShortVersionString`)
   - [`desktop/src/ui/macos/mod.rs`](desktop/src/ui/macos/mod.rs)、[`desktop/src/ui/windows/ui/settings.slint`](desktop/src/ui/windows/ui/settings.slint)、[`desktop/src/ui/windows/mod.rs`](desktop/src/ui/windows/mod.rs)
   - [`desktop/src/main.rs`](desktop/src/main.rs)、[`desktop/src/menu_audio.rs`](desktop/src/menu_audio.rs)、[`desktop/src/renderer/settings_ui.rs`](desktop/src/renderer/settings_ui.rs)
   - [`core/settings.html`](core/settings.html) 与 [`server/index.js`](server/index.js)
4. **测试无脏文件残留**：任何新增的持久化单元测试必须在 `std::env::temp_dir()` 下创建独立沙箱目录并在测试结束时清理，不得向仓库根目录或 `desktop/` 目录写入测试文件。
