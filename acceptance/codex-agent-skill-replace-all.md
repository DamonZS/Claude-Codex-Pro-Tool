# 本地 Agent Skill 全量绑定验收

对应规格：`spec/codex-agent-skill-replace-all.md`

## 必须通过

- Rust 测试覆盖新增、移除、清空、重复项、信任/manifest 失败、revision 冲突和原子回滚。
- 路由解析只接受 `scope_kind`、`scope_id`、`skills`、`expected_revision`，拒绝未知字段和非法集合。
- `cargo fmt --check` 与 `cargo test -p claude-codex-pro-core multica_workspace multica_execution_store --lib` 通过。

## 非目标

- 本验收不把 Codex 原生只读 Skill 清单当作可编辑 workspace Skill。
- 本验收不要求在后端真实绑定完成前开放 Agent 编辑器多选提交控件。
