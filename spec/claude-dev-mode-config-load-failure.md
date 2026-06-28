# Claude 开发配置加载失败回归修复

## 背景

用户点击“Claude 一键开发模式”后，管理工具提示 Claude Desktop 开发模式和本地模型代理已经写入并验证成功，但 Claude Desktop 启动后显示“无法加载配置”，并弹出“无法更新已保存的配置”。本机 `Claude-3p/configLibrary` 中已经存在 gateway profile 文件，但 `_meta.json` 的 `entries` 为空，导致 Claude 在 `deploymentMode=3p` 时找不到当前应用的配置 profile。

## 目标

- 修复 Claude 一键开发模式写入后 `_meta.json` 与 profile 文件不一致的问题。
- 保证即使 API Key 暂时为空，也能进入 Claude 第三方开发模式配置界面。
- 保证 profile JSON 只写入 Claude Desktop 可接受的字段形态，不因默认模型列表或空配置让 Claude 前端拒载。
- 保留现有本地代理写入逻辑和真实端口避让逻辑。

## 非目标

- 不修改用户的真实 API Key 内容。
- 不重做供应商页面或代理转发架构。
- 不删除用户已有 Claude/Codex 数据。
- 不修改 Claude 汉化资源逻辑。

## 用户视角描述

用户在概览页点击“Claude 一键开发模式”后，管理工具写入 `deploymentMode=3p`、gateway profile 和 `_meta.json`。完全退出并重启 Claude Desktop 后，Claude 应进入第三方开发模式页面，而不是停留在“无法加载配置”白屏。

## 功能要求

- 当存在 provider/base URL 时，一键开发模式必须写入 profile 文件。
- 只要写入了 profile 文件，`_meta.json` 必须包含当前 profile 的 `entries` 项，并将 `appliedId` 指向该 profile。
- API Key 允许暂时为空，以支持先进入界面、后续供应商页补 Key 的流程。
- 空模型列表不得强制写入默认 `inferenceModels`；只有用户显式提供模型列表时才写入模型列表。
- 配置状态检测不得因为 API Key 为空而把 `_meta` 清空或显示未写入。

## UI / 交互要求

- 成功提示仍说明已写入开发模式和本地代理状态。
- 失败时返回可理解的中文错误，不能只显示成功 toast。

## 数据与接口要求

- 目标路径包括普通 `%LOCALAPPDATA%\Claude-3p` 和 MSIX package `LocalCache\Roaming\Claude-3p` 变体。
- `claude_desktop_config.json` 必须保留已有字段，仅更新 `deploymentMode=3p`。
- profile 文件必须包含 gateway 必需字段：`inferenceProvider`、`inferenceGatewayBaseUrl`、`inferenceGatewayAuthScheme`、`inferenceGatewayApiKey`、`disableDeploymentModeChooser`、`coworkEgressAllowedHosts`。
- `_meta.json` 必须保留非本工具 profile entries，只更新本工具 profile entry 与 `appliedId`。

## 技术约束

- 不引入新依赖。
- 最小修改 `plugin_hub.rs` 与相关测试。
- 不记录 API Key、Bearer token 或完整授权材料。
- 保持 GitHub Actions 可构建。

## 交付范围

- `crates/claude-codex-pro-core/src/plugin_hub.rs`
- `crates/claude-codex-pro-core/src/claude_desktop_provider.rs`
- 相关 Rust 测试
- 本规格和对应验收文档
