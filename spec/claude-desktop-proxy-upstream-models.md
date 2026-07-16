# Claude Desktop 代理上游与模型发现修复

## 背景

Claude Desktop 第三方供应商页面已写入本地代理地址 `http://127.0.0.1:<port>/claude-desktop`，但测试推理时本地代理返回 502，错误为 `Claude Desktop 上游 Base URL 不能为空`。同时 Claude 的模型列表与期望不一致，缺少 `claude-fable-5`，并且默认写入时可能只包含单个模型。

供应商编辑器的“获取模型”还存在一条独立故障链：`RelayProfile.api_key` 不参与序列化，前端传回的 Key 实际位于 `authContents` 或 `configContents`，但模型发现直接读取了空的 `api_key` 字段。当前实现还会在标准 `/v1/models` 返回空数组时读取供应商公开的 `/api/pricing`，把价格目录显示成“当前 Key 已获取模型”。价格目录不代表当前凭据可见或可调用的模型，因此模型发现、用户保存的模型映射和模型实际可调用性必须分别处理，不能互相冒充结果。

Claude / Claude Desktop 的 Anthropic Messages 配置还存在鉴权字段语义丢失：`ANTHROPIC_AUTH_TOKEN` 与 `OPENAI_API_KEY` 表示 Bearer 中转鉴权，`ANTHROPIC_API_KEY` 表示原生 `x-api-key` 鉴权。模型发现与消息代理不能一律只发 Bearer，也不能无条件同时发送两种凭据头；部分中转会因额外的 `x-api-key` 直接返回 500。请求必须按实际解析到的 Key 字段选择鉴权方式，Messages 请求同时发送 `anthropic-version`。

## 目标

- Claude Desktop 本地代理必须在没有显式上游配置时使用默认上游 `https://api.toporeduce.cn`，避免空 Base URL 502。
- Claude Desktop 模型发现与 profile 写入默认展示四个模型：`claude-fable-5`、`claude-haiku-4-5`、`claude-opus-4-8`、`claude-sonnet-4-6`。
- 供应商页面后续填入真实 key 后，代理继续使用现有 key 读取逻辑，不在日志或测试中泄漏 key。
- 供应商编辑器“获取模型”必须使用 profile 的解析后 Key 请求远端目录，不读取已保存的 `modelList` 或模型映射作为发现结果。
- “获取模型”只以标准 `/v1/models` 为远端目录来源，禁止使用 `/api/pricing` 或其他公开价格目录补全结果。
- 远端目录存在模型不代表当前 Key 一定具有可用通道；实际推理失败必须保留上游原始状态语义，不能自动改写用户映射来伪造成功。
- Claude / Claude Desktop 必须严格使用各自已保存的活动供应商 ID，不能在 ID 失效时静默回退到同目标的第一个历史 Profile。
- Codex 协议转换必须原样透传请求模型；使用 `gpt-5.6-sol` 时不得改写为 `gpt-5.4`，也不得为一次代理请求额外发送另一模型的推理请求。
- Anthropic Messages 模型发现和 Claude Desktop 消息代理必须保留 Key 字段的鉴权语义：`ANTHROPIC_AUTH_TOKEN` / `OPENAI_API_KEY` 使用 Bearer，`ANTHROPIC_API_KEY` 使用原生 `x-api-key`；不通过重试重复发送推理请求。
- Claude Code / Claude Desktop 请求携带的 `?beta=true`、`anthropic-version` 与 `anthropic-beta` 必须按白名单透传到 Messages 上游，避免 Beta 客户端与非 Beta 响应流不匹配。
- Claude Messages 流必须增量校验 `content_block_start`、`content_block_delta`、`content_block_stop` 生命周期；网络拆包不能导致事件丢失，HTTP 200 不能直接等同于协议流成功。
- 对可安全推断类型的孤立文本或思考增量补齐空的块起始事件；无法可靠推断工具 ID/name 的孤立工具参数增量不得伪造工具块，并必须避免继续向客户端发送会触发 `Content block not found` 的孤立事件。
- 流诊断只记录事件类型、块索引、块类型、修复动作和计数，不记录提示词、响应正文、工具参数、API Key 或完整请求头。

## 非目标

- 不验证第三方 API Key 是否真实可用。
- 不修改 Claude 官方文件。
- 不重做供应商页面布局。
- 不自动保存、修改或替换用户模型映射。
- 不把本地默认模型、历史模型列表或映射值作为“获取模型”的远端发现结果。
- 不把 `/api/pricing` 等无需当前 Key 即可读取的公开目录作为可用模型发现结果。
- 不把模型目录请求、配置保存或页面加载当作推理请求；用户主动执行的连接测试仍作为独立操作处理。
- 不持久化或输出 Claude Messages 请求正文、响应正文、工具输入和完整 SSE 数据。

