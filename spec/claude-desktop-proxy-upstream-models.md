# Claude Desktop 代理上游与模型列表修复

## 背景

Claude Desktop 第三方供应商页面已写入本地代理地址 `http://127.0.0.1:<port>/claude-desktop`，但测试推理时本地代理返回 502，错误为 `Claude Desktop 上游 Base URL 不能为空`。同时 Claude 的模型列表与期望不一致，缺少 `claude-fable-5`，并且默认写入时可能只包含单个模型。

## 目标

- Claude Desktop 本地代理必须在没有显式上游配置时使用默认上游 `https://api.toporeduce.cn`，避免空 Base URL 502。
- Claude Desktop 模型发现与 profile 写入默认展示四个模型：`claude-fable-5`、`claude-haiku-4-5`、`claude-opus-4-8`、`claude-sonnet-4-6`。
- 供应商页面后续填入真实 key 后，代理继续使用现有 key 读取逻辑，不在日志或测试中泄漏 key。

## 非目标

- 不验证第三方 API Key 是否真实可用。
- 不修改 Claude 官方文件。
- 不重做供应商页面布局。

## 功能要求

- `claude_desktop_models_response("")` 返回上述四个安全 Claude 模型 ID，顺序与 Claude UI 参考一致。
- `inferenceModels` 在未传入模型列表时仍写入默认四模型列表。
- Claude 模型到上游模型映射支持 `fable`，并在用户未提供模型列表时不会返回空映射。
- Claude Desktop 代理的上游 Base URL 兜底为 `https://api.toporeduce.cn`，但不能把本地代理 URL 当作上游。

## 验收范围

- `crates/claude-codex-pro-core/src/protocol_proxy.rs`
- `crates/claude-codex-pro-core/src/plugin_hub.rs`
- `crates/claude-codex-pro-core/src/claude_desktop_provider.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- 对应 Rust 测试
