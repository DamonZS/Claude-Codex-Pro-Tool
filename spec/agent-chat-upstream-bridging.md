# 规格：只认 chat 的 agent 如何用非 chat 的上游

## 背景

有的 agent 只会说一种协议。WorkBuddy 只认 OpenAI Chat Completions，Cursor 也是（且 CCP 不能替它写文件，只能给它一段值去粘贴）。可它们的供应商列表里会出现 Anthropic 的模型（glm、deepseek、claude 转接等），或者只能通过 Responses API 提供的模型。用户把上游换成这类供应商时，agent 自己说不通，以前只能直连上游、必然失败。

CCP 已经为本机跑着一条协议代理（Codex 那条 57321、Claude 桌面版/命令行那两条 57331）。这一份规格说的是：让「只会说 chat」的 agent 也能走这条代理，由它替 agent 把请求翻译成上游要的那套协议，再把回应翻回来。

## 目标

- 只认 chat 的 agent，在用户选了 Anthropic Messages 或 OpenAI Responses 的上游格式时，能正常对话（含流式）。
- 地址由谁决定、写给谁、写给哪里，规则只有一份，写入与手动粘贴共用。
- 代理起不来时不写死地址，而是当场报错。
- 不改变其它 agent 的行为：自己就会说那种协议的（Gemini、Grok、OpenCode、OpenClaw、Hermes、Pi、MiniMax Code 等）仍然直连上游。

**不包含**：Gemini 原生 generateContent 与 chat 之间的转换（目前没有实现，遇到就明确报错）；Codex 客户端与 Anthropic 上游之间的转换（已按用户决定移除：Codex 只讲 Responses）；模型名自动判断协议（协议一律由用户在供应商上选的「接口格式」决定）。

## 用户视角

1. 在「Agent 供应商」页选 WorkBuddy（或 Cursor），新增一个供应商，接口格式选 Anthropic Messages 或 OpenAI Responses。
2. 表单会提示一句：这个 Agent 只会说 Chat Completions，选了别的格式，CCP 会把地址写成它自己在本机的代理 —— 由代理替它转。
3. 保存并应用。WorkBuddy 的配置文件里写的是本机代理的地址；Cursor 因为不能替它写文件，界面上会给出一段可粘贴的地址与密钥，同样是本机代理的地址。
4. 之后在 WorkBuddy / Cursor 里正常对话即可，上游那家供应商收到的是它认得的那套协议。

## 功能要求

### 规则（唯一一份）

- `agent_providers::agent_chat_needs_proxy(api_format)`：`openai-chat` 之外的格式都需要代理。
- `agent_providers::agent_chat_base_url(provider)`：需要代理时给 `protocol_proxy::agent_chat_proxy_base_url(&provider.id)`，否则给供应商自己的地址。
- 管理器侧 `settings_commands::agent_address_for(provider)` 与上面同一条规则，供手动模式（Cursor）的粘贴值使用。

### 代理入口

- 路径：`/agent/<供应商 id>/chat/completions`、`/agent/<供应商 id>/v1/chat/completions`、`/agent/<供应商 id>/v1/v1/chat/completions`（`protocol_proxy::is_agent_chat_proxy_path`）。
- 端口：默认 `protocol_proxy::DEFAULT_AGENT_PROXY_PORT = 57341`；管理器先把这条入口拉起来，把实际端口记进状态目录（`remember_agent_proxy_port`），写文件与粘贴值读同一个值；端口被占时回退到任意空闲回环端口。
- 上游接口由 `provider.api_format` 决定：`OpenaiChat` 直通、`Anthropic` 与 `OpenaiResponses` 转换、`GeminiNative` 明确报错（「…请把这家供应商的接口改成 Anthropic Messages 或 OpenAI 兼容，要么给这个 agent 换一家供应商。」）。
- 失败的回应：502，正文 `{"status":"failed","message":"…"}`；日志事件 `helper.agent_chat_proxy_failed`。

### 转换

- 请求：`chat_completions_to_anthropic_request`、`chat_completions_to_responses_request`（缺 `max_tokens` 时补 4096，Anthropic 必填）。
- 非流式响应：`anthropic_to_chat_completions_response`、`responses_to_chat_completions_response`。
- 流式：`AnthropicSseToChatConverter`、`ResponsesSseToChatConverter`（收尾事件只发一次，最后补 `data: [DONE]`）。
- 转换种类由 `UpstreamProxyResponse.response_conversion` 显式带出（`AnthropicToChat` / `ResponsesToChat`）。

### 写入

- 有 writer 的（WorkBuddy）：`agent_providers/workbuddy.rs` 用 `agent_chat_base_url` 拼地址。
- 手动模式（Cursor）：`apply_agent_provider` 返回的 `reveal.baseUrl` 用同一条规则；应用前先确保代理起来。

### 界面

- Agent 供应商编辑器的「接口格式」下拉框列出该 agent 后端允许的全部格式；WorkBuddy 与 Cursor 都是三项。
- 选了非 chat 格式时显示中文提示（由代理替它转成上游要的那套协议）。

## 技术约束

- 不新增依赖；沿用 `protocol_proxy.rs` 里既有的 SSE 解析与 HTTP 转发设施。
- 代理只绑回环（127.0.0.1）；沿用既有的浏览器来源防护（无 `Origin`/`Referer` 或带 helper token 才放行转发类路径）。
- 规则与表不能各写一份：前端格式表必须与后端 `allowed_formats` 一致，由契约测试盯着。

## 交付范围

- `crates/claude-codex-pro-core/src/protocol_proxy.rs`（入口、转换、测试）
- `crates/claude-codex-pro-core/src/launcher.rs`（路径分发、按转换种类分流、测试）
- `crates/claude-codex-pro-core/src/agent_providers.rs` 与 `agent_providers/workbuddy.rs`（规则、写入）
- `apps/claude-codex-pro-manager/src-tauri/src/{commands.rs,settings_commands.rs}`（拉起代理、粘贴值）
- `apps/claude-codex-pro-manager/src/components/supplier/{AgentProviderPanel.tsx,agentProviderContract.ts}`（界面）
- 测试：core 单测与 mock 上游端到端、真 socket 与真 HTTP 路由、writer 测试、前端组件测试、前后端表一致的契约测试