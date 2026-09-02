# Codex Agent Skill 绑定投影验收

对应规格：`spec/codex-agent-skill-binding-projection.md`

## 必须通过

- `agent_collection_with_bindings` 测试证明未验证 JSON 不会泄漏到 Agent Skill 列表。
- 测试证明启用绑定包含真实 ID、manifest digest、绑定来源和只读标记。
- 测试证明禁用绑定不出现在列表中。
- `cargo test -p claude-codex-pro-core multica_workspace --lib -- --nocapture` 通过。
- `npm --prefix apps/claude-codex-pro-manager run check` 与 Vite 构建通过。

## 非目标检查

- 本次不声称实现上游远端 `POST /api/agents` 的原子 Agent+Skill 事务。
- 本次不允许写入 Codex 原生 Skill 清单或伪造 workspace Skill UUID。
