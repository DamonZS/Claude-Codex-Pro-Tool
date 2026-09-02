# 统计来源与能力边界验收

验证对象：`spec/multica-statistics-source-fidelity.md`

## 通过标准

1. Rust 统计查询返回首个 item 的 `source` 为 `local_control_plane`。
2. `limitations` 包含四个规定的不可用能力标识。
3. 既有本地计数仍返回，且代码中没有新增伪造 token、成本或上游 usage endpoint 的逻辑。

## 验证方式

- `cargo test -p claude-codex-pro-core multica_workspace -- --nocapture`
- 检查统计 JSON 与上游 API 字段来源说明一致。
