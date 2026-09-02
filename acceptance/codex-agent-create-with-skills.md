# Codex Agent 创建时 Skill 绑定验收

对应规格：`spec/codex-agent-create-with-skills.md`。

1. `/multica/agents/create` 拒绝重复 Skill、未知/未启用/未信任 Skill，以及 digest 不一致的 Skill；发生拒绝时不得创建 Agent 或 binding。
2. 成功请求会创建一个 Agent，并为每个选中 Skill 创建启用的 `Agent` scope binding；Agent 集合投影只显示这些真实 binding。
3. 创建后将任务分配给该 Agent 时，`thread/start` 请求包含与已保存 binding 一致的 Skill 引用和 digest。
4. 创建表单提供搜索、多选、空搜索结果和不可用状态；提交调用专用命令，普通 Agent 更新仍不改变 bindings。
5. `cargo fmt --check`、针对性 Rust 测试、`node --check assets/inject/renderer-inject.js`、前端检查和 Release 构建通过。
