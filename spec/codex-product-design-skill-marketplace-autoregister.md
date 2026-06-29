# Codex Product Design Skill 插件仓库自动注册

## 背景

用户要求将 `DKeken/codex-skills-alternative` 集成到管理工具中，并在管理工具默认启动时自动注册到 Codex 插件第三方仓库。该仓库提供 `product-design` orchestrator 以及 `design-*` 子技能，可作为 Codex Product Design Skill 的社区 fallback。

## 目标

本次要完成：

- 管理工具的 Codex 插件仓库修复逻辑纳入 `DKeken/codex-skills-alternative`。
- 管理工具下载该仓库 zip 后生成当前 Codex 可识别的本地 marketplace 快照。
- 管理工具启动后自动检查并注册 Codex 插件仓库配置，无需用户先进入工具与插件页手动点击。
- 工具与插件页的 Codex 仓库状态展示 `codex-skills-alternative` 的配置状态。
- 保持写入幂等，重复启动或重复修复不产生无意义配置变化。

本次不包含：

- 自动安装或启用 `codex-skills-alternative` 里的具体插件。
- 自动信任第三方 hooks。
- 执行第三方仓库里的安装脚本。
- 修改 Claude 插件仓库、Claude 中文注入或 Codex 注入脚本。

## 用户视角描述

用户启动管理工具后，管理工具会自动确保 Codex `config.toml` 中存在 Product Design Skill 第三方插件仓库配置。用户进入 Codex 插件页并刷新/重启 Codex 后，可由 Codex 自身读取该 marketplace。用户仍需在 Codex 内确认具体插件安装和 hooks 信任。

## 功能要求

- Codex 仓库修复逻辑必须写入 `[marketplaces.codex-skills-alternative]`。
- `source_type` 必须为 `local`。
- `source` 必须指向管理工具生成的本地 marketplace 快照目录。
- 本地快照必须包含 `.agents/plugins/marketplace.json`。
- 本地快照必须包含 `plugins/codex-skills-alternative/.codex-plugin/plugin.json`。
- 本地快照必须包含 `plugins/codex-skills-alternative/skills/product-design/SKILL.md`。
- 规范化后的 marketplace 必须使用 `authentication = "ON_INSTALL"`，避免当前 Codex CLI 拒绝上游 `NONE` 枚举。
- 状态检查必须把该仓库纳入 `needsRepair` 判断。
- 启动自动注册只修复 Codex 插件仓库，不自动修复 Claude 仓库，不弹确认框。
- 修复逻辑不得写入 `[plugins.*]` 第三方插件启用项。

## UI / 交互要求

- Codex 插件仓库状态卡应显示 `Product Design Skill 仓库`、`codex-skills-alternative` 和 GitHub URL。
- 现有“刷新 Codex 插件仓库”和“修复 Codex 插件仓库”按钮继续可用。
- 启动自动注册失败时，只通过已有 notice / 状态刷新呈现，不阻塞管理工具打开。

## 数据与接口要求

- 继续复用 `load_codex_plugin_marketplace_status`。
- 继续复用 `repair_codex_plugin_marketplace`。
- 不新增 Tauri command。
- 允许现有 marketplace status 的 `repositories` 列表增加一项。

## 技术约束

- 不依赖本机固定 Codex 安装路径。
- 不新增 npm 或 Rust 依赖。
- 不执行第三方仓库脚本。
- 仅写入 Codex `config.toml` 的 marketplace 配置和管理工具自己的本地 marketplace 缓存目录。

## 交付范围

- `crates/claude-codex-pro-core/src/codex_plugin_marketplace.rs`
- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src/tauriBridge.ts`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- 本规格与对应验收标准
