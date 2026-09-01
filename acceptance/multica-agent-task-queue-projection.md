# 验收：Multica Agent Task Queue Projection

对应规格：`spec/multica-agent-task-queue-projection.md`

## 通过标准
- `MulticaWorkspaceResourceKey::AgentTaskQueue` 序列化为 `agent_task_queue` 并出现在 bootstrap modules/collections。
- 每个投影项含 `id/status/attempt/max_attempts/parent_task_id/failure_reason/last_heartbeat_at_ms/source`。
- 无 execution binding 时返回空 collection，不伪造数据。
- Rust 格式检查与 core 测试通过。

## 验证方式
```powershell
cargo fmt --check
cargo test -p claude-codex-pro-core multica_workspace -- --nocapture
cargo check -p claude-codex-pro-core
```

## 非范围
真实远程 Multica server 的 worker、Autopilot、Webhook 和 Runtime 注册不在本验收内。
