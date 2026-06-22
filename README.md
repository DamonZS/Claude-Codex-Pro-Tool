# Claude Codex Pro Tool

<p align="center">
  <img src="docs/images/claude-codex-pro.png" alt="Claude Codex Pro Tool 图标" width="160">
</p>

<p align="center">
  中文 | <a href="README_EN.md">English</a>
</p>

<p align="center">
  <img alt="Release" src="https://img.shields.io/github/v/release/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="Stars" src="https://img.shields.io/github/stars/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="License" src="https://img.shields.io/github/license/DamonZS/Claude-Codex-Pro-Tool">
  <img alt="Rust" src="https://img.shields.io/badge/rust-1.85%2B-orange">
  <img alt="Tauri" src="https://img.shields.io/badge/tauri-2.x-24C8DB">
</p>

Claude Codex Pro Tool 是面向 Codex App 与 Claude Desktop 的本地运维控制台。它提供 Codex 启动增强、Claude 中文包装窗口、Claude Desktop MCP 安装、插件中心、供应商配置、会话维护、脚本管理、提示词优化和更新诊断等能力。

项目的安全边界很明确：不修改 Codex 或 Claude 官方安装目录，不改 `app.asar`，不改签名或完整性文件；所有增强都通过外部启动器、本地配置、WebView 包装窗口或可审查的配置写入完成。

## 下载与入口

从 [GitHub Releases](https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases) 下载最新安装包：

- Windows：`claude-codex-pro-*-windows-x64-setup.exe`
- macOS Intel：`claude-codex-pro-*-macos-x64.dmg`
- macOS Apple Silicon：`claude-codex-pro-*-macos-arm64.dmg`

安装后会出现两个入口：

- `Claude Codex Pro`：静默启动器，用来启动 Codex 并加载本工具的增强能力。
- `Claude Codex Pro 管理工具`：Tauri 运维控制台，用来管理 Codex、Claude、供应商、插件、脚本、日志、安装维护和更新。

Windows 安装包会创建桌面和开始菜单快捷方式。macOS DMG 会安装 `Claude Codex Pro.app` 与 `Claude Codex Pro 管理工具.app`。

## 核心能力

- Codex 增强：通过 CDP 和本地 helper 注入状态标识、快捷菜单、会话工具、用户脚本和增强入口。
- Claude 中文窗口：在独立 WebView 中加载 `https://claude.ai/new`，创建窗口时注入中文覆盖脚本和顶部状态标识。
- Claude Desktop 集成：启动官方 Claude Desktop，并将 MCP 配置写入 Claude Desktop 的用户配置文件。
- 插件中心：展示官方 Claude 插件、GitHub MCP、Claude Code 资源、Skills 和社区资源；安装前展示命令或配置 diff。
- 供应商配置：管理兼容 API、中转配置、模型、上下文选择和 Codex provider 写入。
- 会话维护：支持会话删除、恢复、Markdown 导出、项目移动、时间线和本地数据诊断。
- 脚本市场：管理内置脚本、本地用户脚本和远程脚本目录。
- 提示词优化：集成 `linshenkx/prompt-optimizer` 的提示词优化工作流。
- Provider Sync：切换供应商后保持历史会话可见。
- Zed Remote：识别 SSH 场景并从 Codex 打开对应远程文件到 Zed。
- Upstream worktree：从最新远程跟踪分支创建 worktree，避免从过期本地 HEAD 派生。
- 安装维护：提供日志、诊断、修复、版本检查和 GitHub Release 更新。

## Claude Desktop

管理工具会把 Claude 与 Codex 的操作分开：

- `启动 Claude`：启动官方 Claude Desktop，不修改它的安装文件。
- `打开 Claude 中文窗口`：打开独立 WebView 包装窗口，加载 Claude 网页并应用中文覆盖。
- `重启 Codex`：只控制 Codex 增强启动器。
- `插件中心`：在管理工具内部跳转，不额外弹出重复控制窗口。

Claude 中文窗口不是对官方 Claude Desktop 原窗口做 DOM 注入。官方桌面端的 MSIX、签名和完整性校验会阻止这类高风险改造，因此本项目采用安全包装窗口：用户在包装窗口中登录 Claude 网页，中文覆盖脚本只作用于该窗口。

## 插件中心

插件中心统一展示不同来源的能力：

- 官方 Claude 插件市场条目。
- GitHub MCP 和社区 MCP 资源。
- Awesome Claude Code 资源。
- 可识别结构的 Skill bundle。
- Claude Desktop MCP 配置项，包括 Codex 相关 MCP。

安装流程遵循可审查原则：

1. 刷新目录，查看来源、类型、许可证、风险提示和安装状态。
2. 点击预览，检查将执行的命令或将写入的配置 diff。
3. 确认后安装；写入配置前会尽量创建备份。
4. 需要 Claude Desktop 识别的 MCP，安装后重启 Claude Desktop。

