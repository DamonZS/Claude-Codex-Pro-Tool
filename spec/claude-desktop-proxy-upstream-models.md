# Claude Desktop 代理上游与模型发现修复

## 背景

Claude Desktop 第三方供应商页面已写入本地代理地址 `http://127.0.0.1:<port>/claude-desktop`，但测试推理时本地代理返回 502，错误为 `Claude Desktop 上游 Base URL 不能为空`。同时 Claude 的模型列表与期望不一致，缺少 `claude-fable-5`，并且默认写入时可能只包含单个模型。

供应商编辑器的“获取模型”还存在一条独立故障链：`RelayProfile.api_key` 不参与序列化，当前编辑 Key 没有稳定同步到 `authContents` 与 `configContents`，保存或重新打开后可能重新取到旧 Key。于是同一个供应商分组的 `/v1/models` 会以错误凭据返回 HTTP 200 空数组。当前编辑 Key 必须在保存、重新打开、应用配置、模型发现和消息代理中保持一致；模型发现只读取当前 Key 对应的标准模型目录。

Claude / Claude Desktop 的 Anthropic Messages 配置还存在鉴权字段语义丢失：`ANTHROPIC_AUTH_TOKEN` 与 `OPENAI_API_KEY` 表示 Bearer 中转鉴权，`ANTHROPIC_API_KEY` 表示原生 `x-api-key` 鉴权。模型发现与消息代理不能一律只发 Bearer，也不能无条件同时发送两种凭据头；部分中转会因额外的 `x-api-key` 直接返回 500。请求必须按实际解析到的 Key 字段选择鉴权方式，Messages 请求同时发送 `anthropic-version`。

## 目标

- Claude Desktop 本地代理必须在没有显式上游配置时使用默认上游 `https://api.toporeduce.cn`，避免空 Base URL 502。
- Claude Desktop 模型发现与 profile 写入默认展示四个模型：`claude-fable-5`、`claude-haiku-4-5`、`claude-opus-4-8`、`claude-sonnet-4-6`。
- Claude Desktop 所有真实对话（包括形状类似连接探针的请求）不得在发送前请求 `/v1/models`；模型目录只用于用户主动执行的“获取模型”，目录异常不能阻断真实对话。
- 供应商页面后续填入真实 key 后，代理继续使用现有 key 读取逻辑，不在日志或测试中泄漏 key。
- 供应商编辑器“获取模型”必须使用 profile 的解析后 Key 请求远端目录，不读取已保存的 `modelList` 或模型映射作为发现结果。
- “获取模型”只使用标准 `/v1/models`；HTTP 2xx 空数组必须明确报告当前 Key 的标准目录为空，不请求价格接口，也不以公开目录、历史列表或映射补全结果。
- 远端目录存在模型不代表当前 Key 一定具有可用通道；实际推理失败必须保留上游原始状态语义，不能自动改写用户映射来伪造成功。
- Claude / Claude Desktop 必须严格使用各自已保存的活动供应商 ID，不能在 ID 失效时静默回退到同目标的第一个历史 Profile。
- Codex 协议转换必须原样透传请求模型；使用 `gpt-5.6-sol` 时不得改写为 `gpt-5.4`，也不得为一次代理请求额外发送另一模型的推理请求。
- Anthropic Messages 模型发现和 Claude Desktop 消息代理必须保留 Key 字段的鉴权语义：`ANTHROPIC_AUTH_TOKEN` / `OPENAI_API_KEY` 使用 Bearer，`ANTHROPIC_API_KEY` 使用原生 `x-api-key`；不通过重试重复发送推理请求。
- Anthropic Messages 的“测试连接”只验证当前编辑 Profile 的 Base URL 网络可达性，不触发模型发现或推理请求。
- Claude Desktop 的真实 `/v1/messages` 代理在 `modelMappingEnabled=true` 且 `routeId` 精确命中时发送对应 `requestModel`；映射关闭或未命中时保留入站 `model`；每个入站请求最多产生一次 Messages POST。
- Claude Code / Claude Desktop 请求携带的 `?beta=true`、`anthropic-version` 与 `anthropic-beta` 必须按白名单透传到 Messages 上游，避免 Beta 客户端与非 Beta 响应流不匹配。
- Claude Messages 流必须增量校验 `content_block_start`、`content_block_delta`、`content_block_stop` 生命周期；网络拆包不能导致事件丢失，HTTP 200 不能直接等同于协议流成功。
- 对可安全推断类型的孤立文本或思考增量补齐空的块起始事件；无法可靠推断工具 ID/name 的孤立工具参数增量不得伪造工具块，并必须避免继续向客户端发送会触发 `Content block not found` 的孤立事件。
- 流诊断只记录事件类型、块索引、块类型、修复动作和计数，不记录提示词、响应正文、工具参数、API Key 或完整请求头。

