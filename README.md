# Claude Codex Pro Tool

<p align="center">
  <img src="assets/images/claude-codex-pro.png" alt="Claude Codex Pro Tool 图标" width="160">
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

把 Codex App 和 Claude Desktop 变成一个可运营、可维护、可扩展的本地 AI 工作台。

Claude Codex Pro Tool 不只是一个启动器。它把 Codex 增强、Claude Desktop 本地集成、供应商/中转切换、插件中心、Ponytail、多工具 Skills、会话修复、盘古记忆、脚本市场、提示词优化、Zed Remote、自动更新和安装维护整合到一个 Tauri 运维控制台里。你可以把它理解成：给 Codex 和 Claude Desktop 配一个本地控制室，把原本散落在配置文件、命令行、插件目录、脚本市场里的能力集中管理。

项目仓库唯一地址：

<https://github.com/DamonZS/Claude-Codex-Pro-Tool>

## 适合谁

- 想让 Codex App 有更完整本地增强能力的人。
- 需要在多个 API 中转、兼容 OpenAI 协议供应商之间切换的人。
- 想把 Claude Desktop、Claude Code、Codex、MCP、Skills、插件放进同一个管理界面的人。
- 经常修会话、导出历史、切换项目、管理脚本和插件的人。
- 希望功能真实可验证，不想要“按钮看起来有、实际没实现”的人。

## 核心原则

- 本地优先：配置、记忆、插件记录、日志和备份都优先落在本机。
- 可审查：安装插件、写入 MCP、信任 hooks、修改配置前展示命令或 diff。
- 可回退：写入关键配置前尽量备份，Claude 中文资源补丁提供还原入口。
- 不静默信任第三方：Ponytail / Codex hooks 需要单独审查和信任。
- 不伪装能力：无法自动安装或需要人工确认的功能会明确标记为需审查。

## 下载

从 [GitHub Releases](https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases) 下载最新版：

- Windows：`claude-codex-pro-*-windows-x64-setup.exe`
- macOS Intel：`claude-codex-pro-*-macos-x64.dmg`
- macOS Apple Silicon：`claude-codex-pro-*-macos-arm64.dmg`

安装后有两个入口：

- `Claude Codex Pro`：静默启动器，用来启动 Codex 并加载增强能力。
- `Claude Codex Pro 管理工具`：运维控制台，用来管理 Codex、Claude、供应商、插件、脚本、记忆、日志、安装维护和更新。

Windows 安装包会创建桌面和开始菜单快捷方式。macOS DMG 会包含 `Claude Codex Pro.app` 与 `Claude Codex Pro 管理工具.app`。

## 功能总览

### 1. Codex 启动与增强

- 通过外部启动器启动 Codex。
- 自动处理 CDP / helper 连接。
- 在 Codex 页面注入顶部状态标识。
- 支持 Codex 插件入口解锁和插件市场入口解锁。
- 支持 Codex 插件安装通道适配，包括新版 `vscode://codex/list-plugins`、`vscode://codex/plugin/install` 等请求。
- 支持模型白名单解锁。
- 支持服务等级控制入口。
- 支持图片覆盖配置。
- 支持会话滚动位置恢复。
- 支持会话时间线和会话视图增强。
- 支持原生菜单位置调整。
- 支持 Codex Goals 配置写入。
- 支持 Computer Use Guard，减少危险自动化误触。

### 2. Codex 会话管理与修复

- 列出本地 Codex 会话。
- 删除本地会话。
- 导出 Markdown。
- 移动项目归属。
- 查看会话数据库位置。
- 检测新版 `~/.codex/sqlite/*.db` 和旧版 `~/.codex/state_5.sqlite`。
- Provider Sync：切换供应商后修复历史会话可见性。
- 支持从当前配置回填供应商配置，避免切换时覆盖旧配置。

### 3. 供应商与中转配置

- 支持官方模式、官方混合 API 模式、纯 API 模式。
- 支持 Responses 与 Chat Completions 协议。
- 支持多个供应商 Profile。
- 支持 Base URL、API Key、模型、User-Agent、上下文窗口、自动压缩阈值。
- 支持公共配置与上下文配置拆分。
- 支持 MCP / Skills / Plugins 上下文选择。
- 支持从当前 `~/.codex/config.toml` 和 `auth.json` 回填配置。
- 支持测试供应商连通性。
- 支持清除 API 模式，回到官方登录配置。
- 支持兼容 API / 中转站配置，例如自建服务或第三方中转。

