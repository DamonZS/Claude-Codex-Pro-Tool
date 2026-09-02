# 本地 Agent Skill 全量绑定

## 背景

上游 Multica 对 Agent Skill 采用 `PUT /api/agents/{id}/skills` 的 replace-all 语义：提交的 Skill 集合是最终集合，空数组表示清空，并在一次事务内完成校验和 junction 写入。CCP 当前只有单项绑定接口，无法真实表达该工作流。

## 目标

- 为本地 Agent 提供受控的全量 Skill 绑定接口。
- 只接受当前 Codex 页面列出的已安装 Skill，并复用本地 trust snapshot 与 manifest digest 校验。
- 在 `MulticaExecutionStore` 中一次性替换同一 workspace/Agent scope 的绑定；任一 Skill 无效时原集合保持不变。
- 返回最终绑定集合与集合 revision，供后续 Agent 编辑器使用。

## 非目标

- 不把 Codex 原生只读 Skill 清单伪装成上游 workspace Skill UUID。
- 不写入 Agent JSON 的任意 `skills` 字段，不启动第二个 Codex runtime，不安装 Skill。

## 接口

`PUT /multica/skills/bindings/replace` 接收：

```json
{
  "scope_kind": "agent",
  "scope_id": "agent-id",
  "skills": [{"id": "skill-id", "manifestDigest": "..."}],
  "expected_revision": 3
}
```

`skills` 是最终集合，允许为空。`expected_revision` 可选；提供时必须等于该 scope 当前集合 revision（空集合为 0），否则返回 `skill_binding_revision_conflict`。成功响应包含 `bindings` 和新的 `revision`。

## 验收

1. Agent scope 的全量替换可新增、保留和移除绑定，空集合可清空。
2. 未信任、未安装、manifest 不匹配或重复 Skill 使整个操作失败，旧集合不变。
3. revision 冲突被拒绝，文件中不存在部分提交。
4. 非 Agent scope 仍可复用同一受控原子语义，但不能绕过 scope/ID 校验。