## 非目标

- 不验证第三方 API Key 是否真实可用。
- 不修改 Claude 官方文件。
- 不重做供应商页面布局。
- 除修复本工具造成的已知文本/JSON 分裂状态外，不自动修改或替换用户模型映射；真实对话只应用用户已保存且精确命中的 `requestModel`。
- 不把本地默认模型、历史模型列表或映射值作为“获取模型”的远端发现结果。
- 不读取任意第三方价格目录作为模型发现结果。
- 不把模型目录请求、配置保存或页面加载当作推理请求；用户主动执行的连接测试只验证 Base URL 网络可达性，不验证模型、Key 或真实推理。
- 不持久化或输出 Claude Messages 请求正文、响应正文、工具输入和完整 SSE 数据。

## 功能要求

- `claude_desktop_models_response("")` 返回上述四个安全 Claude 模型 ID，顺序与 Claude UI 参考一致。
- `inferenceModels` 在未传入模型列表时仍写入默认四模型列表。
- 默认模型目录和供应商编辑器仍可展示 Fable、Haiku、Opus、Sonnet 四个角色，但展示目录不得成为运行时隐式映射。
- Claude Desktop 真实对话只允许依据 `modelMappingEnabled` 与 `modelMappingJson` 的精确 `routeId` 命中改写模型；映射关闭或未命中时保留入站 `model`，不得根据请求形状、模型家族、`modelList`、默认角色、历史发现结果或运行时目录猜测模型。
- 映射文本必须正确解析 `角色 (routeId): displayName -> requestModel [1M]`，分别保留角色、路由 ID、显示名称、实际请求模型和 1M 标记，不能把整段箭头表达式同时写入两个模型字段。
- `modelMappingJson` 为空或空数组且 `modelMapping` 含有效行时，设置归一化从文本重建 JSON。对于已知分裂状态，即文本中 Haiku 明确为 `claude-opus-4-7 -> claude-opus-4-7`、JSON 中同一路由却为 `claude-haiku-4-5`，以文本中的显式选择修复 JSON；其他自定义 JSON 映射保持原值。
- `configContents.meta.claudeDesktopModelRoutes` 是供应商保存时生成并实际写入 Claude 配置的路由副本。该对象结构有效时，设置归一化必须按相同 `routeId` 将其中的 `model`、`labelOverride` 和 `supports1m` 同步到 `modelMappingJson`，并补齐 JSON 中缺失的路由；不得删除仅存在于 JSON 的自定义条目。无效或缺失的配置路由不得覆盖现有映射。修复后配置路由、`modelMappingJson` 与 `modelMapping` 必须一致，重复归一化结果稳定。
- `configContents.meta.claudeDesktopMode` 为 `proxy` 或 `direct` 时，设置归一化必须同步 `claudeDesktopMode`、`routeEnabled` 与 `routeMode`，且重复归一化结果稳定，避免生成配置与 Profile 路由状态分裂。
- Claude Desktop 代理的上游 Base URL 兜底为 `https://api.toporeduce.cn`，但不能把本地代理 URL 当作上游。
- `fetch_relay_profile_model_ids` 解析 `apiKey`、`authContents`、JSON/TOML `configContents` 中的当前真实 Key。模型发现独立于 Messages 鉴权语义，按照 CC Switch 的 OpenAI-compatible model-fetch 行为对 `/v1/models` 固定使用 `Authorization: Bearer <当前 Key>`，不发送 `x-api-key` 或 `anthropic-version`。真实 `/v1/messages` 仍按 Key 字段语义选择鉴权头。
- `/v1/models` 返回非空目录时直接使用该结果，不请求或合并任何价格目录。
- `/v1/models` 返回 HTTP 2xx 但模型数组为空时明确失败，错误说明当前 Key 的标准模型目录没有返回模型。
- `/v1/models` 返回 401、403 或其他失败时保留标准接口失败，不得用其他目录掩盖鉴权或服务错误。
- 模型目录请求根据 Base URL 构造稳定的候选端点：优先当前路径的 `/v1/models` 或已存在版本段的 `/models`，兼容已知协议子路径剥离后的 `/v1/models` 与 `/models`；仅在候选返回 404/405 时按顺序尝试下一个候选。
- 标准模型目录候选回退不得改变当前 Key、鉴权字段或请求方法；401、403、429、5xx、网络错误和 200 空目录不能通过其他候选掩盖。
- 模型发现状态记录脱敏的候选端点、HTTP 状态、解析数量和最终命中端点；不得记录 API Key、Authorization 值、URL 查询参数或响应正文。
- 标准接口没有返回合格模型时，不回退到价格目录、`modelList`、`modelMapping`、`modelMappingJson` 或本地默认模型。真实对话不使用任何模型目录回退。
- Anthropic Messages 的“测试连接”与“获取模型”严格分离：测试连接仅向当前 Profile 的规范化 Base URL 发送一次 GET，并携带 `Accept: */*`、`Accept-Encoding: identity`；不得请求 `/v1/models`、`/v1/messages`，不得发送 Key、模型或请求正文。
- Anthropic Messages 测试连接收到任意 HTTP 响应即表示网络可达，并保留实际 HTTP 状态用于诊断；DNS、连接、TLS 或超时等传输错误才判定为连接失败。该结果不得表述为模型或推理可用。
- “获取模型”继续使用当前 Profile 的解析后 Key 独立请求 `/v1/models`，固定发送 Bearer，不继承 Anthropic Messages 的 `x-api-key` 与版本头；该约束只作用于模型目录请求。
- Claude Desktop 真实对话使用当前活动 Profile 的解析后 Key、上游 Base URL 和鉴权语义；精确应用当前 Profile 已启用的保存映射，不得发起模型目录请求，也不得回退到全局 Key、环境变量 Key 或其他历史 Profile。
- 保存 Claude Profile 时，以当前编辑器显式 Key 为最高优先级，并同步替换 `configContents`、`authContents` 中所有旧凭据别名；重新打开编辑器、获取模型、应用配置和消息代理必须解析到同一最新 Key。
- 返回信息和错误不得包含 API Key、Bearer token、URL query 或 fragment。
- Claude 实际推理的 503（例如 `No available channel for model`）作为当前 Key/用户组的通道可用性错误单独报告，不改变模型发现结果。
- 每次 Claude Desktop Messages 上游返回后写入一条脱敏路由诊断，只记录 Profile ID、原请求模型、实际上游模型和 HTTP 状态；不得记录 Key、URL、请求正文或响应正文。
- `activeClaudeRelayId` / `activeClaudeDesktopRelayId` 非空时只允许精确匹配对应目标 Profile；ID 无效时返回“未配置供应商”，不得误用旧 Profile。
- 活动 ID 为空时，仅当对应目标恰好存在一个 Profile 才允许兼容历史配置并选择该 Profile；存在零个或多个候选时返回“未配置供应商”。
- `responses_to_chat_completions` 必须保持输入 `model` 字段逐字不变，包括 `gpt-5.6-sol` 等未来模型 ID。
- 正常 Responses 代理链每个入站请求只执行一次上游推理 POST；模型发现只允许请求模型目录端点，不发送推理正文。
- Claude Desktop Messages 代理每个入站请求最多执行一次消息 POST，且不执行目录 GET。请求使用当前 Profile 的最新 Key 和鉴权语义；消息请求发送 `anthropic-version: 2023-06-01`，不得同时发送两种凭据头，也不得因鉴权兼容自动重试或双发推理请求。
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
- `crates/claude-codex-pro-core/src/relay_config.rs`
- `crates/claude-codex-pro-core/src/model_catalog.rs`
- `crates/claude-codex-pro-core/tests/model_catalog.rs`
- `crates/claude-codex-pro-core/tests/relay_config.rs`
- `crates/claude-codex-pro-core/src/settings.rs`
- `crates/claude-codex-pro-core/tests/protocol_proxy.rs`
- `crates/claude-codex-pro-core/src/launcher.rs`
- 对应 Rust 测试

