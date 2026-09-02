# Codex Agent Skill 编辑器

## 背景

上游 Multica 对已有 Agent 的 Skill 绑定使用受控多选控件，并通过 replace-all 接口保存。CCP 已有真实的本地 Agent Skill replace-all 路由，但通用 Agent 编辑器未提供对应入口。

## 目标

为已有本地 Agent 提供基于当前 Codex 页面 Skill 清单的搜索、多选和全量保存界面；保存必须调用 `/multica/skills/bindings/replace`，不能写入 Agent JSON 的任意 `skills` 字段。

## 范围与约束

- 仅支持已有 Agent。新建 Agent 的实体 JSON 与执行绑定当前使用不同持久化存储，不得将非原子两步操作展示为原子创建绑定。
- 仅当当前 Codex 页面声明可执行 Skill 协议时可编辑；只读清单、未信任或不可派发 Skill 必须禁用。
- 保存使用当前 binding revision，并由后端继续执行信任、安装与 manifest digest 校验。
- 不启动 CLI、app-server、daemon 或第二执行器。

## 验收

见 `acceptance/codex-agent-skill-editor.md`。