## 功能要求

- `claude_desktop_models_response("")` 返回上述四个安全 Claude 模型 ID，顺序与 Claude UI 参考一致。
- `inferenceModels` 在未传入模型列表时仍写入默认四模型列表。
- Claude 模型到上游模型映射支持 `fable`，并在用户未提供模型列表时不会返回空映射。
- Claude Desktop 代理的上游 Base URL 兜底为 `https://api.toporeduce.cn`，但不能把本地代理 URL 当作上游。
- `fetch_relay_profile_model_ids` 解析 `apiKey`、`authContents`、JSON/TOML `configContents` 中的真实 Key 及其字段来源。Codex/OpenAI、`ANTHROPIC_AUTH_TOKEN` 与 `OPENAI_API_KEY` 通过 Bearer 请求标准模型接口；`ANTHROPIC_API_KEY` 使用 `x-api-key` 与 `anthropic-version: 2023-06-01`。一次模型发现只选择一种鉴权方式。
- `/v1/models` 返回非空目录时直接使用该结果，不请求或合并 `/api/pricing`。
- `/v1/models` 返回 HTTP 200 但模型数组为空时，“获取模型”明确失败并指出标准接口没有返回可用模型。
- `/v1/models` 返回 401、403 或其他失败时保留标准接口失败，不得用公开目录掩盖鉴权或服务错误。
- 标准接口没有返回合格模型时，不回退到 `/api/pricing`、`modelList`、`modelMapping`、`modelMappingJson` 或本地默认模型。
- 返回信息和错误不得包含 API Key、Bearer token、URL query 或 fragment。
- Claude 实际推理的 503（例如 `No available channel for model`）作为当前 Key/用户组的通道可用性错误单独报告，不改变模型发现结果。
- `activeClaudeRelayId` / `activeClaudeDesktopRelayId` 非空时只允许精确匹配对应目标 Profile；ID 无效时返回“未配置供应商”，不得误用旧 Profile。
- 活动 ID 为空时，仅当对应目标恰好存在一个 Profile 才允许兼容历史配置并选择该 Profile；存在零个或多个候选时返回“未配置供应商”。
- `responses_to_chat_completions` 必须保持输入 `model` 字段逐字不变，包括 `gpt-5.6-sol` 等未来模型 ID。
- 正常 Responses 代理链每个入站请求只执行一次上游推理 POST；模型发现只允许请求模型目录端点，不发送推理正文。
- Claude Desktop Messages 代理在单次上游 POST 中按 Key 字段选择 Bearer 或 `x-api-key`，并发送 `anthropic-version: 2023-06-01`；不得同时发送两种凭据头，也不得因鉴权兼容自动重试或双发推理请求。
- Claude Messages 请求只透传允许的协议元数据：`beta=true` 查询参数、合法的 `anthropic-version` 与 `anthropic-beta`；不得透传客户端凭据覆盖管理工具选择的上游凭据。
- SSE 解析器必须跨任意网络 chunk 缓冲完整事件，同时支持 `\n\n` 与 `\r\n\r\n` 分隔；没有生命周期异常的事件保持原始字节和顺序。
- 对缺少 start 的 `text_delta`、`thinking_delta`、`signature_delta` 可分别补齐 `text` 或 `thinking` 起始块；对无法推断身份的 `input_json_delta` 只记录并抑制孤立事件，不生成虚假的 tool ID/name；孤立 stop 同样抑制。
- `message_stop` 前仍有活动块时按索引补齐 stop；流在事件中间断开或结束时仍有活动块时记录协议不完整，不得记录为普通 `stream_ok`。
- 每条异常诊断最多记录必要元数据并设置数量上限，防止日志失控增长；正常流只写汇总结果。

## 验收范围

- `crates/claude-codex-pro-core/src/protocol_proxy.rs`
- `crates/claude-codex-pro-core/src/plugin_hub.rs`
- `crates/claude-codex-pro-core/src/claude_desktop_provider.rs`
- `apps/claude-codex-pro-manager/src-tauri/src/commands.rs`
- `crates/claude-codex-pro-core/src/model_catalog.rs`
- `crates/claude-codex-pro-core/tests/model_catalog.rs`
- `crates/claude-codex-pro-core/src/settings.rs`
- `crates/claude-codex-pro-core/tests/protocol_proxy.rs`
- `crates/claude-codex-pro-core/src/launcher.rs`
- 对应 Rust 测试
