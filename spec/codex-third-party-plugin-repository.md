# Codex 第三方插件仓库接入

## 背景

用户要求将 `hashgraph-online/awesome-codex-plugins` 加入 Codex 的第三方插件仓库。该仓库提供 Codex marketplace 结构，Codex CLI 的真实写入格式为：

```toml
[marketplaces.awesome-codex-plugins]
source_type = "git"
source = "https://github.com/hashgraph-online/awesome-codex-plugins.git"
ref = "main"
sparse_paths = [".agents/plugins", "plugins"]
```

## 目标

本次要完成：

- 管理工具的 Codex 插件仓库修复逻辑同时配置 OpenAI 官方仓库和 `awesome-codex-plugins` 第三方仓库。
- 工具与插件页的 Codex 插件仓库状态卡能显示第三方仓库配置状态。
- 不自动安装第三方仓库里的具体插件。

本次不包含：

- 自动信任第三方插件 hooks。
- 自动启用第三方插件。
- 删除或覆盖用户已有 Codex 插件配置。

## 用户视角描述

用户点击“修复 Codex 插件仓库”后，管理工具应确保 Codex `config.toml` 中存在 `awesome-codex-plugins` marketplace 配置。用户回到 Codex 插件页并重启/刷新 Codex 后，应能让 Codex 自身读取该第三方 marketplace。

## 功能要求

- 写入 `[marketplaces.awesome-codex-plugins]`。
- `source_type` 必须为 `git`。
- `source` 必须为 `https://github.com/hashgraph-online/awesome-codex-plugins.git`。
- `ref` 必须为 `main`。
- `sparse_paths` 必须为 `[".agents/plugins", "plugins"]`。
- 状态检查必须把该第三方仓库纳入 `needsRepair`。
- 修复逻辑必须具有幂等性，重复执行不产生无意义改动。

## UI / 交互要求

- Codex 插件仓库状态卡显示第三方仓库名称、source 类型、注册状态和仓库 URL。
- 修复按钮继续使用现有“修复 Codex 插件仓库”按钮，不新增复杂流程。

## 数据与接口要求

- 继续复用 `load_codex_plugin_marketplace_status`。
- 继续复用 `repair_codex_plugin_marketplace`。
- 返回结构允许增加仓库列表字段，前端向后兼容。

## 技术约束

- 不依赖固定本机 Codex 安装路径。
- 不修改 Claude 中文注入脚本。
- 不执行第三方插件安装脚本。
- 不新增 npm 依赖。

## 交付范围

- `crates/claude-codex-pro-core/src/codex_plugin_marketplace.rs`
- `apps/claude-codex-pro-manager/src/App.tsx`
- `apps/claude-codex-pro-manager/src-tauri/tests/windows_subsystem.rs`
- `spec/codex-third-party-plugin-repository.md`
- `acceptance/codex-third-party-plugin-repository.md`
