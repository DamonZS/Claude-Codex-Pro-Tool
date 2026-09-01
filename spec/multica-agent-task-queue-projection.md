# Multica Agent Task Queue Projection

## 背景
上游 Multica 以 `agent_task_queue` 表承载任务生命周期，字段包括状态、尝试次数、父任务、失败原因和心跳。CCP 当前只有本地 Codex execution binding，需要提供同名只读投影供“我的任务”核对。

## 目标
- 在 workspace bootstrap 中暴露 `agent_task_queue` collection。
- 状态映射遵循上游 `queued/dispatched/running/completed/failed/cancelled`；本地等待目录状态保留为扩展值。
- 映射仅使用 CCP 已持久化 execution binding，不创建虚假任务、runtime 或 worker。
- 暴露 `attempt`、`max_attempts`、`parent_task_id`、`failure_reason`、`last_heartbeat_at_ms` 等审计字段。
- 提供带 revision/lease 校验的队列状态转换路由，覆盖派发、等待本地目录、运行、完成、失败和取消。

## 非目标
本次不实现远程队列调度器、自动重试 worker、数据库迁移或 Codex SQLite 写入；状态转换只作用于 CCP 本地 execution ledger。

## 技术约束
投影只读、受 workspace 隔离和现有分页上限约束；`max_attempts` 在缺少本地策略时固定为 1，并通过 `source` 标明投影来源。