官方 Claude 插件通常依赖本机 `claude` CLI。社区 MCP 和 Skill 默认只拉取元数据，只有结构可识别且安装方式明确时才提供安装按钮。

## Codex 供应商与中转

供应商配置适合已经准备好兼容 API 或中转服务的场景。管理工具会写入 Codex 的 provider 配置，例如：

```toml
model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.com/v1"
experimental_bearer_token = "sk-..."
```

使用建议：

1. 先确认 Base URL 可访问，并支持所选协议。
2. 用最小请求测试 key 是否可用。
3. 不要把真实 key 放进日志、截图或 issue。
4. 写入前确认 `~/.codex/config.toml` 已有备份。
5. 如需回到官方登录模式，在管理工具中清除 API 模式。

## 安全说明

- 不修改官方 Codex App、Claude Desktop、MSIX、`app.asar`、签名或完整性文件。
- 不把 API key、Bearer token 或完整鉴权配置写入普通日志。
- 第三方 GitHub 内容默认只读取元数据，不自动执行脚本。
- 插件、MCP 和 Skill 安装前先展示命令或配置 diff。
- 旧版本入口、旧快捷方式、旧数据目录会由安装器或维护流程自动迁移、清理；公开入口统一使用当前名称。

## 数据位置

- Codex 配置：`~/.codex/config.toml`
- Codex 登录状态：`~/.codex/auth.json`
- Codex 本地数据库：优先 `~/.codex/sqlite/*.db`，回退到旧版 `~/.codex/state_5.sqlite`
- Claude Desktop MCP 配置：Windows 通常位于 `%APPDATA%\Claude\claude_desktop_config.json`
- Claude Codex Pro 状态与日志：`~/.claude-codex-pro/`
- Provider Sync 备份：`~/.codex/backups_state/provider-sync`

## 常见问题

### Codex 里没有看到增强标识

确认是从 `Claude Codex Pro` 入口启动 Codex，而不是直接启动原始 Codex。仍然没有显示时，打开管理工具查看诊断和日志，重点检查 helper 端口、CDP 连接和 `renderer.script_loaded` 记录。

### Claude 没有变成中文

中文化目标是 `打开 Claude 中文窗口` 创建的独立 WebView。官方 Claude Desktop 原窗口不会被强行修改。如果包装窗口中也没有中文覆盖，查看管理工具日志中的 Claude 中文窗口状态和注入脚本错误。

### 插件安装失败

先打开安装预览，确认安装类型：

- Claude 官方插件需要本机 `claude` CLI。
- Claude Desktop MCP 需要可写入 `claude_desktop_config.json`。
- 社区 MCP 和 Skill 需要结构可识别。
- 安装后需要重启 Claude Desktop 才能被桌面端加载。

### macOS 提示应用无法打开或已损坏

未签名或未公证的构建可能被 Gatekeeper 拦截。可以在“系统设置 -> 隐私与安全性”中允许打开。若仍提示已损坏，可执行：

```bash
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro.app
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro\ 管理工具.app
```

### 是否支持 Intel Mac？

支持。Release 会分别提供 `macos-x64.dmg` 和 `macos-arm64.dmg`。Intel Mac 使用 x64 包，Apple Silicon 使用 arm64 包。

## 构建与开发

本项目是 Rust workspace + Tauri 管理工具 + Vite/React 前端。仓库根目录的 `package.json` 来自上游 Claude Desktop 代码结构，不用于构建本项目；实际前端依赖在 `apps/claude-codex-pro-manager` 下安装和构建。

### 环境要求

- Git。
- Node.js 22 或更高版本。
- npm。
- Rust stable toolchain，包含 `cargo`、`rustc`、`rustfmt`。
- Windows 构建需要 Visual Studio Build Tools / MSVC C++ 工具链，打安装包需要 NSIS。
- macOS 构建需要 Xcode Command Line Tools。打 DMG 会使用系统自带的 `sips`、`iconutil`、`codesign`、`hdiutil`。

Windows 安装 NSIS 示例：

```powershell
choco install nsis -y
```

macOS 安装 Rust 目标示例：

```bash
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

### 安装依赖

```bash
cd apps/claude-codex-pro-manager
npm install --package-lock=false
cd ../..
```

如果希望严格使用 lockfile，也可以把第一条命令换成 `npm ci`。CI 目前使用 `npm install --package-lock=false`，避免构建流程改写锁文件。

### 本地开发启动

```bash
cd apps/claude-codex-pro-manager
npm run dev
```

该命令会由 Tauri CLI 启动管理工具，并自动运行 Vite 开发服务器。Vite 默认监听 `http://localhost:1420`。

只调试前端页面时可以运行：

```bash
cd apps/claude-codex-pro-manager
npm run vite:dev
```

注意：普通浏览器预览没有 Tauri 后端，涉及系统配置、进程、插件安装、Claude 汉化等按钮会返回未实现或无法执行；真实功能请用 `npm run dev` 启动 Tauri 应用验证。

