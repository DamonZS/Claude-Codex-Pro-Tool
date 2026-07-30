# Codex Matt Pocock Skills 第三方仓库

## 背景

用户要求将 `mattpocock/skills` 加入管理工具维护的 Codex 第三方仓库。上游提供 `.claude-plugin/marketplace.json` 与完整 `skills/`，但没有 Codex 所需的 `.agents/plugins/marketplace.json`，不能直接作为 Codex marketplace 注册。

## 目标

- 管理工具下载 `https://github.com/mattpocock/skills` 的官方主分支快照。
- 生成 Codex 可识别的本地 `mattpocock-skills` marketplace。
- 将该本地 marketplace 纳入状态检查、自动修复和工具页仓库列表。
- 重复启动或修复保持幂等。

## 非目标

- 不自动安装或启用具体 Matt 技能。
- 不执行上游 npm 脚本、hooks 或其他程序。
- 不修改上游技能内容。
- 不自动信任第三方代码。

## 用户视角描述

用户启动管理工具后，Codex 插件仓库自动修复流程会下载并注册 Matt Pocock Skills 仓库。工具与插件页显示仓库配置状态；具体技能仍由用户在 Codex 中选择安装。

## 功能要求

- marketplace 名称为 `mattpocock-skills`，来源显示为 `https://github.com/mattpocock/skills`。
- 本地快照包含 `.agents/plugins/marketplace.json`、`plugins/mattpocock-skills/.codex-plugin/plugin.json` 和 `plugins/mattpocock-skills/skills/engineering/ask-matt/SKILL.md`。
- Codex 清单使用本地插件路径与 `authentication = "ON_INSTALL"`。
- `[marketplaces.mattpocock-skills]` 使用 `source_type = "local"`，指向管理工具生成的快照。
- 仓库缺失、快照无效或配置未注册时，整体状态必须标记 `needsRepair`。
- 修复流程不得写入 `[plugins.*]` 启用项。

## UI / 交互要求

- Codex 仓库状态列表显示 `Matt Pocock Skills 仓库`、`mattpocock-skills` 和官方 GitHub URL 或本地快照路径。
- 继续复用现有刷新、修复及启动自动注册流程，不新增按钮或 Tauri command。

## 技术约束

- 不新增 npm 或 Rust 依赖。
- 下载大小和超时必须受限。
- ZIP 解压必须复用现有路径穿越防护。
- 不记录或处理任何凭据。

## 交付范围

- Codex marketplace 核心下载、快照、状态、注册与测试。
- Manager 预览数据、常量、回退状态和 UI 回归测试。
- 本规格及匹配验收标准。
