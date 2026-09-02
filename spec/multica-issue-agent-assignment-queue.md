# Issue 分配 Agent 自动入队

## 背景

上游 Multica 在 Issue 创建或改派到 Agent 后，通过 `agent_task_queue` 投影等待 daemon 领取。本地控制面此前仅在已有 Codex 执行绑定时投影队列，导致分配后的任务不可见。

## 目标

- Issue 的 `assignee_type=agent` 且存在 `assignee_id` 时，写入一个本地执行绑定并以 `queued` 投影。
- 使用 workspace、issue、agent 组成稳定幂等键；重复写入不得创建重复队列项。
- 未分配、成员分配或小队分配不自动创建执行绑定。
- 不创建虚假 Codex thread、runtime 或 daemon；实际执行仍需已连接的 Codex page host。

## 上游依据

- `server/internal/handler/issue.go` 的 Issue 写入后队列协调逻辑。
- `server/pkg/db/queries/issue.sql` 的 Agent assignee 与队列触发规则。
- `server/migrations/004_agent_runtime_loop.up.sql` 的 `agent_task_queue` 状态约束。

## 验收

1. Agent 分配响应包含 queued binding。
2. 相同 Issue/Agent 重复 upsert 返回 replay 且队列总数不增加。
3. 非 Agent 分配响应不创建队列绑定。
4. 队列项状态为 `queued`，后续由显式 Codex 执行路由推进。
