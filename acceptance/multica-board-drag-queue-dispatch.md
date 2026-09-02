# 验收：看板拖拽与待办队列自动派发

对应规格：`spec/multica-board-drag-queue-dispatch.md`。行为依据上游 `agent_task_queue` 与 `ClaimAgentTask` 队列协议。

## 必需证据

- `node --check assets/inject/renderer-inject.js`
- `cargo fmt --check`
- 相关 Rust 队列派发测试通过。
- `git diff --check`
- 手动验证拖拽审核中到待办后，关联 Agent queued binding 在后台 tick 进入 `dispatched` 或在 Host 不可用时保持 `queued`。

## 语义断言

- 未分配 Agent 的 todo Issue 不会触发 dispatch。
- 普通 Issue 没有关联 binding 时不会被前端扫描器派发。
- 每个 binding 同一时刻最多一个 dispatch 请求；revision/lease 冲突后下次 tick 可重试。
