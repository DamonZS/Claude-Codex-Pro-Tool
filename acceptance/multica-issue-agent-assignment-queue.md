# 验收：Issue 分配 Agent 自动入队

对应规格：`spec/multica-issue-agent-assignment-queue.md`

## 必需证据

- Rust 单元/集成测试覆盖 Agent 分配、重复幂等和非 Agent 分配。
- `cargo fmt --check`。
- `cargo test -p claude-codex-pro-core` 的相关测试通过。
- `git diff --check`。

## 非目标

- 不验证远程 PostgreSQL、daemon heartbeat 或真实 Codex Host 执行；这些仍依赖上游服务和已连接页面。
