# Multica Autopilot Run Control

## 背景
上游 Multica 将每次 Autopilot 触发持久化为独立的 `autopilot_run`，而不是把运行对象嵌在前端任务 JSON 中。

## 目标
在 CCP 本地控制面提供受限的运行记录创建、列表和详情查询，并明确 `pending` 仅代表已记录、未代表 Codex 已执行。

## 要求
- 运行记录包含上游核心字段：id、autopilot_id、trigger_id、source、status、issue_id、task_id、triggered_at、completed_at、failure_reason、reason_code、created_at。
- source 仅允许 manual、schedule、webhook、api；新记录状态为 pending。
- 记录持久化到独立 execution state，采用上限保护和文件锁。
- bridge 提供 runs、run、trigger 路由；触发失败不得伪装成功。
- 不保存 webhook payload、凭据或完整任务正文。

## 非目标
本次不实现远端调度器、Webhook worker、配额扣减或 Codex runtime 执行；这些必须以后端真实能力接入后推进。
