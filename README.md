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

**一个窗口管好你所有的 AI 编程 Agent：看清每个 Agent 的 Token 花在哪，一处切换第三方 API，把聊过的会话蒸馏成可复用的 Skill 和记忆，让 Claude Desktop（含 API 模式）也能用上 Computer Use。**

> Windows / macOS 本地 AI 运维控制台，面向 Codex App、Claude Desktop、Claude Code，内置 36 种主流 Agent 工具的识别清单。Rust + Tauri 实现，数据只在本机。
>
> English: a local Rust + Tauri control panel for **Codex App**, **Claude Desktop** and **Claude Code**: a token-usage dashboard across AI coding agents, a cc-switch-style third-party API provider switcher, a session-to-Skill distillation workbench, and a built-in **Computer Use MCP** for Claude Desktop.

<table>
  <tr>
    <td width="50%" align="center"><a href="docs/screenshots/overview-total.webp"><img src="docs/screenshots/overview-total.webp" alt="总览：Agent 覆盖、Token 消耗趋势、缓存命中率、模型消耗" width="100%"></a><br><sub><b>总览</b> · 所有 Agent 的 Token 消耗、缓存命中率、模型排行</sub></td>
    <td width="50%" align="center"><a href="docs/screenshots/overview-agent.webp"><img src="docs/screenshots/overview-agent.webp" alt="Agent 概览：单个 Agent 的消耗趋势与上下文构成" width="100%"></a><br><sub><b>Agent 概览</b> · 单个 Agent 的消耗趋势与上下文构成</sub></td>
  </tr>
  <tr>
    <td width="50%" align="center"><a href="docs/screenshots/distillation.webp"><img src="docs/screenshots/distillation.webp" alt="蒸馏工作台：按会话、按项目选素材，蒸馏成 Skill、工作流、Prompt、画像、任务记忆" width="100%"></a><br><sub><b>蒸馏工作台</b> · 会话 / 项目 → Skill、Prompt、画像、任务记忆</sub></td>
    <td width="50%" align="center"><a href="docs/screenshots/tools-skills-mcp.webp"><img src="docs/screenshots/tools-skills-mcp.webp" alt="插件、Skills 与 MCP：Claude 与 Codex 资产统一清单" width="100%"></a><br><sub><b>插件、Skills 与 MCP</b> · Claude 与 Codex 资产合并成一张清单</sub></td>
  </tr>
</table>

