# 验收：Issue 分配 Agent 自动入队

对应规格：`spec/multica-issue-agent-assignment-queue.md`

## 必需证据

- Rust 单元/集成测试覆盖 Agent 分配、重复幂等、跨 Agent 改派并行、Issue 删除取消和非 Agent 分配。
- `cargo fmt --check`。
- `cargo test -p claude-codex-pro-core` 的相关测试通过。
- `git diff --check`。

## 语义断言

- 改派到另一 Agent 不得隐式取消旧 Agent 的活跃执行；这是上游 `server/internal/handler/issue.go` 的明确约定。
- 只有删除 Issue 才取消该 Issue 的全部非终态执行绑定。

## 非目标

- 不验证远程 PostgreSQL、daemon heartbeat 或真实 Codex Host 执行；这些仍依赖上游服务和已连接页面。