示例配置会写入 `~/.codex/config.toml`：

```toml
model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.com/v1"
experimental_bearer_token = "sk-..."
```

### 4. Claude Desktop 管理

- 启动官方 Claude Desktop。
- 聚焦 Claude Desktop 窗口。
- 打开 Claude Desktop DevTools。
- 新建 Claude Desktop 对话。
- 向 Claude Desktop 粘贴草稿。
- 向 Claude Desktop 提交文本。
- 检测 Claude Desktop 安装位置、进程状态和完整性状态。
- 展示官方 MSIX / CDP 阻断诊断，不伪装成已注入。

### 5. Claude 中文能力

项目提供两类中文化路径，适合不同风险偏好：

- Claude 中文包装窗口：独立 WebView 加载 `https://claude.ai/new`，在创建阶段注入中文覆盖脚本和顶部状态标识。推荐优先使用这个方式，不改官方安装文件。
- Claude Desktop 中文资源补丁：可选的本机资源补丁，参考 `Jyy1529/claude-desktop_win-zh_cn` 的公开资源，写入 `zh-CN.json`、locale 配置和必要的前端语言支持补丁。执行前会备份，管理工具提供还原入口。

注意：官方 Claude Desktop 的 MSIX、签名和完整性机制可能限制直接 DOM 注入或文件补丁。包装窗口是低风险方案；资源补丁是用户明确选择后的本机修改方案。

### 6. 插件中心

插件中心把多个来源统一成一个目录视图：

- Claude 官方插件市场。
- Claude Desktop MCP 配置项。
- GitHub MCP Registry。
- Awesome Claude Code 资源。
- OpenAI Codex Plugins 仓库。
- Ponytail 多工具插件。
- 可识别的 Skill bundle。
- 社区资源链接。

每个条目会展示：

- 来源、分类、作者、许可证。
- 安装状态。
- 风险提示。
- 依赖要求。
- 安装命令预览。
- 配置 diff。
- 安装、卸载、打开来源等操作。

安装策略：

- 官方 Claude 插件通过 `claude plugin marketplace add/install`。
- Claude Desktop MCP 写入 `claude_desktop_config.json`，写入前备份。
- Codex 插件通过 `codex plugin marketplace add/list/add`。
- Skill bundle 只有识别到结构时才安装。
- 未知社区 MCP 默认只展示，不自动执行脚本。

### 7. Ponytail 集成

已集成 [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail)，支持多工具安装：

- Ponytail for Codex：调用 Codex CLI 添加 marketplace、安装 `ponytail@ponytail`。
- Ponytail Codex hooks：可预览待信任 hooks，用户确认后才写入信任状态。
- Ponytail Skills for Codex：复制 Ponytail Skills 到 Codex skills 目录，覆盖前备份。
- Ponytail for Claude Code：通过 Claude Code CLI 安装。
- Ponytail MCP for Claude Desktop：写入 Claude Desktop MCP 配置。
- Ponytail Organization Plugin for Claude Desktop：写入 Claude Desktop 开发模式可读取的组织插件目录。
- Ponytail MCPB：生成 `.mcpb` 安装包并交给 Claude Desktop 官方确认流程。
- Ponytail for GitHub Copilot CLI：通过 Copilot CLI 插件系统安装。

Claude Desktop 的本地插件包流程不会要求 Claude CLI 登录：它配置开发模式、写入 Codex/Ponytail MCP，并复制 Ponytail skills 到组织插件目录。

### 8. Codex OpenAI 插件仓库

- 下载 OpenAI 官方 `openai/plugins` 仓库 zip。
- 限制下载体积。
- 安全解压，防止 zip path traversal。
- 校验 `.agents/plugins/marketplace.json`。
- 校验每个插件目录的 `.codex-plugin/plugin.json`。
- 注册到 `~/.codex/config.toml` 的 `[marketplaces.openai-curated]`。
- 出错时展示具体失败原因，不把坏仓库注册成成功。

### 9. 盘古记忆

盘古记忆使用 SQLite，不引入云端 embedding 或外部向量库。

支持：