**目录：** [为什么用 CCP](#为什么用-ccp) · [快速开始](#快速开始) · [核心功能](#核心功能) · [界面预览](#界面预览) · [安全与隐私](#安全与隐私) · [常见问题](#常见问题) · [构建与开发](#构建与开发)

## 为什么用 CCP

| 你遇到的问题 | CCP 怎么解决 |
| --- | --- |
| 用了好几个 AI 编程工具，不知道 Token 都花哪了 | 本地只读采集，一个看板看所有 Agent 的消耗趋势、缓存命中率、模型与项目排行 |
| 第三方 API 经常换，每个工具都要改一遍配置 | Codex、Claude、Claude Desktop 各自记录当前供应商，一处切换，互不影响 |
| 聊过的好方案散落在历史会话里，下次又从头讲 | 蒸馏工作台把会话或整个项目提炼成 Skill、Prompt、用户画像、任务记忆 |
| 长对话超出上下文后被拆成一串会话文件 | 自动识别“续接”，合并成一行，蒸馏时按时间顺序读整条 |
| Claude Desktop 不能像 Codex 那样操作电脑 | 自带 Computer Use MCP：截图、点击、拖拽、输入，带急停，默认关闭 |
| Skills、MCP、插件分散在各个工具里 | Claude 与 Codex 的资产合并成一张清单，点应用图标只改对应应用 |
| 切换供应商后历史会话“消失”了 | Provider Sync 修复历史会话可见性 |

**适合你，如果你：**

- 同时用 Codex、Claude Code、Claude Desktop、Workbuddy、Cursor 等多个 AI 编程工具，想知道钱和 Token 去哪了。
- 要在多个 API 中转或兼容 OpenAI / Anthropic 协议的供应商之间来回切换。
- 想把反复踩的坑和成熟流程沉淀成 Skill，而不是每次重新解释。
- 希望功能真实可验证，不想要“按钮看起来有、实际没实现”。

## 快速开始

1. 从 [GitHub Releases](https://github.com/DamonZS/Claude-Codex-Pro-Tool/releases) 下载并安装：
   - Windows：`claude-codex-pro-*-windows-x64-setup.exe` 或 `claude-codex-pro-*-windows-x64.msi`
   - macOS Intel：`claude-codex-pro-*-macos-x64.dmg`
   - macOS Apple Silicon：`claude-codex-pro-*-macos-arm64.dmg`
2. 打开 **Claude Codex Pro 管理工具**，在「供应商与路由」里新增供应商，或从本机 cc-switch 数据库导入已有配置。
3. 用管理工具顶部的「启动/重启 Codex」「启动/重启 Claude」启动客户端（不要直接启动原始 Codex，否则没有增强）。
4. 打开「概览」查看各 Agent 的 Token 用量；需要时再开启 Computer Use、蒸馏会话、安装 Skills。

安装后有两个入口：

- `Claude Codex Pro`：统一桌面程序；默认打开管理工具，内部以 `--launcher` 启动独立后台进程并加载 Codex 增强能力。
- `Claude Codex Pro 管理工具`：运维控制台，管理 Codex、Claude、供应商、插件、脚本、日志、安装维护和更新。

Windows 安装包会创建桌面和开始菜单快捷方式；macOS DMG 包含 `Claude Codex Pro.app` 与 `Claude Codex Pro 管理工具.app`。

> **Codex 模型选择由 Codex 原生逻辑负责。** CCP 不向 Codex 模型选择器注入候选项、不修改模型白名单或请求中的模型字段，也不通过前端增强显示特定模型。供应商页面仍可管理模型目录和路由配置；Codex 能否显示或调用某个模型取决于 Codex 自身版本、登录状态、配置和上游 API 能力。

项目仓库唯一地址：<https://github.com/DamonZS/Claude-Codex-Pro-Tool>

## 核心功能

### 多 Agent 用量看板

本地只读采集，内置 36 种主流 Agent 工具的识别清单（Claude Code、Codex、Cursor、Kiro、Gemini CLI、OpenCode、OpenClaw、Hermes、GitHub Copilot、Zed、Cline、Roo Code、Goose、WorkBuddy 等），其中 35 种声明了用量采集路径。你本机有记录的 Agent 才会出数据，没有记录的不会编造。

- **总览**：Agent 覆盖、蒸馏资产、今日消耗；Token 消耗、费用估算、会话总数、缓存命中率、Agent 活跃数；Token 趋势图（缓存读取 / 输入 / 输出，对比上一区间）、按模型的消耗排行、项目消耗总览和近 12 个月活跃日历。时间范围 24 小时 / 7 天 / 30 天。
- **Agent 概览**：选一个 Agent 看它自己的消耗趋势、上下文构成（对话消息、推理过程、缓存命中）、按模型或按项目的消耗明细、会话场次和 Skill 覆盖。时间范围支持今天 / 近 7 天 / 近 30 天 / 全部 / 自定义区间。
- **诚实的数字**：没有价格来源时费用显示“未计价”；采集不到的指标显示“未采集”；会话 ID 匿名化，不读取正文。

### 供应商与路由

- **三个独立目标**：Codex、Claude、Claude Desktop 各自记录当前供应商，切换一个不会改动另外两个；切换成功提示会明确是哪个目标。
- **三种协议**：OpenAI Responses、Chat Completions、Anthropic Messages，并支持兼容渠道的模型映射与协议转换。
- **Profile 管理**：Base URL、API Key、Header、Body、User-Agent、模型目录、上下文窗口、自动压缩阈值、优先级和故障转移；可测试连通性、拖拽排序。
- **Codex 模式**：官方模式、官方混合 API 模式、纯 API 模式；可从当前 `~/.codex/config.toml` 与 `auth.json` 回填配置，也可清除 API 模式回到官方登录。
- **Claude Desktop 直连**：第三方 Anthropic 兼容供应商可直接使用其 `/v1/models` 返回的模型 ID，不必强制模型映射；也可选模型映射或手动模型列表。关闭映射时使用供应商真实 URL，而不是本地代理。
- **一键导入**：从本机 cc-switch 数据库导入已有的 Codex / Claude / Claude Desktop 供应商配置，保留原有路由与 API 格式。

示例，Codex 自定义供应商会写入 `~/.codex/config.toml`：

```toml
model_provider = "custom"

[model_providers.custom]
name = "custom"
wire_api = "responses"
requires_openai_auth = true
base_url = "https://example.com/v1"
experimental_bearer_token = "sk-..."
```

### 蒸馏工作台：把会话变成资产

从本机 Codex 与 Claude 的历史会话里提炼可复用的东西，交互与处理逻辑对齐 AITracker 的蒸馏模块（Copyright (C) 2026 AITracker contributors，已获版权方授权），界面使用 CCP 液态玻璃风格。

- **选素材**：快速模式按会话或按项目勾选，可按今天 / 近 7 天 / 近 30 天 / 全部过滤；高级模式在素材库里跨会话框选消息区间。
- **读真实对话**：选中整场会话或整个项目时，模型读到的是对话正文（不含推理块），不是只有标题和轮数。
- **产物**：能力资产（Skill、工作流、Prompt，保存到 Skill 库并可安装到所选 Agent）与记忆资产（用户画像、任务记忆，写入记忆库）。
- **长对话不截断**：输入预算随所选供应商的 `contextWindow` 放大（单次最多用窗口的 60%）；超出就按时间顺序分批提取要点，再合并成最终产物，最多 12 批，可随时取消。
- **续接合并**：Claude 对话超出上下文后会新建会话文件，CCP 把同项目、同标题、首条消息为续接提示且时间紧贴的会话合并成一行，Token 累加，副标题显示“N 段续接”。
- **项目口径**：Claude 与 Codex 的“项目”都取项目文件夹（git 仓库根）的名字，git worktree 归并到主仓库。
- **质检与追溯**：Skill / 工作流 / Prompt 自动质检并最多重试 2 次；失败会显示带稳定编码的中文原因，如 `ai.provider-network`、`ai.provider-unavailable`、`ai.provider-auth`。
- **你的模型，你的数据**：使用你在“供应商与路由”里配置的模型；蒸馏时选中的会话内容会发送给该供应商，请确认你信任它。也可选“离线回退”，此时不读取正文、不联网。

### Claude Desktop Computer Use（MCP）

让 Claude Desktop 拥有类似 Codex 的“看屏幕、动鼠标键盘”能力。CCP 自带一个本地 stdio MCP 服务（`claude-codex-pro.exe --mcp-computer-use`，不新增独立可执行文件），在 Claude Desktop 配置里注册为 `claude-codex-pro-computer-use`。

- **10 个工具**：`screenshot`、`click`、`move_mouse`、`drag`、`drag_path`、`scroll`、`type_text`、`press_keys`、`cursor_position`、`wait`。
- **一笔拖拽**：`drag_path` 按住左键依次经过 2~200 个点后只松开一次，适合画曲线、圆和签名；`drag` 与 `drag_path` 中途出错也会先松开左键，不会卡键。
- **截图**：仅主显示器，缩放到不超过 1280×800 的 JPEG；所有坐标使用截图像素，服务内部换算为真实屏幕坐标，越界坐标会被拒绝。
- **安全**：总开关默认关闭，每次调用前重新读取；把鼠标甩到主屏左上角触发**急停**，下次动作被拒绝并自动关闭开关；诊断日志记录工具名和坐标，`type_text` 只记录字数。
- **配置写入**：开启时写入所有常规 Claude Desktop 配置，以及已存在的 `Claude-3p`（API / 开发模式）配置，写前备份；无法解析的配置拒绝覆盖。切换开关后需要完全退出并重启 Claude Desktop。
- **入口**：管理工具「设置」页的独立 **Computer Use** 标签，含开关、注册状态和最近调用记录。
- **平台**：Windows（SendInput + GDI 截图）与 macOS（CoreGraphics）。macOS 路径目前只做过编译级检查，未在真机上验证。
- **注意**：会移动真实鼠标，截图可能包含屏幕上的敏感信息，使用前请整理好窗口。

实验性：`uia` 模块提供 Windows UI Automation 的元素查找、Pattern 操作和键盘输入，目前仅供内部测试，**尚未接入上述 MCP 工具**。

### 插件、Skills 与 MCP 统一清单

- **一行一个资产**：Claude 与 Codex 的 MCP、Skills、插件合并成一张清单，同一资产只显示一行，行尾的应用图标表示它在哪个应用里启用，点击只改对应应用。
- **完整发现**：不只显示 CCP 自己管理的条目，而是检测本机能解析到的全部资产，并给出“已启用”计数和来源路径。
- **新增 MCP / Skill**：新增 MCP 会同步写入 Claude 配置与 `~/.codex/config.toml`；Skill 描述按 YAML frontmatter 解析。

### 插件中心与 Ponytail

插件中心把多个来源统一成一个目录视图：Claude 官方插件市场、Claude Desktop MCP 配置项、GitHub MCP Registry、Awesome Claude Code、OpenAI Codex Plugins 仓库、Ponytail 多工具插件和可识别的 Skill bundle。每个条目展示来源、分类、作者、许可证、安装状态、风险提示、依赖要求、安装命令预览和配置 diff。

- 官方 Claude 插件通过 `claude plugin marketplace add/install`；Codex 插件通过 `codex plugin marketplace add/list/add`。
- Claude Desktop MCP 写入 `claude_desktop_config.json`，写入前备份。
- 未知社区 MCP 默认只展示，不自动执行脚本；Skill bundle 只有识别到结构时才安装。
- 已集成 [DietrichGebert/ponytail](https://github.com/DietrichGebert/ponytail)（Codex 中为 `ponytail@ponytail`）：支持 Codex、Claude Code、GitHub Copilot CLI、Claude Desktop MCP / 组织插件 / MCPB（`.mcpb` 安装包，交给 Claude Desktop 官方确认流程）；Ponytail 的 Codex hooks 需要预览后单独确认才写入信任状态。
- 可下载 OpenAI 官方 `openai/plugins` 仓库，限制体积、安全解压（防 zip path traversal），校验 `.agents/plugins/marketplace.json` 与每个插件目录的 `.codex-plugin/plugin.json` 后再注册到 `[marketplaces.openai-curated]`。

### 会话管理

- **按项目分组**：Codex 风格的项目 / 会话列表，点开查看完整上下文（分页加载，长消息自然撑开）。
- **数据源隔离**：Codex 读本地 SQLite / rollout，Claude 读 `~/.claude/projects` 等真实数据源，互不混用。
- **修复与迁移**：Provider Sync 在切换供应商后修复历史会话可见性；支持删除会话、导出 Markdown、移动项目归属；同时识别新版 `~/.codex/sqlite/*.db` 与旧版 `state_5.sqlite`。

### Codex 启动与增强

通过外部启动器启动 Codex，自动处理 CDP / helper 连接，并在 Codex 页面注入顶部状态标识。不修改 Codex 官方安装文件。

- 插件入口与插件市场入口解锁；适配新版插件安装通道（`vscode://codex/list-plugins`、`vscode://codex/plugin/install`）。
- 服务等级控制入口、图片覆盖配置、Codex Goals 配置写入。
- 会话滚动位置恢复、会话时间线与会话视图增强、原生菜单位置调整。
- Computer Use Guard：减少危险自动化误触。

### Claude Desktop 管理与中文

- 启动 / 聚焦官方 Claude Desktop，打开 DevTools，新建对话，向其粘贴草稿或提交文本。
- 检测安装位置、进程状态和完整性；官方 MSIX / CDP 被阻断时如实展示诊断，不伪装成已注入。
- **两条中文路径**：
  - **Claude 中文包装窗口**：独立 WebView 加载 `https://claude.ai/new`，创建阶段注入中文覆盖脚本，不改官方安装文件，推荐优先使用。
  - **一键汉化（资源补丁）**：可选的本机补丁，参考 `Jyy1529/claude-desktop_win-zh_cn` 的公开资源，写入 `zh-CN.json`、locale 配置和必要的前端语言支持；执行前备份，提供还原入口，需要你明确触发。

### 主题与外观

- 独立一级导航，三列主题卡片展示已安装主题，Codex 默认主题固定第一项；“CCP 外观”和“Codex 主题”是两个独立视图。
- 无代码 DIY 工作台：可视化调整玻璃透光度、模糊、圆角、字号和本地背景图，实时预览、保存、再次编辑。
- 从官方 GitHub 主题库按需下载、导入、预览、应用、删除和恢复默认；精选主题包与制作指南见 [`Theme/`](Theme/)。
- 安装主题时先在临时目录验证，再原子替换并保留上一版本；视觉注入与汉化、模型标识注入相互隔离。
- CCP 背景图库可保存、切换、删除多张本地高清背景，恢复默认不会清空图库。

### 系统提示词与指令模板

- 独立一级导航，紧凑卡片管理通用、破甲、逆向分析等指令模板；内置五套 Markdown 模板，位于 [`assets/system-prompts/`](assets/system-prompts/)。
- 新增、编辑、删除、导入 Markdown，或通过 URL / GitHub 地址同步。
- “保留原提示词”和“替换原提示词”两种启用方式，清楚显示当前生效状态。
- 写入 `~/.codex/config.toml` 前自动备份，并检测外部修改，避免静默覆盖其他工具或你手工更新的配置。
- 内置“使用方式”教程，源文件为 [`ccp-deepseek-guide.md`](apps/claude-codex-pro-manager/src/content/ccp-deepseek-guide.md)。

### 脚本、Zed Remote、Worktree 与自恢复

- **脚本市场**：刷新、下载安装、启用 / 禁用、删除用户脚本，构建已启用脚本 bundle，通过 Codex 注入扩展前端能力。
- **Zed Remote**：识别 Zed 安装与 SSH host / user / port，从 Codex 的全局状态和线程上下文解析远程项目，构造 `zed://ssh/...` 链接，支持默认 / 复用窗口 / 新窗口 / 追加到当前窗口。
- **Upstream Worktree**：读取 remote、branch、worktree 列表，从最新远程跟踪分支创建 worktree，校验分支名和 base branch，避免从过期本地 HEAD 派生任务分支。
- **Watcher 与自恢复**（Windows）：检测 Codex 进程与 CDP 端口，恢复失效 launcher，可启用、禁用、安装、卸载。

### 安装维护与自动发布

- 安装 / 卸载入口、修复快捷方式与后端配置、检查更新、下载 Release 资产并启动安装器、读取最新日志、复制诊断信息、重置设置。
- **自动发布**：`Auto release installers` 在 `main` push 或手动触发后自动计算 `V0.01` 系列版本、创建 tag、构建 Windows 安装包与 macOS x64 / arm64 DMG，并上传 `latest.json`；Release 的“更新内容”会自动列出本次包含的提交摘要。版本按 `V0.01 → V0.02 → … → V0.99 → V1.00` 递增。

## 界面预览

上方四张图分别是总览、Agent 概览、蒸馏工作台、插件 Skills 与 MCP。完整页面包括：概览（总览 / Agent 概览）、供应商与路由、蒸馏工作台、主题中心、系统提示词、会话、插件 Skills 与 MCP、设置（含 Computer Use）。

## 安全与隐私

**原则**

- **本地优先**：配置、插件记录、日志和备份都优先落在本机；用量采集只读，会话 ID 匿名化。
- **可审查**：安装插件、写入 MCP、信任 hooks、修改配置前展示命令或 diff。
- **可回退**：写入关键配置前尽量备份；Claude 中文资源补丁提供还原入口。
- **不静默信任第三方**：Ponytail / Codex hooks 需要单独审查和信任。
- **不伪装能力**：无法自动安装或需要人工确认的功能会明确标记。

**边界**

- Computer Use 默认关闭；会移动真实鼠标和键盘，鼠标甩到主屏左上角即可急停。
- 不静默修改 Claude Desktop 私有插件库；不自动执行未知社区 MCP 安装脚本；不把第三方 GitHub 内容默认当作可信代码执行。
- 不把 API key、Bearer token 或完整鉴权配置写入普通日志。
- Claude 中文包装窗口不修改官方 Claude Desktop 文件；一键汉化是你明确触发的本机补丁，执行前备份，可还原。
- CCP 只管理本机配置与第三方 API，不接管官方账号、订阅或支付。

**数据会发往哪里**：用量看板不联网。只有你主动使用蒸馏、连通性测试、下载主题 / 插件 / 更新时，才会向你配置的供应商或对应来源发起请求；蒸馏时选中的会话内容会发送给你所选的模型供应商。

## 数据位置

| 内容 | 位置 |
| --- | --- |
| Codex 配置 / 登录状态 | `~/.codex/config.toml`、`~/.codex/auth.json` |
| Codex 数据库 | 优先 `~/.codex/sqlite/*.db`，回退旧版 `~/.codex/state_5.sqlite` |
| Codex 插件仓库缓存 / skills | `~/.codex/.tmp/plugins`、`~/.codex/skills` |
| Claude Desktop MCP 配置 | Windows 通常为 `%APPDATA%\Claude\claude_desktop_config.json` |
| Claude Desktop 3P（API）配置 | Windows 通常为 `%LOCALAPPDATA%\Claude-3p`；MSIX 版在 `%LOCALAPPDATA%\Packages\Claude_*\LocalCache\Roaming\Claude-3p` |
| CCP 状态 | `~/.claude-codex-pro/` |
| 蒸馏任务与候选 | `~/.claude-codex-pro/aitracker/` |
| Provider Sync 备份 | `~/.codex/backups_state/provider-sync` |

## 常见问题

### Codex 里没有看到增强标识
确认是从 `Claude Codex Pro` 入口启动 Codex，而不是直接启动原始 Codex。仍然没有显示时，打开管理工具查看诊断和日志，重点检查 helper 端口、CDP 连接和 `renderer.script_loaded` 记录。

### Codex 模型菜单由谁负责？
Codex 模型菜单、可见模型和当前选择均由 Codex 原生客户端管理。CCP 不注入模型候选、不追加“CCP 模型增强”组，也不覆盖请求中的 `model`、`model_slug` 或 `modelId`。模型没有显示时，请在 Codex 自身的登录状态、版本和配置，以及供应商上游模型目录中排查；CCP 的供应商模型列表仅用于配置和路由管理。

### Claude 没有变成中文
优先使用 `打开 Claude 中文窗口`，它是独立 WebView 包装窗口，不是官方 Claude Desktop 原窗口。若使用资源补丁，请确认 Claude Desktop 已完全退出、安装目录可写，失败时查看补丁状态或执行还原。

### 蒸馏失败了怎么办？
历史里会显示带稳定编码的原因：`ai.provider-network` 表示连不上（检查 Base URL 与网络），`ai.provider-unavailable` 表示上游 5xx（稍后重试或换模型），`ai.provider-auth` 表示鉴权失败（检查 API Key），`ai.provider-invalid-response` 表示模型只返回了推理内容或空内容（换模型）。素材很长时会分批处理，任务详情会显示“已处理 x / n 批”。

### Computer Use 开了但 Claude Desktop 里看不到工具
开关只负责把 MCP 写进 Claude Desktop 的配置。写完后需要**完全退出**并重启 Claude Desktop（含托盘）。API（3P）模式的配置只有在 `Claude-3p` 配置文件已存在时才会被写入；仍看不到时，检查设置页「Computer Use」标签里的注册状态列出了哪些配置路径。

### 插件安装失败
先打开安装预览确认类型：Claude 官方插件需要 `claude` CLI；Claude Desktop MCP 需要写入 `claude_desktop_config.json`；Claude Desktop 本地组织插件需要开发模式和目录写入权限；Codex 插件需要 `codex` CLI；Ponytail hooks 需要单独审查和信任；社区 MCP 和 Skill 需要结构可识别。

### Release 里为什么只有 Source code？
安装包构建 job 成功但发布 job 失败时，GitHub 页面只会显示自动生成的源码压缩包。查看 `Auto release installers` 的 `Publish release and latest.json` 步骤即可。

### macOS 提示应用无法打开或已损坏
未签名或未公证的构建可能被 Gatekeeper 拦截。可在“系统设置 → 隐私与安全性”中允许打开；若仍提示已损坏：

```bash
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro.app
sudo xattr -rd com.apple.quarantine /Applications/Claude\ Codex\ Pro\ 管理工具.app
```

### 是否支持 Intel Mac？
支持。Release 分别提供 `macos-x64.dmg` 和 `macos-arm64.dmg`，Intel Mac 用 x64，Apple Silicon 用 arm64。

## 构建与开发

本项目是 Rust workspace + Tauri 管理工具 + Vite/React 前端。仓库根目录的 `package.json` 来自上游结构，不用于构建本项目；实际前端依赖在 `apps/claude-codex-pro-manager` 下安装和构建。

### 环境要求

- Git、Node.js 22 或更高版本、npm。
- Rust stable toolchain，包含 `cargo`、`rustc`、`rustfmt`。
- Windows：Visual Studio Build Tools / MSVC C++ 工具链；打安装包需要 NSIS（`choco install nsis -y`）；MSI 需要 WiX Toolset（Release 工作流会自动安装）。
- macOS：Xcode Command Line Tools；打 DMG 使用系统自带的 `sips`、`iconutil`、`codesign`、`hdiutil`；需要 `rustup target add x86_64-apple-darwin aarch64-apple-darwin`。

### 安装依赖

```bash
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
```

存在匹配的 lockfile 时可使用 `npm ci`，CI 目前使用 `npm install --package-lock=false`。

管理器的 `vite:build` 会先运行 `renderer:test`（Codex 页面宿主探测的注入脚本测试），再执行 Vite 构建；`dev` 直接启动 Tauri；`build` 经 Tauri 的 `beforeBuildCommand` 走 `vite:build`。纯浏览器预览的 `vite:dev` 保持独立。普通构建不自动联网安装依赖。

### 本地开发启动

```bash
cd apps/claude-codex-pro-manager
npm run dev        # 由 Tauri CLI 启动管理工具并自动运行 Vite（http://localhost:1420）
npm run vite:dev   # 只调试前端页面
```

普通浏览器预览没有 Tauri 后端，涉及系统配置、进程、插件安装、Claude 汉化等按钮会返回预览或无法执行；真实功能请用 `npm run dev` 验证。

### 本地验证

提交前建议运行：

```bash
node scripts/release/verify-release-workflow.js
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo fmt --check
cargo test --workspace
cargo build --release
```

常用定向验证：

```bash
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml plugin_hub -- --nocapture
cargo test -p claude-codex-pro-core --manifest-path Cargo.toml relay_config -- --nocapture
cargo test -p claude-codex-pro-manager --manifest-path Cargo.toml --test windows_subsystem -- --nocapture
cargo test -p claude-codex-pro-manager --lib distill_pipeline     # 蒸馏流水线与分批
cargo test -p claude-codex-pro-core --lib claude_session_chain    # 续接链识别
```

### 生产二进制

```bash
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo build --release
```

产物为 `target/release/claude-codex-pro.exe`（macOS / Linux 无 `.exe` 后缀）。也可在管理工具目录运行 `npm run build` 构建统一桌面程序；正式安装包仍以仓库里的 NSIS / DMG 脚本为准。

### Windows 安装包

```powershell
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run check
npm --prefix apps/claude-codex-pro-manager run vite:build
cargo test --workspace
cargo build --release

New-Item -ItemType Directory -Force dist/windows/app | Out-Null
Copy-Item target/release/claude-codex-pro.exe dist/windows/app/

$version = "0.12"
$makensis = "${env:ProgramFiles(x86)}\NSIS\makensis.exe"
if (-not (Test-Path $makensis)) { $makensis = "makensis" }
Push-Location scripts/installer/windows
& $makensis "/INPUTCHARSET" "UTF8" "/DVERSION=$version" ClaudeCodexPro.nsi
Pop-Location
```

输出 `dist/windows/claude-codex-pro-0.12-windows-x64-setup.exe`。MSI 使用 `scripts/installer/windows/tauri-msi.conf.json` 保留 Leila 资源，在管理器目录运行：

```bash
npm exec tauri build -- --bundles msi --config ../../scripts/installer/windows/tauri-msi.conf.json
```

### macOS DMG

```bash
# Apple Silicon
npm --prefix apps/claude-codex-pro-manager install --package-lock=false
npm --prefix apps/claude-codex-pro-manager run vite:build
rustup target add aarch64-apple-darwin
cargo build --release --target aarch64-apple-darwin
BINARY_DIR="$PWD/target/aarch64-apple-darwin/release" bash scripts/installer/macos/package-dmg.sh 0.12 arm64

# Intel Mac：把 aarch64-apple-darwin 换成 x86_64-apple-darwin，最后一个参数换成 x64
```

输出 `dist/macos/claude-codex-pro-0.12-macos-arm64.dmg`（Intel 为 `-macos-x64.dmg`）。本地脚本使用 ad-hoc codesign，不做 Apple Developer ID 签名或公证，本地 DMG 可能被 Gatekeeper 提示，按上文常见问题手动允许。

### GitHub Actions

- `.github/workflows/auto-release-installers.yml`：`main` push 或手动触发后自动发版。
- `.github/workflows/pr-build.yml`：PR、`main` push、手动触发时构建验证产物。
- `.github/workflows/release-assets.yml`：保留给手动 GitHub Release 使用。

自动发版：推送到 `main` → `scripts/release/next-release-tag.js` 读取现有 tag 并生成下一版 → 创建 tag 和 draft Release → Windows、macOS Intel、macOS Apple Silicon 三个 runner 分别构建 → 上传安装包 → 发布 Release → 生成并上传 `latest.json`。

## 项目结构

```text
apps/
  claude-codex-pro-launcher/          内部 launcher 库
  claude-codex-pro-manager/           统一 Tauri 主程序与管理工具（React/Vite 前端 + Rust 后端）
assets/inject/
  renderer-inject.js                  Codex 增强脚本
  claude-chinese-inject.js            Claude 中文包装窗口脚本
crates/
  claude-codex-pro-core/              启动、注入、配置、供应商、插件、Computer Use、更新、安装、bridge
  claude-codex-pro-data/              会话数据、用量采集、导出、Provider Sync
scripts/installer/
  windows/ClaudeCodexPro.nsi          Windows NSIS 安装器
  macos/package-dmg.sh                macOS DMG 打包脚本
spec/ acceptance/                     任务规格与验收标准
docs/                                 架构、评审与截图
```

## 反馈

- Issues：<https://github.com/DamonZS/Claude-Codex-Pro-Tool/issues>
- 讨论群二维码：<https://kcnl7iasnc4t.feishu.cn/wiki/O4T8wAodLiz05MkpqVkcoI7SnRd?from=from_copylink>

## 协议与规则

本仓库采用自定义源码可见限制协议，不是 OSI 认证开源协议。未经 DamonZS 或授权维护者书面允许，禁止以任何方式修改、发布、分发、改名、重打包或隐藏本项目来源，包括人工修改、AI 辅助修改、脚本、codemod、批量替换、自动化重写、二进制补丁和元数据改写。

作者信息、仓库地址、版权声明、产品名称、品牌、发布者、赞助或支付身份、协议和规则文件不得被删除、替换、隐藏或弱化。

这些限制不约束 DamonZS、仓库所有者、授权维护者以及在其指令下工作的 AI 助手、脚本、CI、codemod、格式化工具或自动化工具。官方项目后续开发可以继续使用 AI 和自动化能力。

授权维护者名单见 [MAINTAINERS.md](MAINTAINERS.md)。详见 [LICENSE](LICENSE) 和 [RULES.md](RULES.md)。

## 说明

Claude Codex Pro Tool 是外部增强工具，不是 OpenAI、Anthropic、Claude 或 Codex 的官方项目。官方应用更新后，如果页面结构、协议、CLI、插件格式或配置路径变化，本项目的注入脚本和适配逻辑可能需要同步更新。