## 2026-07-20 补充：Claude Desktop 本地请求覆盖

- Claude Desktop `/v1/messages` 代理必须读取当前 Profile 保存的 `headerOverride` 与 `bodyOverride`。
- `bodyOverride` 仅接受 JSON Object，发送前合并到出站 JSON；`stream` 为代理控制字段，保持入站请求原值。
- `headerOverride` 仅接受 JSON Object，发送时只写入合法且非受保护 Header；`Authorization`、`x-api-key`、`Host`、`Content-Length`、`Content-Type` 等传输或凭据 Header 不允许被覆盖。
- 路由诊断只记录已应用的 Header 名称和 Body 字段名，不记录 Header 值、Key、请求正文或响应正文。

## 2026-07-20 补充：Claude Desktop 上游失败脱敏诊断

- Claude Desktop `/v1/messages` 每次上游响应后，路由诊断除 profile、原始模型、实际上游模型和 HTTP 状态外，还记录脱敏的上游 path、鉴权头模式、协议、目标应用、是否流式、是否带 beta query、是否带 anthropic-version / anthropic-beta，以及请求体顶层字段名。
- Claude Desktop `/v1/messages` 非 2xx 响应必须额外写入脱敏错误摘要，只记录状态、content-type、错误 type/message 摘要；不得记录 API Key、Bearer token、完整 URL query、完整请求正文或完整响应正文。