- 手动写入长期记忆。
- 自动生成待确认记忆。
- 用户确认后再进入长期记忆。
- 工作区隔离。
- `global` 全局记忆。
- 当前工作区 + 全局混合查询。
- 关键词归一化和轻量相似度排序。
- 命中后更新访问次数和最后访问时间。
- 密钥脱敏，避免 API key、Bearer token、`sk-` 类内容原文入库。
- 自检和修复。
- 导出 JSON。
- 导入 JSON，支持合并或替换。

默认数据库：

```text
~/.claude-codex-pro/memory_assist.sqlite
```

### 10. 脚本市场与用户脚本

- 刷新脚本市场。
- 下载并安装脚本。
- 管理本地用户脚本。
- 启用/禁用单个脚本。
- 删除用户脚本。
- 构建已启用脚本 bundle。
- 通过 Codex 注入脚本扩展前端能力。

### 11. 提示词优化器

集成 [linshenkx/prompt-optimizer](https://github.com/linshenkx/prompt-optimizer) 的提示词优化工作流。

支持：

- 管理工具内打开提示词优化页面。
- 独立提示词优化窗口。
- 作为工具体系的一部分使用，不再额外弹出重复控制窗口。

### 12. Zed Remote

- 识别 Zed 安装路径。
- 解析 SSH host、user、port。
- 从 Codex global state 和线程上下文解析远程项目。
- 维护最近远程项目注册表。
- 构造 `zed://ssh/...` 远程打开链接。
- 支持默认打开、复用窗口、新窗口、追加到当前窗口等策略。
- 支持忘记远程项目。

### 13. Upstream Worktree

- 读取 Git remote、branch、worktree 列表。
- 从最新远程跟踪分支创建 worktree。
- 支持本地项目和远程项目。
- 校验分支名和 base branch。
- 避免从过期本地 HEAD 派生任务分支。

### 14. Watcher 与自恢复

- Windows 下可安装 watcher。
- 检测 Codex 进程与 CDP 端口状态。
- 恢复失效 launcher。
- 支持启用、禁用、安装、卸载。
- 支持停止 launcher / Codex 相关进程。

### 15. 安装维护与更新

- 安装入口。
- 卸载入口。
- 修复快捷方式。
- 修复后端配置。
- 检查更新。
- 下载 Release 资产。
- 启动安装器。
- 读取最新日志。
- 复制诊断信息。
- 重置设置。
- 重置图片覆盖设置。

### 16. 自动构建与 Release

- `Auto release installers`：main push 或手动触发后自动计算 `V0.01` 系列版本、创建 tag、构建 Windows 安装包、macOS x64 DMG、macOS arm64 DMG，并上传 `latest.json`。
- `PR build artifacts`：用于 PR 和日常构建校验。
- `release-assets`：保留给手动 GitHub Release 使用。

自动版本递增规则：

```text
V0.01 -> V0.02 -> ... -> V0.99 -> V1.00
```

## 管理工具页面

- 概览：运行状态、Codex/Claude 快捷动作、日志摘要、官方中转站入口。
- 供应商：管理 Codex API / 中转 / 官方混合配置。
- 工具与插件：会话管理、历史会话修复、插件中心、Ponytail、Codex 插件仓库、Claude Desktop 本地插件、盘古记忆。
- 提示词：提示词优化器。
- 脚本：脚本市场和用户脚本管理。
- 设置：真实可用开关、启动参数、增强矩阵、记忆、Zed、Watcher、安装维护。

## 安全边界

- 不静默修改 Claude Desktop 私有插件库。
- 不静默信任第三方 hooks。
- 不自动执行未知社区 MCP 安装脚本。
- 不把 API key、Bearer token 或完整鉴权配置写入普通日志。
- 不把第三方 GitHub 内容默认当作可信代码执行。
- Claude 中文包装窗口不修改官方 Claude Desktop 文件。
- Claude Desktop 中文资源补丁属于用户明确触发的本机补丁，执行前备份，可还原。

## 数据位置

- Codex 配置：`~/.codex/config.toml`
- Codex 登录状态：`~/.codex/auth.json`
- Codex 数据库：优先 `~/.codex/sqlite/*.db`，回退到旧版 `~/.codex/state_5.sqlite`
- Codex 插件仓库缓存：`~/.codex/.tmp/plugins`
- Codex skills：`~/.codex/skills`
- Claude Desktop MCP 配置：Windows 通常为 `%APPDATA%\Claude\claude_desktop_config.json`
- Claude Desktop 3P 配置：Windows 通常为 `%LOCALAPPDATA%\Claude-3p`
- Claude Codex Pro 状态：`~/.claude-codex-pro/`
- 记忆数据库：`~/.claude-codex-pro/memory_assist.sqlite`
- Provider Sync 备份：`~/.codex/backups_state/provider-sync`

## 常见问题

### 为什么一个提交会出现两个 Actions？

因为当前仓库有两个 workflow 监听 `main` push：

- `Auto release installers`：构建并发布安装包。
- `PR build artifacts`：日常构建校验。

这不是发布两个版本，只是同一个提交触发了两条流水线。

### Release 里为什么只有 Source code？

如果安装包构建 job 成功但发布 job 失败，GitHub 页面会只显示自动生成的源码 zip/tar.gz。需要查看 `Auto release installers` 的 `Publish release and latest.json` 步骤。当前 workflow 会先发布 draft release，再生成并上传 `latest.json`，避免 draft release 无法按 tag 查询导致失败。

### Codex 里没有看到增强标识

确认是从 `Claude Codex Pro` 入口启动 Codex，而不是直接启动原始 Codex。仍然没有显示时，打开管理工具查看诊断和日志，重点检查 helper 端口、CDP 连接和 `renderer.script_loaded` 记录。

### Claude 没有变成中文

优先使用 `打开 Claude 中文窗口`。这是独立 WebView 包装窗口，不是官方 Claude Desktop 原窗口。若使用资源补丁，请确认 Claude Desktop 已完全退出、安装目录可写，并在失败时查看补丁状态或执行还原。

### 插件安装失败

先打开安装预览，确认安装类型：

- Claude 官方插件需要 `claude` CLI。
- Claude Desktop MCP 需要写入 `claude_desktop_config.json`。
- Claude Desktop 本地组织插件需要开发模式和目录写入权限。
- Codex 插件需要 `codex` CLI。
- Ponytail hooks 需要单独审查和信任。
- 社区 MCP 和 Skill 需要结构可识别。

### macOS 提示应用无法打开或已损坏

未签名或未公证的构建可能被 Gatekeeper 拦截。可以在“系统设置 -> 隐私与安全性”中允许打开。若仍提示已损坏，可执行：

```bash
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro.app
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro\ 管理工具.app
```

### 是否支持 Intel Mac？

支持。Release 会分别提供 `macos-x64.dmg` 和 `macos-arm64.dmg`。Intel Mac 使用 x64 包，Apple Silicon 使用 arm64 包。

## 构建与开发

本项目是 Rust workspace + Tauri 管理工具 + Vite/React 前端。仓库根目录的 `package.json` 来自上游结构，不用于构建本项目；实际前端依赖在 `apps/claude-codex-pro-manager` 下安装和构建。

### 环境要求

- Git。
- Node.js 22 或更高版本。
- npm。
- Rust stable toolchain，包含 `cargo`、`rustc`、`rustfmt`。
- Windows 构建需要 Visual Studio Build Tools / MSVC C++ 工具链。
- Windows 打安装包需要 NSIS。
- macOS 构建需要 Xcode Command Line Tools。
- macOS 打 DMG 会使用系统自带的 `sips`、`iconutil`、`codesign`、`hdiutil`。

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

如果希望严格使用 lockfile，也可以把第一条命令换成 `npm ci`。CI 目前使用 `npm install --package-lock=false`。

### 本地开发启动

```bash
cd apps/claude-codex-pro-manager
npm run dev
```

该命令会由 Tauri CLI 启动管理工具，并自动运行 Vite 开发服务器。Vite 默认监听：

```text
http://localhost:1420
```

只调试前端页面时可以运行：

```bash
cd apps/claude-codex-pro-manager
npm run vite:dev
```

普通浏览器预览没有 Tauri 后端，涉及系统配置、进程、插件安装、Claude 汉化等按钮会返回预览或无法执行；真实功能请用 `npm run dev` 启动 Tauri 应用验证。

### 本地验证

提交前建议运行：

```bash
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

常用定向验证：

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml plugin_hub -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml memory_assist -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml relay_config -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
```

### 生产二进制

```bash
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
```

主要产物：

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

该脚本会先构建静默启动器，再执行 `tauri build`。当前正式安装包仍以仓库里的 NSIS / DMG 脚本为准。

### Windows 安装包

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

输出：

```text
dist/windows/claude-codex-pro-1.2.9-windows-x64-setup.exe
```

### macOS DMG

Apple Silicon：

```bash
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:build
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
BINARY_DIR="$PWD/target/aarch64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 1.2.9 arm64
```

Intel Mac：

```bash
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:build
rustup target add x86_64-apple-darwin
cargo build --release --target x86_64-apple-darwin
BINARY_DIR="$PWD/target/x86_64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 1.2.9 x64
```

输出：

```text
dist/macos/claude-codex-pro-1.2.9-macos-arm64.dmg
dist/macos/claude-codex-pro-1.2.9-macos-x64.dmg
```

本地脚本使用 ad-hoc codesign，不做 Apple Developer ID 签名或公证。因此本地 DMG 可能被 Gatekeeper 提示，需要按上文 macOS 常见问题手动允许。

## GitHub Actions

主要工作流：

- `.github/workflows/auto-release-installers.yml`：main push 或手动触发后自动发版。
- `.github/workflows/pr-build.yml`：PR、main push、手动触发时构建验证产物。
- `.github/workflows/release-assets.yml`：保留给手动 GitHub Release 使用。

自动发版流程：

1. 推送到 `main` 或手动运行 `Auto release installers`。
2. `scripts/release/next-release-tag.js` 读取现有 tag。
3. 生成下一版 `V0.01` 系列 tag。
4. 创建 tag 和 draft Release。
5. Windows runner 构建 `.exe` 安装包。
6. macOS Intel runner 构建 x64 DMG。
7. macOS Apple Silicon runner 构建 arm64 DMG。
8. 上传安装包。
9. 发布 Release。
10. 生成并上传 `latest.json`。

自动发版产物示例：

```text
claude-codex-pro-0.01-windows-x64-setup.exe
claude-codex-pro-0.01-macos-x64.dmg
claude-codex-pro-0.01-macos-arm64.dmg
latest.json
```

## 项目结构

```text
apps/
  claude-codex-pro-launcher/          静默启动器
  claude-codex-pro-manager/           Tauri 管理工具
assets/inject/
  renderer-inject.js                  Codex 增强脚本
  claude-chinese-inject.js            Claude 中文包装窗口脚本
crates/
  claude-codex-pro-core/              启动、注入、配置、插件、更新、安装、bridge
  claude-codex-pro-data/              会话数据、导出、Provider Sync
scripts/installer/
  windows/ClaudeCodexPro.nsi          Windows NSIS 安装器
  macos/package-dmg.sh                macOS DMG 打包脚本
docs/
  code-knowledge-graph.md             代码知识图谱
  full-code-review.md                 全量代码评审记录
```

## 反馈

- Issues：<https://github.com/DamonZS/Claude-Codex-Pro-Tool/issues>
- 讨论群二维码：<https://docs.qq.com/doc/DQ2VOanZTTFZJcUpZ#>

## 协议与规则

本仓库采用自定义源码可见限制协议，不是 OSI 认证开源协议。未经 DamonZS 或授权维护者书面允许，禁止以任何方式修改、发布、分发、改名、重打包或隐藏本项目来源，包括人工修改、AI 辅助修改、脚本、codemod、批量替换、自动化重写、二进制补丁和元数据改写。

作者信息、仓库地址、版权声明、产品名称、品牌、发布者、赞助或支付身份、协议和规则文件不得被删除、替换、隐藏或弱化。

这些限制不约束 DamonZS、仓库所有者、授权维护者以及在其指令下工作的 AI 助手、脚本、CI、codemod、格式化工具或自动化工具。官方项目后续开发可以继续使用 AI 和自动化能力。

授权维护者名单见 [MAINTAINERS.md](MAINTAINERS.md)。详见 [LICENSE](LICENSE) 和 [RULES.md](RULES.md)。

## 说明

Claude Codex Pro Tool 是外部增强工具，不是 OpenAI、Anthropic、Claude 或 Codex 的官方项目。官方应用更新后，如果页面结构、协议、CLI、插件格式或配置路径变化，本项目的注入脚本和适配逻辑可能需要同步更新。
