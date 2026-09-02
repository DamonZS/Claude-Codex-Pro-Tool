# Codex Responses 工具输出调用 ID 兼容验收

对应规格：[spec/codex-responses-tool-output-call-id.md](../spec/codex-responses-tool-output-call-id.md)。

## 通过条件

- 出站原生 Responses 请求中，紧随已知调用且只携带相同 `id` 的工具输出获得 `call_id`。
- Chat Completions 转换把该输出转换为相同 `tool_call_id` 的 tool message。
- 未匹配、空 ID 或不同 ID 的输出不被补齐。
- 其他请求字段，包括 `previous_response_id`，保持不变。

## 必需证据

```powershell
cargo test -p claude-codex-pro-core protocol_proxy --lib -- --nocapture
cargo fmt --check
git diff --check
```
