# 工具与插件页插件仓库自动修复

## 背景

用户反馈“工具与插件”页不需要在应用内显示插件目录和安装流程，也不需要“打开仓库”按钮。页面应该告诉用户 Codex 与 Claude 的插件仓库是否已经配置，并提供能真实修复配置的按钮。

当前问题：

- Codex 与 Claude 仓库卡片仍保留打开外部仓库或 deep link 的按钮，容易让用户误以为只是跳转而不是修复。
- Claude 插件仓库状态缺少自动写入能力，开发模式配置写入时没有同步写入已知插件仓库。
- 用户记忆中仓库不止一个，状态卡需要能表达多仓库配置。
- Codex / Claude 的 MCP、Skills、Plugins 条目开关显示重叠，编辑按钮图标不够清晰。

## 目标

本次要完成：

- Codex 仓库卡只保留刷新与“修复插件仓库”类操作，不再显示“打开仓库”按钮。
- Claude 仓库卡只保留刷新与“修复插件仓库”类操作，不再显示“打开 Claude 官方插件仓库”按钮。
- Codex 修复按钮必须继续执行真实的 OpenAI 插件仓库下载、校验与 `~/.codex/config.toml` 注册。
- Claude 修复按钮必须真实写入开发配置中的已知插件仓库，而不是只打开 deep link。
- Claude 一键开发模式写入配置时，也必须同步写入插件仓库配置。
- Claude 插件仓库配置至少包含 Anthropic 官方仓库与 Ponytail 仓库。
- 仓库状态结构应能显示多仓库配置状态。
- Codex / Claude 工具与插件卡片中的开关必须正常显示，不得与下方按钮或图标重叠。
- 编辑按钮改为更清晰的倾斜笔图标。

本次不包含：

- 自动安装具体 Claude 官方插件。
- 自动信任第三方 hooks。
- 删除插件中心后端能力。
- 重构供应商、模型、启动器或发布流程。

## 用户视角描述

用户进入“工具与插件”页后，应看到 Codex 与 Claude 两张插件仓库状态卡。用户可以刷新状态，也可以点击“修复插件仓库”让应用直接修复本地配置。用户不需要离开应用去打开 GitHub 或 Claude deep link。

在 Codex / Claude 的 MCP、Skills、Plugins 列表中，启用开关、编辑按钮和删除按钮应排列稳定、大小一致、无重叠。编辑按钮应表现为一个倾斜笔图标。

## 功能要求

- 页面必须显示 Codex 插件仓库状态卡。
- 页面必须显示 Claude 插件仓库状态卡。
- 页面不得显示插件目录市场浏览区。
- 页面不得显示插件详情、安装预览、安装/卸载按钮等应用内插件安装 UI。
- Codex 仓库状态卡不得显示打开 OpenAI 仓库按钮。
- Claude 仓库状态卡不得显示打开 Claude 官方仓库按钮。
- Codex 修复按钮必须调用 `repair_codex_plugin_marketplace`。
- Claude 修复按钮必须调用真实写入配置的后端命令，不得调用只打开链接的命令。
- Claude 开发模式配置写入流程必须确保插件仓库配置同步写入。
- Claude 插件仓库状态应反映多个仓库是否已写入。
- Codex / Claude 上下文条目行的 toggle、编辑、删除控件必须对齐且不重叠。

## UI / 交互要求

- Codex 和 Claude 插件仓库状态卡在工具页顶部并列或连续展示。
- 每个状态卡至少展示：整体状态、配置方式、仓库列表、目标路径或配置路径。
- “修复插件仓库”按钮文案必须明确区分 Codex 与 Claude。
- 修复按钮点击后应刷新对应状态，并展示真实结果消息。
- 编辑按钮使用 lucide 的笔类图标，视觉上应为倾斜笔，不使用 `PencilRuler`。
- toggle 视觉应为紧凑开关，启用态不过度发光，不得撑破行高。

## 数据与接口要求

- Codex 继续复用 `load_codex_plugin_marketplace_status` 与 `repair_codex_plugin_marketplace`。
- Claude 新增或复用一个真实写入配置的 Tauri command，用于修复插件仓库配置。
- Claude marketplace 状态结构应能返回仓库列表，每个仓库至少包含 label、repository、url、configured。
- Claude 配置写入应使用 JSON 结构化写入，避免字符串拼接。

## 技术约束

- 不引入新依赖。
- 不执行第三方安装脚本。
- 不自动安装具体插件。
- 写入用户本地 Codex/Claude 配置前沿用现有备份和原子写入能力。
- 保持最小必要改动。

## 交付范围

- `spec/tools-page-plugin-repository-status-only.md`
- `acceptance/tools-page-plugin-repository-status-only.md`
- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src/styles.css`
- `apps/claude-codex-pro-manager/src/tauriBridge.ts`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/lib.rs`
- `crates/claude-codex-pro-core/src/plugin_hub.rs`
