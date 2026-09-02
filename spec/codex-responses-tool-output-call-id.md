# Codex Responses 工具输出调用 ID 兼容

## 背景

Codex 页面经过 CCP 的 Responses 中转访问 HTTP 上游时，曾返回 `function_call_output requires call_id on HTTP requests`。Responses 输入转换已接受工具调用项的 `id` 作为调用 ID，但工具输出项只读取 `call_id`，导致同一请求内可证明关联的输出未被规范化。

## 目标

- 在发往上游前，仅为同一个 `input` 序列中能与已出现工具调用精确匹配的工具输出补齐 `call_id`。
- 同时覆盖 `function_call_output` 与 `custom_tool_call_output`，且 Chat Completions 转换复用相同规则。

## 约束

- 只接受已出现 `function_call` 或 `custom_tool_call` 的 `call_id` 或 `id`。
- 输出候选仅可来自 `call_id`、`tool_call_id` 或 `id`，并且必须等于已见调用 ID；不能猜测、生成、截断或重写其他 ID。
- 不能证明关联的输出保持原样，由上游按原协议拒绝。
- 不修改 `previous_response_id`、工具内容、身份验证或传输协议。

## 非目标

- 不实现 Responses WebSocket v2，也不把 HTTP 请求伪装成 WebSocket。
- 不修改供应商配置、不重放失败请求、不创建第二个 Codex runtime。
