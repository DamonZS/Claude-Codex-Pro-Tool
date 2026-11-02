# 验收：只认 chat 的 agent 如何用非 chat 的上游

验证对象：`spec/agent-chat-upstream-bridging.md`。
下面每一条都写明「怎么算过」与「用什么证据证明」。没有证据的一项不算通过。

## A1 · 规则只有一份，且与界面一致

- **通过标准**：前端 `agentProviderContract.ts` 里每个 agent 的 `formats` 与后端 `agent_providers::allowed_formats(app)` 完全一致；WorkBuddy 与 Cursor 都是三项（`openai-chat`、`anthropic`、`openai-responses`）。
- **验证方式**：`cargo test -p claude-codex-pro-manager --test windows_subsystem` 里的 `the_frontend_format_table_matches_the_backend`。
- **证据（必须能复现这条结论）**：把前端那两行临时退回单项，考试必须红（实测报 `workbuddy 的前端格式表与后端不一致`，`test result: FAILED`）；恢复后通过。
- **不在范围**：其它 agent 的格式集本身是否正确（各自由后端表决定，本次未改）。

## A2 · 界面能选到、并能看到解释

- **通过标准**：打开 Agent 供应商编辑器，某 agent 有多个格式时，下拉框列出全部；选 `anthropic` 或 `openai-responses` 时出现「由代理替它转成上游要的那套协议」的提示；选回 `openai-chat` 提示消失。
- **验证方式**：`npm --prefix apps/claude-codex-pro-manager run test` 里的 `AgentProviderPanel.test.tsx`（两组断言）。
- **证据**：vitest 输出 3 files / 10 tests passed（含该文件 2 条）。
- **不在范围**：视觉样式与截图。

## A3 · 请求会被翻成上游认得的那套协议

- **通过标准**：从 `/agent/<id>/v1/chat/completions` 进来的 chat 请求，按该供应商的接口格式分别变成 Anthropic Messages 请求或 Responses 请求（含 `max_tokens` 默认值、tools 与 tool_choice 的对应写法），并把 `response_conversion` 设成对应的种类。
- **验证方式**：`cargo test -p claude-codex-pro-core --lib protocol_proxy` 里的 `chat_completions_to_anthropic_request` / `chat_completions_to_responses_request` 相关用例，以及两条真起 mock 上游的端到端：`a_chat_only_agent_reaches_an_anthropic_messages_supplier`（断言打到 `POST /v1/messages`、带 `x-api-key` 与 `anthropic-version`）、`a_chat_only_agent_reaches_a_responses_supplier`（断言打到 `POST /v1/responses`、带 `Authorization`、body 有 `input` 无 `messages`）。
- **证据**：core lib 630 passed / 0 failed（含上述用例）。

## A4 · 回应会被翻回 chat，含流式

- **通过标准**：非流式回应的正文与用法统计按 chat 形状给出；流式把上游的 SSE 翻成 chat 分片并以 `data: [DONE]` 收尾；乱序/半包不崩。
- **验证方式**：`protocol_proxy` 里的 `an_anthropic_stream_becomes_chat_chunks`、`a_responses_stream_becomes_chat_chunks`、`an_anthropic_answer_becomes_a_chat_answer`、`a_responses_answer_becomes_a_chat_answer`（含逐块喂入与一次喂入两条路径）。

## A5 · 真 HTTP 入口走得通

- **通过标准**：不直接调 handler，而是通过真实 socket 的 HTTP 请求到达 agent 入口，得到 HTTP 回应（这里是 502 + `{"status":"failed",…}`，因为供应商不存在）。
- **验证方式**：`launcher.rs` 的 `the_agent_chat_path_is_reached_through_a_real_http_request`。

## A6 · 写入的地址按规则决定

- **通过标准**：`openai-chat` 写上游地址；其它格式写本机代理（`http://127.0.0.1:<端口>/agent/<供应商 id>`）；代理起不来时不写死地址，而是把错误报给用户。
- **验证方式**：`agent_providers::agent_chat_needs_proxy` / `agent_chat_base_url` 单测、`agent_providers/workbuddy.rs` 的 `a_non_chat_upstream_is_written_as_our_proxy`、`settings_commands::agent_address_for` 的单测 `the_address_a_pasted_manual_setup_gets_follows_the_writers_rule`。
- **证据**：core lib 与管理器 lib 的输出；管理器 lib 147 passed。

## A7 · 明确不做的事有明确说法

- **通过标准**：Gemini 原生格式不假装能转，而是给出中文错误；Codex 侧不与 Anthropic 转换（沿用用户决定）；不按模型名猜协议。
- **验证方式**：`a_gemini_native_supplier_says_so_instead_of_guessing`；`protocol_proxy` 里 `the_upstream_interface_follows_the_users_choice`（协议由用户选择决定）；Codex 那条由既有 `codex_base_url_for_protocol` 测试覆盖。

## A8 · 其它 agent 不受影响

- **通过标准**：自己就会说目标协议的 agent 仍然直连上游、地址不变。
- **验证方式**：既有的继电器/纯 API 配置测试（`apply_pure_api_config_to_home_with_protocol` 相关）与 `allowed_formats` 各 agent 的用法保持不变；本次改动只加了 `agent_chat_needs_proxy` 这一处开关。

## 已完成 / 未完成

- **已满足**：A1–A8（都有可复现命令与实测输出）。
- **未满足（如实标注）**：本机没有装 WorkBuddy，也没有 Cursor 账号，**没有做进程级联调**（真 agent 进程连到 57341 上问一句、拿到回答）。协议层、写入层、入口路由层都有真测试与真 socket/真 HTTP 证据；端到端那一步需要一台装了 WorkBuddy 或配好 Cursor 的机器，或后续补一个「假 agent 进程」的联调脚本。
- **非目标**：Gemini 原生转换、Codex 与 Anthropic 之间的转换、模型名自动判断协议、UI 视觉。