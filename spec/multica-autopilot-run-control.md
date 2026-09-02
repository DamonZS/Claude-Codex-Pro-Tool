# Multica Autopilot Run Control

## 背景
上游 Multica 将每次 Autopilot 触发持久化为独立的 `autopilot_run`，而不是把运行对象嵌在前端任务 JSON 中。

## 目标
在 CCP 本地控制面提供受限的运行记录创建、列表和详情查询，并明确 `pending` 仅代表已记录、未代表 Codex 已执行。

## 要求
- 运行记录包含上游核心字段：id、autopilot_id、trigger_id、source、status、issue_id、task_id、triggered_at、completed_at、failure_reason、reason_code、created_at。
- source 仅允许 manual、schedule、webhook、api；新记录状态为 pending。
- 记录持久化到独立 execution state，采用上限保护和文件锁。
- bridge 提供 runs、run、trigger 路由；触发必须按上游语义创建 Issue 或执行绑定，并尝试进入 Codex 原生 thread/subagent 调度链；触发失败不得伪装成功。
- `create_issue` 自动化创建带 `origin_type=autopilot`、`origin_id` 的 Issue，并通过 Agent assignment 幂等键生成执行绑定。
- `run_only` 自动化直接生成执行绑定；Codex host 不可用时保持 `queued`/`pending`，返回稳定诊断码。
- `run_only` 不得向工作区 `issues` 集合写入占位 Issue；执行上下文从工作流运行记录和工作流实体恢复后直接进入原生 Codex 调度。
- 下游非连接类错误必须把运行记录推进到 `failed`，写入 `failure_reason` 与稳定 `reason_code`；仅 Codex host 暂不可用可保持排队状态。
- 任务队列的终态事件必须同步运行记录：`completed` -> `completed`，`failed`/`cancelled` -> `failed`；重复事件遵守 revision/CAS，不得覆盖更新后的状态。
- 重复 trigger 使用稳定幂等键，不得重复创建 Issue 或执行绑定。
- 不保存 webhook payload、凭据或完整任务正文。

## 非目标
本次不实现远端调度器、Webhook worker、配额扣减或 Codex runtime 执行；这些必须以后端真实能力接入后推进。