### 本地验证

提交前建议至少运行：

```bash
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

常用的定向验证：

```bash
# Claude 本机汉化补丁
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml claude_zh_patch::tests -- --nocapture

# 插件中心
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml plugin_hub -- --nocapture

# 管理工具窗口、路由、打包约束
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
```

### 生产二进制

生产构建分两步：先构建前端静态资源，再构建 Rust workspace。

```bash
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
```

构建完成后，主要产物在：

```text
target/release/claude-codex-pro.exe
target/release/claude-codex-pro-manager.exe
```

macOS 或 Linux 上没有 `.exe` 后缀：

```text
target/release/claude-codex-pro
target/release/claude-codex-pro-manager
```

也可以在管理工具目录运行：

```bash
cd apps/claude-codex-pro-manager
npm run build
```

该脚本会先构建静默启动器，再执行 `tauri build`。当前 `tauri.conf.json` 的 `bundle.active` 为 `false`，因此正式安装包仍以仓库里的 NSIS / DMG 脚本为准。

### Windows 安装包

Windows 打包流程与 GitHub Actions 一致：

```powershell
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test --workspace
cargo build --release

New-Item -ItemType Directory -Force dist/windows/app | Out-Null
Copy-Item target/release/claude-codex-pro.exe dist/windows/app/
Copy-Item target/release/claude-codex-pro-manager.exe dist/windows/app/

$version = "1.2.9"
$makensis = "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
if (-not (Test-Path $makensis)) { $makensis = "makensis" }
Push-Location scripts/installer/windows
& $makensis "/INPUTCHARSET" "UTF8" "/DVERSION=$version" ClaudeCodexPro.nsi
Pop-Location
```

安装包输出到：

```text
dist/windows/claude-codex-pro-1.2.9-windows-x64-setup.exe
```

### macOS DMG

macOS 需要在对应架构的 macOS runner 或本机上构建。Intel Mac 使用 `x86_64-apple-darwin`，Apple Silicon 使用 `aarch64-apple-darwin`。

```bash
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:build

# Apple Silicon
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
BINARY_DIR="$PWD/target/aarch64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 1.2.9 arm64

# Intel Mac
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
BINARY_DIR="$PWD/target/x86_64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 1.2.9 x64
```

DMG 输出到：

```text
dist/macos/claude-codex-pro-1.2.9-macos-arm64.dmg
dist/macos/claude-codex-pro-1.2.9-macos-x64.dmg
```

本地脚本使用 ad-hoc codesign，不做 Apple Developer ID 签名或公证。因此本地 DMG 可能被 Gatekeeper 提示，需要按上文 macOS 常见问题手动允许。

### GitHub Actions 构建

仓库包含两个主要工作流：

- `.github/workflows/pr-build.yml`：在 PR、`main` push、手动触发时构建 Windows artifacts 和 macOS DMG artifacts。
- `.github/workflows/release-assets.yml`：发布 GitHub Release 后自动构建并上传 Windows 安装包、macOS x64/arm64 DMG 和 `latest.json`。

发版流程：

1. 确认本地验证通过。
2. 推送代码到 `main`。
3. 创建并发布 tag，例如 `v1.2.9`。
4. GitHub Release 发布后，`release-assets.yml` 会从 release tag 构建安装包并上传到该 Release。

版本号来源：

- Rust workspace 版本在根目录 `Cargo.toml` 的 `[workspace.package] version`。
- 管理工具前端版本在 `apps/claude-codex-pro-manager/package.json`。
- Tauri 配置版本在 `apps/claude-codex-pro-manager/src-tauri/tauri.conf.json`。
- NSIS / DMG 输出文件名使用传入脚本的 `$version` 或 GitHub tag 去掉开头的 `v`。

项目结构：

```text
apps/
  claude-codex-pro-launcher/          静默启动器
  claude-codex-pro-manager/           Tauri 管理工具
assets/inject/
  renderer-inject.js                  Codex 增强脚本
  claude-chinese-inject.js            Claude 中文窗口脚本
crates/
  claude-codex-pro-core/              启动、注入、配置、插件、更新、安装、bridge
  claude-codex-pro-data/              会话数据、导出、Provider Sync
scripts/installer/
  windows/ClaudeCodexPro.nsi          Windows NSIS 安装器
  macos/package-dmg.sh                macOS DMG 打包脚本
```

## 反馈

- Issues：<https://github.com/DamonZS/Claude-Codex-Pro-Tool/issues>
- 讨论群二维码：<https://docs.qq.com/doc/DQ2VOanZTTFZJcUpZ#>

## 说明

Claude Codex Pro Tool 是外部增强工具，不是 OpenAI、Anthropic、Claude 或 Codex 的官方项目。官方应用更新后，如果页面结构、协议或配置格式变化，本项目的注入脚本和适配逻辑可能需要同步更新。
