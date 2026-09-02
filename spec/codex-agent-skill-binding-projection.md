# Codex Agent Skill 绑定投影

## 背景

上游 Multica 创建 Agent 时将 `skill_ids` 与 Agent 字段放在同一个请求和事务中。本地 CCP 当前没有上游 workspace Agent API，只有经过信任与 manifest 校验的执行账本绑定。通用 Agent JSON 中可能遗留 `skills` 字段，但它不能证明 Skill 可执行。

## 目标

- “我的任务”中的本地 Agent 列表只展示执行账本中真实存在、启用的 Agent Skill 绑定。
- 每个投影 Skill 保留 Skill ID、manifest digest、绑定 ID、来源和只读标记。
- 忽略或覆盖本地 Agent JSON 中未经过绑定校验的 `skills` 字段，避免把任意 JSON 展示为可执行能力。
- 保持 Codex 原生 Skill 与本地执行账本命名空间和来源可区分。

## 非目标

- 不把 Codex 原生只读 Skill 清单伪装成上游 workspace UUID。
- 不在没有原子 Agent+Skill 保存命令前增加可提交的假多选创建流程。
- 不修改 Codex 状态库、远端 Multica 数据库或运行时协议。

## 数据与接口

Agent 列表投影读取 `MulticaExecutionStore` 的 `scope_kind=agent` 绑定，仅纳入 `enabled=true` 的条目；每个条目输出 `skill_ref`、`binding_id`、`source=codex_execution_store`、`trusted` 和 `read_only=true`。列表同时输出 `skills_source=codex_execution_store` 与 `skills_read_only=true`。

## 验收

1. 含有未验证 `skills` JSON 的 Agent 不会在列表中显示该字段内容。
2. 已启用 Agent 绑定会显示真实 Skill ID、manifest digest 和执行账本来源。
3. 禁用绑定不会显示。
4. Rust 定向测试和前端构建通过。
