# Codex Agent 创建时 Skill 绑定

## 背景

上游 Multica 的创建智能体请求在同一个 `POST /api/agents` 中提交 `skill_ids`，服务端在同一数据库事务内写入 Agent 与 Agent-Skill 关联。本地控制面此前只能先创建 Agent，再单独编辑绑定；这会与上游创建语义不一致，并可能留下不完整配置。

## 目标

为本地“我的任务”提供受控的 Agent 创建命令。该命令在一次请求内接收 Agent 与 Skill 引用，并只在当前 Codex 页面 Host 确认这些 Skill 可执行后持久化真实绑定。

## 要求

- 新命令为 `/multica/agents/create`，不得复用通用 `/multica/workspace/upsert` 绕过绑定校验。
- 输入包含 Agent 实体与 `skills`；Skill ID 必须唯一。
- 在任何本地写入前，必须通过当前 Host 的 capabilities 与 `list_skills` 校验每项存在、启用及 manifest digest；同时通过本地信任快照校验 `dispatch_allowed` 与 digest。
- 成功结果必须同时返回已保存的 Agent 和真实的 execution-store Agent binding；任何失败不得对外报告创建成功。
- 创建后派发仍只读取 execution-store 中的已验证绑定，并通过当前 Codex Host 写入原生 `thread/start`。
- 创建表单按上游 `SkillMultiSelect` 语义提供搜索、多选、已选状态、空态；Host 不可用或没有可派发 Skill 时显示准确不可用状态。
- 不启动 CLI、`codex.exe app-server`、daemon 或第二执行器。

## 范围外

- 不把 Codex 原生 Skill ID 伪装为上游 workspace Skill UUID。
- 不从本地状态猜测 Host 可执行能力。
- 不实现远程 Multica server 的数据库 API。
