# 供应商导入与切换 Key 保留修复

## 背景

用户反馈供应商页面中多个从第三方导入的供应商不可用，点击“使用”没有实际效果，且第三方导入的配置没有带上 key。当前后端设计会在保存设置时避免把 API Key 持久化到 `apiKey` 字段，而是保留在 `authContents` 或 `configContents` 中；前端部分逻辑仍只检查 `apiKey`，导致有效配置被误判为空 key。

## 目标

本次要完成：

- 从第三方导入的 Codex 供应商，如果 key 存在于 `apiKey`、`authContents.OPENAI_API_KEY` 或 `configContents.experimental_bearer_token`，都必须被识别为有效。
- 点击“使用”或“保存并使用”时，不得仅因 `apiKey` 字段为空而拦截已有 key 的供应商。
- 前端重新生成供应商 `config.toml` / `auth.json` 时必须保留已存在的 key。
- ccswitch 导入时，如果 key 只存在于 Codex config 的 `experimental_bearer_token` 中，也必须带入导入结果。
- 导入时若供应商 ID 冲突并被自动改名，生成的 `config.toml` provider id 必须与新 ID 一致。

本次不包含：

- 改变后端“不持久化 `apiKey` 字段”的安全设计。
- 在日志、文档或测试输出中暴露真实用户 API Key。
- 重构供应商页面 UI 或供应商存储结构。
- 修改 Claude 中文注入脚本或清空用户记忆数据库。

## 用户视角描述

用户从第三方导入 Codex 供应商后，可以看到导入记录可用；点击“使用”会真实写入 Codex 配置。即使设置文件中看不到明文 `apiKey` 字段，只要 `auth.json` 或 `config.toml` 中有可用 key，该供应商就不应被误判为缺少 key。

## 功能要求

- 前端供应商标准化逻辑必须能从 `authContents` 中解析 `OPENAI_API_KEY`、`api_key` 或 `apiKey`。
- 前端供应商标准化逻辑必须能从 `configContents` 中解析 `experimental_bearer_token`。
- `switchCodexRelayProfile` 和 `saveAndSwitchDraft` 必须使用统一的 key 判断函数。
- `withSupplierGeneratedFiles` 不得在解析 key 前清空 `authContents` 和 `configContents`。
- ccswitch 后端导入必须支持从 config 的 active provider 中读取 `experimental_bearer_token`。

## UI / 交互要求

- 不新增页面或卡片。
- 保持现有按钮、文案和布局。
- 缺少真实 key 时仍应提示补全 API Key。
- 有真实 key 时点击“使用”不应被前端空 key 判断拦截。

## 数据与接口要求

- 输入来源包括前端 `RelayProfile`、ccswitch SQLite 中的 provider `settings_config`。
- 输出仍为现有 `RelayProfile` 结构。
- 不把真实 API Key 写入普通日志。
- 保存后仍由后端 normalization 负责最终安全存储。

## 技术约束

- 沿用当前 React/Tauri/Rust 架构。
- 不新增依赖。
- 不改变 `RelayProfile.api_key` 的 `skip_serializing` 行为。
- 不修改 `assets/inject/claude-chinese-inject.js`。

## 交付范围

- 供应商前端 key 解析、生成和切换判断修复。
- ccswitch 后端导入 token 兜底解析。
- 对应测试与本文档、验收文档。
